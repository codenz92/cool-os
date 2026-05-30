pub fn init() {
    ensure_layout();
    let mut installed = Vec::new();
    for app in crate::app_metadata::APPS {
        installed.push(String::from(app.id));
        let dir = app_dir(app.command);
        let _ = crate::vfs::vfs_kernel_create_dir(&dir);
        let manifest = manifest_for(app);
        let path = app_manifest_path(app.command);
        match crate::vfs::vfs_kernel_read_file(&path) {
            Some(bytes) if bytes == manifest.as_bytes() => {}
            Some(_) => {
                let _ = safe_write(&path, manifest.as_bytes());
            }
            None => {
                let _ = safe_write(&path, manifest.as_bytes());
            }
        }
    }
    ensure_fixture_signature("/Packages/guidemo.pkg");
    for manifest in crate::app_metadata::installed_app_manifests() {
        if !installed
            .iter()
            .any(|id| id.eq_ignore_ascii_case(&manifest.id))
        {
            installed.push(manifest.id);
        }
    }
    *INSTALLED.lock() = installed;
    crate::event_bus::emit("packages", "init", "built-in package manifests ready");
}

pub fn list() -> Vec<Package> {
    let installed = INSTALLED.lock();
    let mut packages: Vec<Package> = crate::app_metadata::APPS
        .iter()
        .map(|app| Package {
            id: String::from(app.id),
            name: String::from(app.name),
            version: String::from("builtin"),
            permissions: String::from(app.permission),
            exec_path: exec_for_app(app),
            installed: installed.iter().any(|id| id == app.id),
            builtin: true,
        })
        .collect();
    for manifest in crate::app_metadata::installed_app_manifests() {
        if crate::app_metadata::is_builtin_id(&manifest.id) {
            continue;
        }
        packages.push(Package {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            permissions: manifest.permission,
            exec_path: manifest.exec_path,
            installed: true,
            builtin: false,
        });
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

pub fn lines() -> Vec<String> {
    list()
        .iter()
        .map(|pkg| {
            format!(
                "{} {} version={} perms={} exec={} {} {}",
                pkg.id,
                pkg.name,
                pkg.version,
                pkg.permissions,
                pkg.exec_path,
                if pkg.installed {
                    "installed"
                } else {
                    "removed"
                },
                if pkg.builtin { "builtin" } else { "package" },
            )
        })
        .collect()
}

pub fn install(id_or_command: &str) -> Result<(), &'static str> {
    if is_archive_path(id_or_command) {
        return install_archive(id_or_command);
    }
    let app = find_app(id_or_command).ok_or("unknown package")?;
    let mut installed = INSTALLED.lock();
    if !installed.iter().any(|id| id == app.id) {
        installed.push(String::from(app.id));
    }
    let dir = app_dir(app.command);
    let _ = crate::vfs::vfs_kernel_create_dir(&dir);
    let path = app_manifest_path(app.command);
    let manifest = manifest_for(app);
    let _ = safe_write(&path, manifest.as_bytes());
    crate::event_bus::emit("packages", "install", app.id);
    Ok(())
}

pub fn install_archive(path: &str) -> Result<(), &'static str> {
    install_archive_inner(path, false)
}

pub fn install_archive_with_fault(path: &str) -> Result<(), &'static str> {
    install_archive_inner(path, true)
}

fn install_archive_inner(path: &str, inject_failure: bool) -> Result<(), &'static str> {
    ensure_layout();
    let (archive, verification) = verify_archive(path)?;
    check_archive_collision(&archive)?;
    check_dependencies(&archive)?;
    check_package_downgrade(&archive)?;
    check_payload_targets(&archive)?;

    let dir = app_dir(&archive.command);
    let manifest_path = app_manifest_path(&archive.command);
    let owner_path = owner_path(&archive.command);
    let app_dir_existed = crate::vfs::vfs_kernel_list_dir(&dir).is_some();
    let rollback = capture_rollback(&transaction_paths(
        &archive.payloads,
        &manifest_path,
        &owner_path,
    ));
    write_transaction(
        "running",
        "install",
        &archive.id,
        &[
            format!("command={}", archive.command),
            format!("payloads={}", archive.payloads.len()),
        ],
    );
    let result = write_installed_files(path, &archive, &verification, inject_failure);
    if result.is_err() {
        restore_rollback(&rollback);
        if !app_dir_existed {
            let _ = delete_app_dir(&archive.command);
        }
        write_transaction(
            "rolled-back",
            "install",
            &archive.id,
            &[format!(
                "error={}",
                result.err().unwrap_or("install failed")
            )],
        );
        append_log(
            "install-rollback",
            &[format!("id={} command={}", archive.id, archive.command)],
        );
        return Err("package transaction rollback");
    }
    let mut installed = INSTALLED.lock();
    if !installed
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&archive.id))
    {
        installed.push(archive.id.clone());
    }
    append_log(
        "install",
        &[format!(
            "id={} command={} version={} key={} source={} payloads={}",
            archive.id,
            archive.command,
            archive.version,
            verification.key,
            path,
            archive.payloads.len()
        )],
    );
    write_transaction(
        "clean",
        "install",
        &archive.id,
        &[format!("payloads={}", archive.payloads.len())],
    );
    crate::event_bus::emit("packages", "install-pkg", &archive.id);
    crate::println!(
        "[pkg] installed {} name={} exec={} payloads={}",
        archive.id,
        archive.name,
        archive.exec_path,
        archive.payloads.len()
    );
    Ok(())
}

pub fn uninstall(id_or_command: &str) -> Result<(), &'static str> {
    if let Some(app) = find_app(id_or_command) {
        INSTALLED.lock().retain(|id| id != app.id);
        let _ = delete_app_dir(app.command);
        append_log("remove", &[format!("id={} builtin=true", app.id)]);
        crate::event_bus::emit("packages", "remove", app.id);
        crate::println!("[pkg] removed {}", app.id);
        return Ok(());
    }
    let Some(owner) = owner_record_by_id_or_command(id_or_command) else {
        let manifest = crate::app_metadata::installed_manifest_by_id_or_command(id_or_command)
            .ok_or("unknown package")?;
        INSTALLED
            .lock()
            .retain(|id| !id.eq_ignore_ascii_case(&manifest.id));
        delete_app_dir(&manifest.command).map_err(|_| "remove failed")?;
        append_log(
            "remove",
            &[format!(
                "id={} command={} legacy=true",
                manifest.id, manifest.command
            )],
        );
        crate::event_bus::emit("packages", "remove", &manifest.id);
        crate::println!("[pkg] removed {}", manifest.id);
        return Ok(());
    };
    let manifest = crate::app_metadata::installed_manifest_by_id_or_command(id_or_command)
        .unwrap_or_else(|| owner_to_manifest(&owner));
    let manifest_path = app_manifest_path(&manifest.command);
    let owner_path = owner_path(&manifest.command);
    let app_dir_existed = crate::vfs::vfs_kernel_list_dir(&app_dir(&manifest.command)).is_some();
    let rollback = capture_rollback(&transaction_paths(
        &owner.payloads,
        &manifest_path,
        &owner_path,
    ));
    write_transaction(
        "running",
        "remove",
        &manifest.id,
        &[format!("payloads={}", owner.payloads.len())],
    );
    let result = remove_owned_files(&manifest.command, &owner);
    if result.is_err() {
        restore_rollback(&rollback);
        if !app_dir_existed {
            let _ = delete_app_dir(&manifest.command);
        }
        write_transaction(
            "rolled-back",
            "remove",
            &manifest.id,
            &[format!("error={}", result.err().unwrap_or("remove failed"))],
        );
        append_log(
            "remove-rollback",
            &[format!("id={} command={}", manifest.id, manifest.command)],
        );
        return Err("package transaction rollback");
    }
    INSTALLED
        .lock()
        .retain(|id| !id.eq_ignore_ascii_case(&manifest.id));
    append_log(
        "remove",
        &[format!(
            "id={} command={} payloads={}",
            manifest.id,
            manifest.command,
            owner.payloads.len()
        )],
    );
    write_transaction(
        "clean",
        "remove",
        &manifest.id,
        &[format!("payloads={}", owner.payloads.len())],
    );
    crate::event_bus::emit("packages", "remove", &manifest.id);
    crate::println!("[pkg] removed {}", manifest.id);
    Ok(())
}

pub fn launch(id_or_command: &str, args: &[&str]) -> Result<PackageLaunch, &'static str> {
    let manifest = launch_manifest(id_or_command).ok_or("unknown package")?;
    if !is_installed(&manifest.id) {
        return Err("package not installed");
    }
    if !crate::app_metadata::is_builtin_id(&manifest.id) {
        verify_installed_package(&manifest.id)?;
    }
    if !manifest.exec_path.starts_with('/') {
        return Err("package has no userspace executable");
    }
    let credentials = crate::security::package_credentials(&manifest.permission);
    let pid =
        crate::elf::spawn_elf_process_with_credentials(&manifest.exec_path, args, credentials)
            .map_err(|err| err.as_str())?;
    crate::app_lifecycle::record_process_start(pid, &manifest.name, &manifest.exec_path);
    crate::app_lifecycle::record_app(&manifest.name);
    crate::println!(
        "[pkg] launched {} exec={} pid={}",
        manifest.id,
        manifest.exec_path,
        pid
    );
    Ok(PackageLaunch {
        pid,
        name: manifest.name,
        exec_path: manifest.exec_path,
    })
}

pub fn is_installed(id: &str) -> bool {
    INSTALLED
        .lock()
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(id))
}

pub fn launch_manifest(id_or_command: &str) -> Option<crate::app_metadata::AppManifest> {
    if let Some(app) = find_app(id_or_command) {
        return Some(builtin_manifest(app));
    }
    crate::app_metadata::installed_manifest_by_id_or_command(id_or_command)
}

pub fn key_lines() -> Vec<String> {
    ensure_layout();
    let mut lines = alloc::vec![
        format!(
            "keys={} built_in=4 signature_required=yes repair=yes",
            TRUST_KEYS_PATH
        ),
        String::from("signature_sidecar=<package>.sig"),
    ];
    for key in trusted_keys() {
        lines.push(format!(
            "key={} algorithm={} status={} scope=packages not_before={} not_after={} generation={} public={}",
            key.id,
            key.algorithm,
            key.status,
            key.not_before,
            key.not_after,
            key.generation,
            crate::update_crypto::hex(&key.public_key)
        ));
    }
    lines
}

pub fn verify_lines(value: &str) -> Vec<String> {
    if is_archive_path(value) {
        return archive_verify_lines(value);
    }
    if let Some(app) = find_app(value) {
        if !is_installed(app.id) {
            return alloc::vec![String::from(
                "installed_trust=failed error=package not installed"
            )];
        }
        return alloc::vec![
            format!(
                "installed_trust=ok id={} command={} version=builtin key=built-in algorithm=kernel",
                app.id, app.command
            ),
            format!("source=built-in exec={}", exec_for_app(app)),
        ];
    }
    match verify_installed_package(value) {
        Ok((manifest, owner)) => alloc::vec![
            format!(
                "installed_trust=ok id={} command={} version={} key={} algorithm={}",
                manifest.id, manifest.command, manifest.version, owner.verified_by, owner.algorithm
            ),
            format!("manifest_sha256={}", owner.installed_sha256),
            format!("source={}", owner.source),
            format!("payloads=ok count={}", owner.payloads.len()),
        ],
        Err(err) => alloc::vec![format!("installed_trust=failed error={}", err)],
    }
}

pub fn info_lines(value: &str) -> Vec<String> {
    if is_archive_path(value) {
        let mut lines = archive_info_lines(value);
        lines.extend(archive_verify_lines(value));
        return lines;
    }
    if let Some(app) = find_app(value) {
        return alloc::vec![
            format!("id={} name={} command={}", app.id, app.name, app.command),
            format!(
                "version=builtin permission={} exec={}",
                app.permission,
                exec_for_app(app)
            ),
            format!(
                "installed={} trust=builtin",
                if is_installed(app.id) { "yes" } else { "no" }
            ),
        ];
    }
    match owner_record_by_id_or_command(value) {
        Some(owner) => alloc::vec![
            format!(
                "id={} name={} command={}",
                owner.id, owner.name, owner.command
            ),
            format!("version={} source={}", owner.version, owner.source),
            format!(
                "verified_by={} algorithm={} package_manifest_sha256={}",
                owner.verified_by, owner.algorithm, owner.package_manifest_sha256
            ),
            format!(
                "manifest={} installed_sha256={}",
                owner.manifest_path, owner.installed_sha256
            ),
            format!("depends={}", join_csv(&owner.dependencies)),
            format!("payloads={}", owner.payloads.len()),
            payload_summary_lines(&owner.payloads),
        ],
        None => alloc::vec![String::from("package=unknown")],
    }
}

pub fn transaction_lines() -> Vec<String> {
    let Some(bytes) = crate::vfs::vfs_kernel_read_file(PACKAGE_TXN_PATH) else {
        return alloc::vec![String::from("transaction=clean")];
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return alloc::vec![String::from("transaction=unreadable")];
    };
    let mut lines = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            lines.push(String::from(line));
        }
    }
    if lines.is_empty() {
        lines.push(String::from("transaction=clean"));
    }
    lines
}

pub fn sign_archive(path: &str) -> Result<(), &'static str> {
    sign_archive_as(path, TRUST_KEY_ID)
}

pub fn sign_archive_as(path: &str, key_id: &str) -> Result<(), &'static str> {
    ensure_layout();
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package file not found")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package is not UTF-8 manifest")?;
    let archive = parse_archive_manifest(text)?;
    write_signature(path, &archive, key_id)?;
    append_log(
        "sign",
        &[format!(
            "path={} id={} version={} key={}",
            path, archive.id, archive.version, key_id
        )],
    );
    Ok(())
}

pub fn remove_signature(path: &str) -> Result<(), &'static str> {
    let sig_path = signature_path(path);
    match crate::vfs::vfs_kernel_delete(&sig_path) {
        Ok(()) => {
            append_log("unsign", &[format!("path={} signature={}", path, sig_path)]);
            Ok(())
        }
        Err(crate::fat32::FsError::NotFound) => Err("package is unsigned"),
        Err(_) => Err("signature delete failed"),
    }
}

pub fn tamper_archive_name(path: &str) -> Result<(), &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package file not found")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package is not UTF-8 manifest")?;
    let mut out = String::new();
    let mut changed = false;
    for line in text.lines() {
        if line.trim_start().starts_with("name=") {
            out.push_str("name=Tampered Package\n");
            changed = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !changed {
        out.push_str("name=Tampered Package\n");
    }
    safe_write(path, out.as_bytes())?;
    append_log("tamper", &[format!("path={} signature=preserved", path)]);
    Ok(())
}

pub fn set_archive_dependencies(path: &str, deps: &str) -> Result<(), &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package file not found")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package is not UTF-8 manifest")?;
    let dependencies = split_csv(deps);
    for dep in &dependencies {
        if !safe_token(dep, true) {
            return Err("invalid dependency");
        }
    }
    let mut out = String::new();
    let mut changed = false;
    for line in text.lines() {
        if line.trim_start().starts_with("depends=") {
            out.push_str("depends=");
            out.push_str(&join_csv_field(&dependencies));
            out.push('\n');
            changed = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !changed {
        out.push_str("depends=");
        out.push_str(&join_csv_field(&dependencies));
        out.push('\n');
    }
    safe_write(path, out.as_bytes())?;
    append_log(
        "depends",
        &[format!("path={} depends={}", path, join_csv(&dependencies))],
    );
    Ok(())
}

pub fn break_installed(id_or_command: &str) -> Result<(), &'static str> {
    let owner = owner_record_by_id_or_command(id_or_command).ok_or("package owner missing")?;
    safe_write(&owner.manifest_path, b"broken=true\n")?;
    append_log(
        "break",
        &[format!("id={} manifest={}", owner.id, owner.manifest_path)],
    );
    Ok(())
}

pub fn break_installed_payload(id_or_command: &str) -> Result<(), &'static str> {
    let owner = owner_record_by_id_or_command(id_or_command).ok_or("package owner missing")?;
    let payload = owner.payloads.first().ok_or("package has no payloads")?;
    safe_write(&payload.target, b"broken package payload\n")?;
    append_log(
        "break-payload",
        &[format!("id={} target={}", owner.id, payload.target)],
    );
    Ok(())
}

pub fn tamper_archive_payload(path: &str) -> Result<(), &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package file not found")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package is not UTF-8 manifest")?;
    let archive = parse_archive_manifest(text)?;
    let payload = archive.payloads.first().ok_or("package has no payloads")?;
    safe_write(&payload.source, b"tampered package source payload\n")?;
    append_log(
        "tamper-payload",
        &[format!("path={} source={}", path, payload.source)],
    );
    Ok(())
}

pub fn repair(id_or_command: &str) -> Result<(), &'static str> {
    let owner = owner_record_by_id_or_command(id_or_command).ok_or("package owner missing")?;
    let (archive, verification) = verify_archive(&owner.source)?;
    if !archive.id.eq_ignore_ascii_case(&owner.id)
        || !archive.command.eq_ignore_ascii_case(&owner.command)
    {
        return Err("package source mismatch");
    }
    check_dependencies(&archive)?;
    check_payload_targets(&archive)?;
    let owner_path = owner_path(&archive.command);
    let rollback = capture_rollback(&transaction_paths(
        &archive.payloads,
        &owner.manifest_path,
        &owner_path,
    ));
    write_transaction(
        "running",
        "repair",
        &archive.id,
        &[format!("payloads={}", archive.payloads.len())],
    );
    let result = write_installed_files(&owner.source, &archive, &verification, false);
    if result.is_err() {
        restore_rollback(&rollback);
        write_transaction(
            "rolled-back",
            "repair",
            &archive.id,
            &[format!("error={}", result.err().unwrap_or("repair failed"))],
        );
        append_log(
            "repair-rollback",
            &[format!("id={} command={}", archive.id, archive.command)],
        );
        return Err("package transaction rollback");
    }
    let mut installed = INSTALLED.lock();
    if !installed
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&archive.id))
    {
        installed.push(archive.id.clone());
    }
    append_log(
        "repair",
        &[format!(
            "id={} command={} key={} source={} payloads={}",
            archive.id,
            archive.command,
            verification.key,
            owner.source,
            archive.payloads.len()
        )],
    );
    write_transaction(
        "clean",
        "repair",
        &archive.id,
        &[format!("payloads={}", archive.payloads.len())],
    );
    crate::event_bus::emit("packages", "repair", &archive.id);
    Ok(())
}

pub fn recovery_lines() -> Vec<String> {
    let mut signed = 0usize;
    let mut broken = 0usize;
    for package in list()
        .into_iter()
        .filter(|pkg| !pkg.builtin && pkg.installed)
    {
        match verify_installed_package(&package.id) {
            Ok(_) => signed += 1,
            Err(_) => broken += 1,
        }
    }
    let mut lines = alloc::vec![
        format!(
            "packages installed={} signed={} broken={}",
            signed + broken,
            signed,
            broken
        ),
        format!("package_keys={} signature_required=yes", TRUST_KEYS_PATH),
    ];
    if broken == 0 {
        lines.push(format!("package_trust=ok signed={}", signed));
    } else {
        lines.push(format!("package_trust=failed broken={}", broken));
    }
    lines
}

pub fn status_lines() -> Vec<String> {
    let mut lines = alloc::vec![
        format!("keys={} log={}", TRUST_KEYS_PATH, PACKAGE_LOG_PATH),
        format!(
            "archives=/Packages signature_sidecar=<package>.sig transaction={}",
            PACKAGE_TXN_PATH
        ),
    ];
    lines.extend(recovery_lines());
    lines.extend(transaction_lines());
    for package in list()
        .into_iter()
        .filter(|pkg| !pkg.builtin && pkg.installed)
    {
        match verify_installed_package(&package.id) {
            Ok((manifest, owner)) => lines.push(format!(
                "package={} command={} version={} trust=ok key={} source={} payloads={}",
                manifest.id,
                manifest.command,
                manifest.version,
                owner.verified_by,
                owner.source,
                owner.payloads.len()
            )),
            Err(err) => lines.push(format!("package={} trust=failed error={}", package.id, err)),
        }
    }
    lines
}

pub fn history_lines() -> Vec<String> {
    let Some(data) = crate::vfs::vfs_kernel_read_file(PACKAGE_LOG_PATH) else {
        return alloc::vec![format!("history={} missing", PACKAGE_LOG_PATH)];
    };
    let Ok(text) = core::str::from_utf8(&data) else {
        return alloc::vec![format!("history={} unreadable", PACKAGE_LOG_PATH)];
    };
    let mut lines = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            lines.push(String::from(line));
        }
    }
    if lines.is_empty() {
        lines.push(format!("history={} empty", PACKAGE_LOG_PATH));
    }
    lines
}
