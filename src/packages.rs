extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

const TRUST_KEYS_PATH: &str = "/CONFIG/PACKAGE-KEYS.TXT";
const PACKAGE_LOG_PATH: &str = "/LOGS/PACKAGES.TXT";
const PACKAGE_TXN_PATH: &str = "/LOGS/PACKAGE-TXN.TXT";
const TRUST_ALGORITHM: &str = "ed25519";
const TRUST_KEY_ID: &str = "phase69-pkg-a";
const CURRENT_OS_VERSION: u64 = 70;
const CURRENT_EPOCH: u64 = 70;
const MAX_EPOCH: u64 = 9999;

const PKG_A_SEED: [u8; 32] = [
    0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11, 0x4e, 0x0f,
    0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed, 0x4f, 0xb8, 0xa6, 0xfb,
];
const PKG_A_PUBLIC: [u8; 32] = [
    0x3d, 0x40, 0x17, 0xc3, 0xe8, 0x43, 0x89, 0x5a, 0x92, 0xb7, 0x0a, 0xa7, 0x4d, 0x1b, 0x7e, 0xbc,
    0x9c, 0x98, 0x2c, 0xcf, 0x2e, 0xc4, 0x96, 0x8c, 0xc0, 0xcd, 0x55, 0xf1, 0x2a, 0xf4, 0x66, 0x0c,
];
const PKG_B_SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];
const PKG_B_PUBLIC: [u8; 32] = [
    0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07, 0x3a,
    0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07, 0x51, 0x1a,
];
const PKG_REVOKED_SEED: [u8; 32] = [
    0xc5, 0xaa, 0x8d, 0xf4, 0x3f, 0x9f, 0x83, 0x7b, 0xed, 0xb7, 0x44, 0x2f, 0x31, 0xdc, 0xb7, 0xb1,
    0x66, 0xd3, 0x85, 0x35, 0x07, 0x6f, 0x09, 0x4b, 0x85, 0xce, 0x3a, 0x2e, 0x0b, 0x44, 0x58, 0xf7,
];
const PKG_REVOKED_PUBLIC: [u8; 32] = [
    0xfc, 0x51, 0xcd, 0x8e, 0x62, 0x18, 0xa1, 0xa3, 0x8d, 0xa4, 0x7e, 0xd0, 0x02, 0x30, 0xf0, 0x58,
    0x08, 0x16, 0xed, 0x13, 0xba, 0x33, 0x03, 0xac, 0x5d, 0xeb, 0x91, 0x15, 0x48, 0x90, 0x80, 0x25,
];
const PKG_EXPIRED_SEED: [u8; 32] = [
    0xf5, 0xe5, 0x76, 0x7c, 0xf1, 0x53, 0x31, 0x95, 0x17, 0x63, 0x0f, 0x22, 0x68, 0x76, 0xb8, 0x6c,
    0x81, 0x60, 0xcc, 0x58, 0x3b, 0xc0, 0x13, 0x74, 0x4c, 0x6b, 0xf2, 0x55, 0xf5, 0xcc, 0x0e, 0xe5,
];
const PKG_EXPIRED_PUBLIC: [u8; 32] = [
    0x27, 0x81, 0x17, 0xfc, 0x14, 0x4c, 0x72, 0x34, 0x0f, 0x67, 0xd0, 0xf2, 0x31, 0x6e, 0x83, 0x86,
    0xce, 0xff, 0xbf, 0x2b, 0x24, 0x28, 0xc9, 0xc5, 0x1f, 0xef, 0x7c, 0x59, 0x7f, 0x1d, 0x42, 0x6e,
];

#[derive(Clone)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub permissions: String,
    pub exec_path: String,
    pub installed: bool,
    pub builtin: bool,
}

#[derive(Clone)]
pub struct PackageLaunch {
    pub pid: usize,
    pub name: String,
    pub exec_path: String,
}

struct ArchiveManifest {
    id: String,
    name: String,
    command: String,
    version: String,
    icon: String,
    category: String,
    permission: String,
    exec_path: String,
    aliases: String,
    associations: String,
    dependencies: Vec<String>,
    payloads: Vec<PackagePayload>,
    min_os_version: u64,
    trust_manifest: String,
    installed_manifest: String,
}

struct PackageSignature {
    key: String,
    algorithm: String,
    package_id: String,
    package_version: String,
    issued_epoch: u64,
    manifest_sha256: String,
    signature: String,
}

struct TrustedKey {
    id: String,
    algorithm: String,
    status: String,
    public_key: [u8; 32],
    not_before: u64,
    not_after: u64,
    generation: u64,
}

struct PackageVerification {
    id: String,
    command: String,
    version: String,
    key: String,
    algorithm: String,
    manifest_sha256: String,
    signature_path: String,
    dependencies: Vec<String>,
    payloads: Vec<PackagePayload>,
}

struct OwnerRecord {
    id: String,
    name: String,
    command: String,
    version: String,
    source: String,
    manifest_path: String,
    installed_sha256: String,
    package_manifest_sha256: String,
    verified_by: String,
    algorithm: String,
    dependencies: Vec<String>,
    payloads: Vec<PackagePayload>,
}

#[derive(Clone)]
struct PackagePayload {
    target: String,
    source: String,
    sha256: String,
    mode: u16,
}

struct RollbackFile {
    path: String,
    before: Option<Vec<u8>>,
    metadata: Option<crate::vfs::FileMetadata>,
}

static INSTALLED: Mutex<Vec<String>> = Mutex::new(Vec::new());

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

fn package_signature(path: &str) -> Result<PackageSignature, &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package is unsigned")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package signature is not UTF-8")?;
    let signature = PackageSignature {
        key: String::from(manifest_value(text, "key").unwrap_or("")),
        algorithm: String::from(manifest_value(text, "algorithm").unwrap_or("")),
        package_id: String::from(manifest_value(text, "package_id").unwrap_or("")),
        package_version: String::from(manifest_value(text, "package_version").unwrap_or("")),
        issued_epoch: manifest_value(text, "issued_epoch")
            .and_then(parse_u64)
            .unwrap_or(0),
        manifest_sha256: String::from(manifest_value(text, "manifest_sha256").unwrap_or("")),
        signature: String::from(manifest_value(text, "signature").unwrap_or("")),
    };
    if signature.key.is_empty()
        || signature.algorithm.is_empty()
        || signature.package_id.is_empty()
        || signature.package_version.is_empty()
        || signature.issued_epoch == 0
        || signature.manifest_sha256.is_empty()
        || signature.signature.is_empty()
    {
        return Err("package signature incomplete");
    }
    Ok(signature)
}

fn write_signature(
    path: &str,
    archive: &ArchiveManifest,
    key_id: &str,
) -> Result<(), &'static str> {
    let seed = signing_seed(key_id).ok_or("signing key unavailable")?;
    let manifest_sha256 = crate::update_crypto::digest_hex(archive.trust_manifest.as_bytes());
    let signature = crate::update_crypto::hex(&crate::update_crypto::ed25519_sign(
        seed,
        archive.trust_manifest.as_bytes(),
    ));
    let mut out = String::from("coolOS package signature\n");
    out.push_str("key=");
    out.push_str(key_id);
    out.push_str("\nalgorithm=");
    out.push_str(TRUST_ALGORITHM);
    out.push_str("\npackage_id=");
    out.push_str(&archive.id);
    out.push_str("\npackage_version=");
    out.push_str(&archive.version);
    out.push_str("\nissued_epoch=");
    out.push_str(&format!("{}", CURRENT_EPOCH));
    out.push_str("\nmanifest_sha256=");
    out.push_str(&manifest_sha256);
    out.push_str("\nsignature=");
    out.push_str(&signature);
    out.push('\n');
    safe_write(&signature_path(path), out.as_bytes())
}

fn trusted_keys() -> Vec<TrustedKey> {
    ensure_trust_keys_file();
    let Some(bytes) = crate::vfs::vfs_kernel_read_file(TRUST_KEYS_PATH) else {
        return built_in_trusted_keys();
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return built_in_trusted_keys();
    };
    let mut keys = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.starts_with("key=") {
            continue;
        }
        let Some(id) = token_value(line, "key=") else {
            continue;
        };
        let Some(public_hex) = token_value(line, "public=") else {
            continue;
        };
        let Some(public_key) = crate::update_crypto::hex_to_32(public_hex) else {
            continue;
        };
        keys.push(TrustedKey {
            id: String::from(id),
            algorithm: String::from(token_value(line, "algorithm=").unwrap_or(TRUST_ALGORITHM)),
            status: String::from(token_value(line, "status=").unwrap_or("untrusted")),
            public_key,
            not_before: token_value(line, "not_before=")
                .and_then(parse_u64)
                .unwrap_or(1),
            not_after: token_value(line, "not_after=")
                .and_then(parse_u64)
                .unwrap_or(MAX_EPOCH),
            generation: token_value(line, "generation=")
                .and_then(parse_u64)
                .unwrap_or(0),
        });
    }
    if keys.is_empty() {
        built_in_trusted_keys()
    } else {
        keys
    }
}

fn find_trusted_key(id: &str) -> Option<TrustedKey> {
    trusted_keys().into_iter().find(|key| key.id == id)
}

fn built_in_trusted_keys() -> Vec<TrustedKey> {
    alloc::vec![
        TrustedKey {
            id: String::from("phase69-pkg-a"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: PKG_A_PUBLIC,
            not_before: 1,
            not_after: MAX_EPOCH,
            generation: 1,
        },
        TrustedKey {
            id: String::from("phase69-pkg-b"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: PKG_B_PUBLIC,
            not_before: 1,
            not_after: MAX_EPOCH,
            generation: 2,
        },
        TrustedKey {
            id: String::from("phase69-pkg-revoked"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("revoked"),
            public_key: PKG_REVOKED_PUBLIC,
            not_before: 1,
            not_after: MAX_EPOCH,
            generation: 0,
        },
        TrustedKey {
            id: String::from("phase69-pkg-expired"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: PKG_EXPIRED_PUBLIC,
            not_before: 1,
            not_after: 1,
            generation: 0,
        },
    ]
}

fn signing_seed(key_id: &str) -> Option<&'static [u8; 32]> {
    match key_id {
        "phase69-pkg-a" => Some(&PKG_A_SEED),
        "phase69-pkg-b" => Some(&PKG_B_SEED),
        "phase69-pkg-revoked" => Some(&PKG_REVOKED_SEED),
        "phase69-pkg-expired" => Some(&PKG_EXPIRED_SEED),
        "phase69-pkg-unknown" => Some(&PKG_B_SEED),
        _ => None,
    }
}

fn ensure_layout() {
    let _ = crate::vfs::vfs_kernel_create_dir("/APPS");
    let _ = crate::vfs::vfs_kernel_create_dir("/CONFIG");
    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
    ensure_trust_keys_file();
}

fn ensure_trust_keys_file() {
    if let Some(bytes) = crate::vfs::vfs_kernel_read_file(TRUST_KEYS_PATH) {
        if core::str::from_utf8(&bytes)
            .map(|text| text.contains("key=phase69-pkg-a") && text.contains("algorithm=ed25519"))
            .unwrap_or(false)
        {
            return;
        }
    }
    let mut out = String::from("coolOS package trust keys\n");
    out.push_str("# public keys only; signing seeds are not stored in this metadata file\n");
    for key in built_in_trusted_keys() {
        out.push_str("key=");
        out.push_str(&key.id);
        out.push_str(" algorithm=");
        out.push_str(&key.algorithm);
        out.push_str(" status=");
        out.push_str(&key.status);
        out.push_str(" scope=packages public=");
        out.push_str(&crate::update_crypto::hex(&key.public_key));
        out.push_str(" not_before=");
        out.push_str(&format!("{}", key.not_before));
        out.push_str(" not_after=");
        out.push_str(&format!("{}", key.not_after));
        out.push_str(" generation=");
        out.push_str(&format!("{}", key.generation));
        out.push_str(" source=built-in\n");
    }
    let _ = safe_write(TRUST_KEYS_PATH, out.as_bytes());
}

fn ensure_fixture_signature(path: &str) {
    if crate::vfs::vfs_kernel_read_file(path).is_none()
        || crate::vfs::vfs_kernel_read_file(&signature_path(path)).is_some()
    {
        return;
    }
    let _ = sign_archive(path);
}

fn append_log(action: &str, details: &[String]) {
    ensure_layout();
    let mut log = match crate::vfs::vfs_kernel_read_file(PACKAGE_LOG_PATH) {
        Some(bytes) => core::str::from_utf8(&bytes)
            .map(String::from)
            .unwrap_or_else(|_| String::from("coolOS package journal\n")),
        None => String::from("coolOS package journal\n"),
    };
    if log.len() > 8192 {
        log = String::from("coolOS package journal\ntrimmed=true\n");
    }
    log.push_str(&format!(
        "tick={} action={}\n",
        crate::interrupts::ticks(),
        action
    ));
    for detail in details {
        log.push_str(detail);
        log.push('\n');
    }
    let _ = safe_write(PACKAGE_LOG_PATH, log.as_bytes());
}

fn write_transaction(status: &str, action: &str, id: &str, details: &[String]) {
    let mut out = String::new();
    out.push_str("transaction=");
    out.push_str(status);
    out.push_str(" action=");
    out.push_str(action);
    out.push_str(" id=");
    out.push_str(id);
    out.push_str(" tick=");
    out.push_str(&format!("{}", crate::interrupts::ticks()));
    out.push('\n');
    for detail in details {
        out.push_str(detail);
        out.push('\n');
    }
    let _ = safe_write(PACKAGE_TXN_PATH, out.as_bytes());
}

fn safe_write(path: &str, data: &[u8]) -> Result<(), &'static str> {
    match crate::vfs::vfs_kernel_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return Err("create failed"),
    }
    crate::vfs::vfs_kernel_safe_write_file(path, data).map_err(|_| "write failed")
}

fn safe_write_mode(path: &str, data: &[u8], mode: u16) -> Result<(), &'static str> {
    crate::vfs::vfs_kernel_safe_write_file_with_mode(path, data, mode).map_err(|_| "write failed")
}

fn safe_write_metadata(
    path: &str,
    data: &[u8],
    uid: u32,
    gid: u32,
    mode: u16,
) -> Result<(), &'static str> {
    crate::vfs::vfs_kernel_safe_write_file_with_metadata(path, data, uid, gid, mode)
        .map_err(|_| "write failed")
}

fn find_app(id_or_command: &str) -> Option<&'static crate::app_metadata::AppMetadata> {
    crate::app_metadata::APPS.iter().find(|app| {
        app.id.eq_ignore_ascii_case(id_or_command)
            || app.command.eq_ignore_ascii_case(id_or_command)
            || app.name.eq_ignore_ascii_case(id_or_command)
    })
}

fn app_dir(command: &str) -> String {
    let mut path = String::from("/APPS/");
    path.push_str(command);
    path
}

fn app_manifest_path(command: &str) -> String {
    let mut path = app_dir(command);
    path.push_str("/APP.CFG");
    path
}

fn owner_path(command: &str) -> String {
    let mut path = app_dir(command);
    path.push_str("/OWNER.TXT");
    path
}

fn signature_path(path: &str) -> String {
    let mut out = String::from(path);
    out.push_str(".sig");
    out
}

fn is_archive_path(value: &str) -> bool {
    value.starts_with('/') || value.to_ascii_uppercase().ends_with(".PKG")
}

fn manifest_for(app: &crate::app_metadata::AppMetadata) -> String {
    let mut associations = String::new();
    for (idx, assoc) in app.associations.iter().enumerate() {
        if idx > 0 {
            associations.push(',');
        }
        associations.push_str(assoc);
    }
    format!(
        "id={}\nname={}\ncommand={}\nversion=builtin\nicon={}\ncategory={}\npermission={}\nexec={}\naliases={}\nassociations={}\n",
        app.id,
        app.name,
        app.command,
        app.glyph,
        app.category.label(),
        app.permission,
        exec_for_app(app),
        aliases_for_app(app),
        associations
    )
}

fn builtin_manifest(app: &crate::app_metadata::AppMetadata) -> crate::app_metadata::AppManifest {
    crate::app_metadata::AppManifest {
        id: String::from(app.id),
        name: String::from(app.name),
        command: String::from(app.command),
        version: String::from("builtin"),
        icon: String::from(app.glyph),
        category: String::from(app.category.label()),
        permission: String::from(app.permission),
        exec_path: exec_for_app(app),
        aliases: app
            .aliases
            .iter()
            .map(|alias| String::from(*alias))
            .collect(),
        associations: app
            .associations
            .iter()
            .map(|assoc| String::from(*assoc))
            .collect(),
    }
}

fn exec_for_app(app: &crate::app_metadata::AppMetadata) -> String {
    match app.command {
        "editor" | "notes" | "trash" | "screenshot" | "guidemo" | "procdemo" => {
            let mut path = String::from("/bin/");
            path.push_str(app.command);
            path
        }
        _ => {
            let mut path = String::from("internal:");
            path.push_str(app.command);
            path
        }
    }
}

fn aliases_for_app(app: &crate::app_metadata::AppMetadata) -> String {
    let mut aliases = String::new();
    for (idx, alias) in app.aliases.iter().enumerate() {
        if idx > 0 {
            aliases.push(',');
        }
        aliases.push_str(alias);
    }
    aliases
}

fn delete_app_dir(command: &str) -> Result<(), crate::fat32::FsError> {
    let dir = app_dir(command);
    delete_tree(&dir)
}

fn delete_tree(path: &str) -> Result<(), crate::fat32::FsError> {
    if let Some(entries) = crate::vfs::vfs_kernel_list_dir(path) {
        for entry in entries {
            let mut child = String::from(path);
            if !child.ends_with('/') {
                child.push('/');
            }
            child.push_str(&entry.name);
            delete_tree(&child)?;
        }
    }
    crate::vfs::vfs_kernel_delete(path)
}

fn manifest_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(key) {
            let value = v.trim();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn manifest_values<'a>(text: &'a str, key: &str) -> Vec<&'a str> {
    let mut values = Vec::new();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(key) {
            let value = v.trim();
            if !value.is_empty() {
                values.push(value);
            }
        }
    }
    values
}

fn token_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for token in text.split_whitespace() {
        if let Some(value) = token.strip_prefix(key) {
            return Some(value);
        }
    }
    None
}

fn split_csv(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let item = part.trim();
        if !item.is_empty() {
            out.push(String::from(item));
        }
    }
    out
}

fn payload_summary_lines(payloads: &[PackagePayload]) -> String {
    if payloads.is_empty() {
        return String::from("payload=none");
    }
    let mut out = String::new();
    for (idx, payload) in payloads.iter().enumerate() {
        if idx > 0 {
            out.push_str(";");
        }
        out.push_str("payload=");
        out.push_str(&payload.target);
        out.push_str("|source=");
        out.push_str(&payload.source);
        out.push_str("|sha256=");
        out.push_str(&payload.sha256);
        out.push_str("|mode=");
        out.push_str(&crate::security::format_mode(payload.mode));
    }
    out
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) || (b'A'..=b'F').contains(&byte)
        })
}

fn valid_payload_target(path: &str) -> bool {
    if !path.starts_with('/') || path.contains("..") {
        return false;
    }
    path.starts_with("/bin/")
        || path.starts_with("/Documents/")
        || path.starts_with("/Pictures/")
        || path.starts_with("/Desktop/")
        || path.starts_with("/Downloads/")
        || path.starts_with("/FONTS/")
        || path.starts_with("/SDK/")
}

fn join_csv(values: &[String]) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    let mut out = String::new();
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(value);
    }
    out
}

fn join_csv_field(values: &[String]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(value);
    }
    out
}

fn parse_u64(value: &str) -> Option<u64> {
    let mut out = 0u64;
    if value.is_empty() {
        return None;
    }
    for byte in value.bytes() {
        if !byte.is_ascii_digit() {
            return None;
        }
        out = out.saturating_mul(10).saturating_add((byte - b'0') as u64);
    }
    Some(out)
}

fn version_rank(version: &str) -> u64 {
    let mut rank = 0u64;
    let mut parts = 0usize;
    for part in version.split('.') {
        if parts >= 4 {
            break;
        }
        let value = parse_u64(part).unwrap_or(0).min(999);
        rank = rank.saturating_mul(1000).saturating_add(value);
        parts += 1;
    }
    while parts < 4 {
        rank = rank.saturating_mul(1000);
        parts += 1;
    }
    rank
}

fn safe_token(value: &str, allow_dot: bool) -> bool {
    !value.is_empty()
        && !value.contains('/')
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || byte == b'-'
                || byte == b'_'
                || (allow_dot && byte == b'.')
        })
}
