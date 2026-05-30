fn write_owner_record(
    source: &str,
    archive: &ArchiveManifest,
    verification: &PackageVerification,
) -> Result<(), &'static str> {
    let owner_path = owner_path(&archive.command);
    let manifest_path = app_manifest_path(&archive.command);
    let installed_sha = crate::update_crypto::digest_hex(archive.installed_manifest.as_bytes());
    let mut out = String::new();
    out.push_str("id=");
    out.push_str(&archive.id);
    out.push_str("\nname=");
    out.push_str(&archive.name);
    out.push_str("\ncommand=");
    out.push_str(&archive.command);
    out.push_str("\nversion=");
    out.push_str(&archive.version);
    out.push_str("\nsource=");
    out.push_str(source);
    out.push_str("\nmanifest=");
    out.push_str(&manifest_path);
    out.push_str("\ninstalled_sha256=");
    out.push_str(&installed_sha);
    out.push_str("\npackage_manifest_sha256=");
    out.push_str(&verification.manifest_sha256);
    out.push_str("\nverified_by=");
    out.push_str(&verification.key);
    out.push_str("\nalgorithm=");
    out.push_str(&verification.algorithm);
    out.push_str("\ndepends=");
    out.push_str(&join_csv_field(&verification.dependencies));
    out.push('\n');
    for payload in &verification.payloads {
        out.push_str("payload=");
        out.push_str(&payload.target);
        out.push('|');
        out.push_str(&payload.source);
        out.push('|');
        out.push_str(&payload.sha256);
        out.push('|');
        out.push_str(&crate::security::format_mode(payload.mode));
        out.push('\n');
    }
    safe_write(&owner_path, out.as_bytes())
}

fn write_installed_files(
    source: &str,
    archive: &ArchiveManifest,
    verification: &PackageVerification,
    inject_failure: bool,
) -> Result<(), &'static str> {
    let dir = app_dir(&archive.command);
    let _ = crate::vfs::vfs_kernel_create_dir(&dir);
    for (idx, payload) in archive.payloads.iter().enumerate() {
        let data =
            crate::vfs::vfs_kernel_read_file(&payload.source).ok_or("package payload missing")?;
        let actual_sha = crate::update_crypto::digest_hex(&data);
        if actual_sha != payload.sha256 {
            return Err("package payload hash mismatch");
        }
        safe_write_mode(&payload.target, &data, payload.mode)?;
        if inject_failure && idx == 0 {
            return Err("injected install failure");
        }
    }
    let manifest_path = app_manifest_path(&archive.command);
    safe_write(&manifest_path, archive.installed_manifest.as_bytes())
        .map_err(|_| "install write failed")?;
    write_owner_record(source, archive, verification)?;
    Ok(())
}

fn remove_owned_files(command: &str, owner: &OwnerRecord) -> Result<(), &'static str> {
    for payload in &owner.payloads {
        match crate::vfs::vfs_kernel_delete(&payload.target) {
            Ok(()) | Err(crate::fat32::FsError::NotFound) => {}
            Err(_) => return Err("remove payload failed"),
        }
    }
    delete_app_dir(command).map_err(|_| "remove failed")
}

fn verify_installed_payloads(
    owner: &OwnerRecord,
    archive: &ArchiveManifest,
) -> Result<(), &'static str> {
    if owner.payloads.len() != archive.payloads.len() {
        return Err("installed payload table mismatch");
    }
    for payload in &archive.payloads {
        let Some(owner_payload) = owner
            .payloads
            .iter()
            .find(|owned| owned.target.eq_ignore_ascii_case(&payload.target))
        else {
            return Err("installed payload table mismatch");
        };
        if owner_payload.source != payload.source
            || owner_payload.sha256 != payload.sha256
            || owner_payload.mode != payload.mode
        {
            return Err("installed payload table mismatch");
        }
        let data =
            crate::vfs::vfs_kernel_read_file(&payload.target).ok_or("installed payload missing")?;
        let actual_sha = crate::update_crypto::digest_hex(&data);
        if actual_sha != payload.sha256 {
            return Err("installed payload hash mismatch");
        }
    }
    Ok(())
}

fn transaction_paths(
    payloads: &[PackagePayload],
    manifest_path: &str,
    owner_path: &str,
) -> Vec<String> {
    let mut paths: Vec<String> = payloads
        .iter()
        .map(|payload| payload.target.clone())
        .collect();
    paths.push(String::from(manifest_path));
    paths.push(String::from(owner_path));
    paths
}

fn capture_rollback(paths: &[String]) -> Vec<RollbackFile> {
    let mut rollback = Vec::new();
    for path in paths {
        if rollback
            .iter()
            .any(|entry: &RollbackFile| entry.path.eq_ignore_ascii_case(path))
        {
            continue;
        }
        rollback.push(RollbackFile {
            path: path.clone(),
            before: crate::vfs::vfs_kernel_read_file(path),
            metadata: crate::vfs::vfs_kernel_metadata(path),
        });
    }
    rollback
}

fn restore_rollback(rollback: &[RollbackFile]) {
    for file in rollback {
        match &file.before {
            Some(data) => {
                if let Some(metadata) = file.metadata {
                    let _ = safe_write_metadata(
                        &file.path,
                        data,
                        metadata.uid,
                        metadata.gid,
                        metadata.mode,
                    );
                } else {
                    let _ = safe_write(&file.path, data);
                }
            }
            None => {
                let _ = crate::vfs::vfs_kernel_delete(&file.path);
            }
        }
    }
}

fn owner_record_by_id_or_command(value: &str) -> Option<OwnerRecord> {
    let dirs = crate::vfs::vfs_kernel_list_dir("/APPS")?;
    for dir in dirs.iter().filter(|entry| entry.is_dir).take(64) {
        let path = owner_path(&dir.name);
        let Some(bytes) = crate::vfs::vfs_kernel_read_file(&path) else {
            continue;
        };
        let Ok(text) = core::str::from_utf8(&bytes) else {
            continue;
        };
        let Some(owner) = parse_owner_record(text) else {
            continue;
        };
        if owner.id.eq_ignore_ascii_case(value)
            || owner.command.eq_ignore_ascii_case(value)
            || owner.name.eq_ignore_ascii_case(value)
        {
            return Some(owner);
        }
    }
    None
}

fn payload_owner_by_target(target: &str) -> Option<OwnerRecord> {
    let dirs = crate::vfs::vfs_kernel_list_dir("/APPS")?;
    for dir in dirs.iter().filter(|entry| entry.is_dir).take(64) {
        let path = owner_path(&dir.name);
        let Some(bytes) = crate::vfs::vfs_kernel_read_file(&path) else {
            continue;
        };
        let Ok(text) = core::str::from_utf8(&bytes) else {
            continue;
        };
        let Some(owner) = parse_owner_record(text) else {
            continue;
        };
        if owner
            .payloads
            .iter()
            .any(|payload| payload.target.eq_ignore_ascii_case(target))
        {
            return Some(owner);
        }
    }
    None
}

fn parse_owner_record(text: &str) -> Option<OwnerRecord> {
    let mut payloads = Vec::new();
    for raw in manifest_values(text, "payload") {
        let parts: Vec<&str> = raw.split('|').map(str::trim).collect();
        if parts.len() < 3 {
            continue;
        }
        let mode = parts
            .get(3)
            .and_then(|mode| crate::security::parse_mode(mode))
            .unwrap_or(0o644);
        payloads.push(PackagePayload {
            target: String::from(parts[0]),
            source: String::from(parts[1]),
            sha256: String::from(parts[2]),
            mode,
        });
    }
    Some(OwnerRecord {
        id: String::from(manifest_value(text, "id")?),
        name: String::from(manifest_value(text, "name").unwrap_or("Package")),
        command: String::from(manifest_value(text, "command")?),
        version: String::from(manifest_value(text, "version").unwrap_or("1.0")),
        source: String::from(manifest_value(text, "source")?),
        manifest_path: String::from(manifest_value(text, "manifest")?),
        installed_sha256: String::from(manifest_value(text, "installed_sha256")?),
        package_manifest_sha256: String::from(manifest_value(text, "package_manifest_sha256")?),
        verified_by: String::from(manifest_value(text, "verified_by")?),
        algorithm: String::from(manifest_value(text, "algorithm").unwrap_or(TRUST_ALGORITHM)),
        dependencies: manifest_value(text, "depends")
            .map(split_csv)
            .unwrap_or_default(),
        payloads,
    })
}

fn owner_to_manifest(owner: &OwnerRecord) -> crate::app_metadata::AppManifest {
    crate::app_metadata::AppManifest {
        id: owner.id.clone(),
        name: owner.name.clone(),
        command: owner.command.clone(),
        version: owner.version.clone(),
        icon: String::from("PK"),
        category: String::from("Tools"),
        permission: String::from("user"),
        exec_path: String::new(),
        aliases: Vec::new(),
        associations: Vec::new(),
    }
}

fn manifest_from_text(text: &str) -> Result<crate::app_metadata::AppManifest, &'static str> {
    let id = manifest_value(text, "id").ok_or("missing id")?;
    let name = manifest_value(text, "name").unwrap_or(id);
    let command = manifest_value(text, "command").ok_or("missing command")?;
    let version = manifest_value(text, "version").unwrap_or("1.0");
    let icon = manifest_value(text, "icon").unwrap_or("PK");
    let category = manifest_value(text, "category").unwrap_or("Tools");
    let permission = manifest_value(text, "permission").unwrap_or("user");
    let exec_path = manifest_value(text, "exec").ok_or("missing exec")?;
    Ok(crate::app_metadata::AppManifest {
        id: String::from(id),
        name: String::from(name),
        command: String::from(command),
        version: String::from(version),
        icon: String::from(icon),
        category: String::from(category),
        permission: String::from(permission),
        exec_path: String::from(exec_path),
        aliases: manifest_value(text, "aliases")
            .map(split_csv)
            .unwrap_or_default(),
        associations: manifest_value(text, "associations")
            .map(split_csv)
            .unwrap_or_default(),
    })
}
