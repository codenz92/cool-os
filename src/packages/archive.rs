fn archive_verify_lines(path: &str) -> Vec<String> {
    match verify_archive(path) {
        Ok((archive, verification)) => {
            let dependency_line = match first_missing_dependency(&archive.dependencies) {
                Some(dep) => format!("dependencies=missing {}", dep),
                None => format!("dependencies=ok count={}", archive.dependencies.len()),
            };
            alloc::vec![
                format!(
                    "package_trust=ok key={} algorithm={} id={} version={} command={}",
                    verification.key,
                    verification.algorithm,
                    verification.id,
                    verification.version,
                    verification.command
                ),
                format!("manifest_sha256={}", verification.manifest_sha256),
                format!("signature={}", verification.signature_path),
                dependency_line,
                format!("payloads=ok count={}", verification.payloads.len()),
                payload_summary_lines(&verification.payloads),
            ]
        }
        Err(err) => alloc::vec![format!("package_trust=failed error={}", err)],
    }
}

fn archive_info_lines(path: &str) -> Vec<String> {
    let Some(bytes) = crate::vfs::vfs_kernel_read_file(path) else {
        return alloc::vec![format!("path={} missing", path)];
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return alloc::vec![format!("path={} not_utf8", path)];
    };
    match parse_archive_manifest(text) {
        Ok(archive) => alloc::vec![
            format!("path={}", path),
            format!(
                "id={} name={} command={} version={}",
                archive.id, archive.name, archive.command, archive.version
            ),
            format!(
                "icon={} category={} exec={} permission={}",
                archive.icon, archive.category, archive.exec_path, archive.permission
            ),
            format!(
                "aliases={} associations={}",
                archive.aliases, archive.associations
            ),
            format!(
                "depends={} min_os_version={}",
                join_csv(&archive.dependencies),
                archive.min_os_version
            ),
            format!("payloads={}", archive.payloads.len()),
            payload_summary_lines(&archive.payloads),
        ],
        Err(err) => alloc::vec![format!("path={} invalid={}", path, err)],
    }
}

fn verify_archive(path: &str) -> Result<(ArchiveManifest, PackageVerification), &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package file not found")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package is not UTF-8 manifest")?;
    let archive = parse_archive_manifest(text)?;
    if archive.min_os_version > CURRENT_OS_VERSION {
        return Err("package requires newer OS");
    }
    let sig_path = signature_path(path);
    let signature = package_signature(&sig_path)?;
    if signature.algorithm != TRUST_ALGORITHM {
        return Err("unsupported package signature");
    }
    if !signature.package_id.eq_ignore_ascii_case(&archive.id) {
        return Err("signature package mismatch");
    }
    if signature.package_version != archive.version {
        return Err("signature version mismatch");
    }
    let key = find_trusted_key(&signature.key).ok_or("package key not trusted")?;
    if key.algorithm != signature.algorithm {
        return Err("unsupported package signature");
    }
    if key.status == "revoked" {
        return Err("package key revoked");
    }
    if key.status != "trusted" {
        return Err("package key not trusted");
    }
    if signature.issued_epoch < key.not_before {
        return Err("package key not yet valid");
    }
    if signature.issued_epoch > key.not_after {
        return Err("package key expired");
    }
    let digest = crate::update_crypto::sha256(archive.trust_manifest.as_bytes());
    if !crate::update_crypto::hex_matches_digest(&signature.manifest_sha256, &digest) {
        return Err("package manifest hash mismatch");
    }
    let signature_bytes =
        crate::update_crypto::hex_to_64(&signature.signature).ok_or("package signature invalid")?;
    if !crate::update_crypto::ed25519_verify(
        &key.public_key,
        archive.trust_manifest.as_bytes(),
        &signature_bytes,
    ) {
        return Err("package signature invalid");
    }
    let verification = PackageVerification {
        id: archive.id.clone(),
        command: archive.command.clone(),
        version: archive.version.clone(),
        key: signature.key,
        algorithm: signature.algorithm,
        manifest_sha256: crate::update_crypto::hex(&digest),
        signature_path: sig_path,
        dependencies: archive.dependencies.clone(),
        payloads: archive.payloads.clone(),
    };
    Ok((archive, verification))
}

fn verify_installed_package(
    id_or_command: &str,
) -> Result<(crate::app_metadata::AppManifest, OwnerRecord), &'static str> {
    if let Some(app) = find_app(id_or_command) {
        if is_installed(app.id) {
            return Err("built-in package trust is implicit");
        }
        return Err("package not installed");
    }
    let owner = owner_record_by_id_or_command(id_or_command).ok_or("package owner missing")?;
    let bytes = crate::vfs::vfs_kernel_read_file(&owner.manifest_path)
        .ok_or("installed manifest missing")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "installed manifest is not UTF-8")?;
    crate::app_metadata::validate_manifest_text(text).map_err(|_| "installed manifest invalid")?;
    let manifest = manifest_from_text(text)?;
    if !manifest.id.eq_ignore_ascii_case(&owner.id)
        || !manifest.command.eq_ignore_ascii_case(&owner.command)
    {
        return Err("installed manifest owner mismatch");
    }
    let installed_sha = crate::update_crypto::digest_hex(text.as_bytes());
    if installed_sha != owner.installed_sha256 {
        return Err("installed manifest hash mismatch");
    }
    let (archive, verification) = verify_archive(&owner.source)?;
    if !archive.id.eq_ignore_ascii_case(&owner.id)
        || !archive.command.eq_ignore_ascii_case(&owner.command)
        || verification.manifest_sha256 != owner.package_manifest_sha256
    {
        return Err("package source mismatch");
    }
    verify_installed_payloads(&owner, &archive)?;
    Ok((manifest, owner))
}

fn parse_archive_manifest(text: &str) -> Result<ArchiveManifest, &'static str> {
    let id = manifest_value(text, "id").ok_or("package missing id")?;
    let name = manifest_value(text, "name").unwrap_or(id);
    let command = manifest_value(text, "command").ok_or("package missing command")?;
    let version = manifest_value(text, "version").unwrap_or("1.0");
    let icon = manifest_value(text, "icon").unwrap_or("PK");
    let category = manifest_value(text, "category").unwrap_or("Tools");
    let permission = manifest_value(text, "permission").unwrap_or("user");
    let exec_path = manifest_value(text, "exec").ok_or("package missing exec")?;
    let aliases = manifest_value(text, "aliases").unwrap_or("");
    let associations = manifest_value(text, "associations").unwrap_or("");
    let dependencies = manifest_value(text, "depends")
        .map(split_csv)
        .unwrap_or_default();
    let payloads = parse_payload_lines(text)?;
    let min_os_version = manifest_value(text, "min_os_version")
        .and_then(parse_u64)
        .unwrap_or(1);
    if !id.starts_with("app.")
        || !safe_token(id, true)
        || command.contains('/')
        || command.contains("..")
    {
        return Err("invalid package manifest");
    }
    if !exec_path.starts_with('/')
        || (crate::vfs::vfs_kernel_read_file(exec_path).is_none()
            && !payloads
                .iter()
                .any(|payload| payload.target.eq_ignore_ascii_case(exec_path)))
    {
        return Err("package exec not found");
    }
    for dep in &dependencies {
        if !safe_token(dep, true) {
            return Err("invalid dependency");
        }
    }
    let installed_manifest = installed_manifest_text(
        id,
        name,
        command,
        version,
        icon,
        category,
        permission,
        exec_path,
        aliases,
        associations,
    );
    crate::app_metadata::validate_manifest_text(&installed_manifest)
        .map_err(|_| "invalid package manifest")?;
    let mut trust_manifest = installed_manifest.clone();
    trust_manifest.push_str("depends=");
    trust_manifest.push_str(&join_csv_field(&dependencies));
    trust_manifest.push_str("\nmin_os_version=");
    trust_manifest.push_str(&format!("{}", min_os_version));
    trust_manifest.push('\n');
    for payload in &payloads {
        trust_manifest.push_str("payload=");
        trust_manifest.push_str(&payload.target);
        trust_manifest.push('|');
        trust_manifest.push_str(&payload.source);
        trust_manifest.push('|');
        trust_manifest.push_str(&payload.sha256);
        trust_manifest.push('|');
        trust_manifest.push_str(&crate::security::format_mode(payload.mode));
        trust_manifest.push('\n');
    }
    Ok(ArchiveManifest {
        id: String::from(id),
        name: String::from(name),
        command: String::from(command),
        version: String::from(version),
        icon: String::from(icon),
        category: String::from(category),
        permission: String::from(permission),
        exec_path: String::from(exec_path),
        aliases: String::from(aliases),
        associations: String::from(associations),
        dependencies,
        payloads,
        min_os_version,
        trust_manifest,
        installed_manifest,
    })
}

fn installed_manifest_text(
    id: &str,
    name: &str,
    command: &str,
    version: &str,
    icon: &str,
    category: &str,
    permission: &str,
    exec_path: &str,
    aliases: &str,
    associations: &str,
) -> String {
    let mut manifest = String::new();
    manifest.push_str("id=");
    manifest.push_str(id);
    manifest.push_str("\nname=");
    manifest.push_str(name);
    manifest.push_str("\ncommand=");
    manifest.push_str(command);
    manifest.push_str("\nversion=");
    manifest.push_str(version);
    manifest.push_str("\nicon=");
    manifest.push_str(icon);
    manifest.push_str("\ncategory=");
    manifest.push_str(category);
    manifest.push_str("\npermission=");
    manifest.push_str(permission);
    manifest.push_str("\nexec=");
    manifest.push_str(exec_path);
    manifest.push_str("\naliases=");
    manifest.push_str(aliases);
    manifest.push_str("\nassociations=");
    manifest.push_str(associations);
    manifest.push('\n');
    manifest
}

fn parse_payload_lines(text: &str) -> Result<Vec<PackagePayload>, &'static str> {
    let mut payloads = Vec::new();
    for raw in manifest_values(text, "payload") {
        let parts: Vec<&str> = raw.split('|').map(str::trim).collect();
        if parts.len() < 2 || parts.len() > 4 {
            return Err("invalid package payload");
        }
        let target = parts[0];
        let source = parts[1];
        if !valid_payload_target(target) || !source.starts_with('/') {
            return Err("invalid package payload");
        }
        if payloads
            .iter()
            .any(|payload: &PackagePayload| payload.target.eq_ignore_ascii_case(target))
        {
            return Err("duplicate package payload");
        }
        let mut expected_sha = "";
        let mut mode_text = "644";
        match parts.len() {
            2 => {}
            3 => {
                if is_sha256_hex(parts[2]) {
                    expected_sha = parts[2];
                } else {
                    mode_text = parts[2];
                }
            }
            4 => {
                if !is_sha256_hex(parts[2]) {
                    return Err("invalid package payload");
                }
                expected_sha = parts[2];
                mode_text = parts[3];
            }
            _ => return Err("invalid package payload"),
        }
        let mode = crate::security::parse_mode(mode_text).ok_or("invalid package payload mode")?;
        let source_data =
            crate::vfs::vfs_kernel_read_file(source).ok_or("package payload missing")?;
        let actual_sha = crate::update_crypto::digest_hex(&source_data);
        if !expected_sha.is_empty() && !expected_sha.eq_ignore_ascii_case(&actual_sha) {
            return Err("package payload hash mismatch");
        }
        payloads.push(PackagePayload {
            target: String::from(target),
            source: String::from(source),
            sha256: actual_sha,
            mode,
        });
    }
    Ok(payloads)
}

fn check_archive_collision(archive: &ArchiveManifest) -> Result<(), &'static str> {
    if crate::app_metadata::is_builtin_id(&archive.id)
        || crate::app_metadata::app_by_id_or_command(&archive.command).is_some()
        || crate::app_metadata::app_by_name(&archive.name).is_some()
    {
        return Err("package collides with built-in app");
    }
    Ok(())
}

fn check_dependencies(archive: &ArchiveManifest) -> Result<(), &'static str> {
    if let Some(dep) = first_missing_dependency(&archive.dependencies) {
        return Err(match dep.as_str() {
            _ => "package dependency missing",
        });
    }
    Ok(())
}

fn first_missing_dependency(dependencies: &[String]) -> Option<String> {
    dependencies
        .iter()
        .find(|dep| !is_installed(dep))
        .map(String::from)
}

fn check_package_downgrade(archive: &ArchiveManifest) -> Result<(), &'static str> {
    let existing_version = crate::app_metadata::installed_manifest_by_id_or_command(&archive.id)
        .or_else(|| crate::app_metadata::installed_manifest_by_id_or_command(&archive.command))
        .map(|manifest| manifest.version)
        .or_else(|| owner_record_by_id_or_command(&archive.id).map(|owner| owner.version));
    if let Some(existing_version) = existing_version {
        if version_rank(&archive.version) < version_rank(&existing_version) {
            return Err("package downgrade refused");
        }
    }
    Ok(())
}

fn check_payload_targets(archive: &ArchiveManifest) -> Result<(), &'static str> {
    for payload in &archive.payloads {
        if let Some(owner) = payload_owner_by_target(&payload.target) {
            if !owner.id.eq_ignore_ascii_case(&archive.id) {
                return Err("package payload target owned");
            }
        } else if crate::vfs::vfs_kernel_read_file(&payload.target).is_some() {
            return Err("package payload target exists");
        }
    }
    Ok(())
}
