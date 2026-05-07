extern crate alloc;

use alloc::{format, string::String, vec::Vec};

pub const UPDATE_DIR: &str = "/UPDATES";
pub const STAGED_DIR: &str = "/UPDATES/STAGED";
pub const SNAPSHOT_DIR: &str = "/UPDATES/SNAPSHOTS";
pub const LAST_SNAPSHOT_DIR: &str = "/UPDATES/SNAPSHOTS/LAST";
pub const STAGED_MANIFEST: &str = "/UPDATES/STAGED/UPDATE.MF";
pub const STAGED_SIGNATURE: &str = "/UPDATES/STAGED/UPDATE.SIG";
pub const STAGED_PAYLOAD: &str = "/UPDATES/STAGED/PAYLOAD.TXT";
pub const SNAPSHOT_MANIFEST: &str = "/UPDATES/SNAPSHOTS/LAST/MANIFEST.TXT";
pub const APPLIED_MANIFEST: &str = "/UPDATES/APPLIED.MF";
pub const LOG_PATH: &str = "/LOGS/UPDATE.TXT";
pub const TRUST_KEYS_PATH: &str = "/CONFIG/UPDATE-KEYS.TXT";

const CURRENT_OS_VERSION: u64 = 68;
const TRUST_KEY_ID: &str = "phase68-root-a";
const TRUST_ALGORITHM: &str = "ed25519";
const MAX_UPDATE_VERSION: u64 = 9999;

const ROOT_A_SEED: [u8; 32] = [
    0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11, 0x4e, 0x0f,
    0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed, 0x4f, 0xb8, 0xa6, 0xfb,
];
const ROOT_A_PUBLIC: [u8; 32] = [
    0x3d, 0x40, 0x17, 0xc3, 0xe8, 0x43, 0x89, 0x5a, 0x92, 0xb7, 0x0a, 0xa7, 0x4d, 0x1b, 0x7e, 0xbc,
    0x9c, 0x98, 0x2c, 0xcf, 0x2e, 0xc4, 0x96, 0x8c, 0xc0, 0xcd, 0x55, 0xf1, 0x2a, 0xf4, 0x66, 0x0c,
];
const ROOT_B_SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];
const ROOT_B_PUBLIC: [u8; 32] = [
    0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07, 0x3a,
    0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07, 0x51, 0x1a,
];
const REVOKED_SEED: [u8; 32] = [
    0xc5, 0xaa, 0x8d, 0xf4, 0x3f, 0x9f, 0x83, 0x7b, 0xed, 0xb7, 0x44, 0x2f, 0x31, 0xdc, 0xb7, 0xb1,
    0x66, 0xd3, 0x85, 0x35, 0x07, 0x6f, 0x09, 0x4b, 0x85, 0xce, 0x3a, 0x2e, 0x0b, 0x44, 0x58, 0xf7,
];
const REVOKED_PUBLIC: [u8; 32] = [
    0xfc, 0x51, 0xcd, 0x8e, 0x62, 0x18, 0xa1, 0xa3, 0x8d, 0xa4, 0x7e, 0xd0, 0x02, 0x30, 0xf0, 0x58,
    0x08, 0x16, 0xed, 0x13, 0xba, 0x33, 0x03, 0xac, 0x5d, 0xeb, 0x91, 0x15, 0x48, 0x90, 0x80, 0x25,
];
const EXPIRED_SEED: [u8; 32] = [
    0xf5, 0xe5, 0x76, 0x7c, 0xf1, 0x53, 0x31, 0x95, 0x17, 0x63, 0x0f, 0x22, 0x68, 0x76, 0xb8, 0x6c,
    0x81, 0x60, 0xcc, 0x58, 0x3b, 0xc0, 0x13, 0x74, 0x4c, 0x6b, 0xf2, 0x55, 0xf5, 0xcc, 0x0e, 0xe5,
];
const EXPIRED_PUBLIC: [u8; 32] = [
    0x27, 0x81, 0x17, 0xfc, 0x14, 0x4c, 0x72, 0x34, 0x0f, 0x67, 0xd0, 0xf2, 0x31, 0x6e, 0x83, 0x86,
    0xce, 0xff, 0xbf, 0x2b, 0x24, 0x28, 0xc9, 0xc5, 0x1f, 0xef, 0x7c, 0x59, 0x7f, 0x1d, 0x42, 0x6e,
];

#[derive(Clone)]
struct UpdateFile {
    target: String,
    source: String,
    sha256: Option<String>,
}

#[derive(Clone)]
struct UpdatePlan {
    id: String,
    version: u64,
    min_os_version: u64,
    target_os_version: u64,
    services: Vec<String>,
    files: Vec<UpdateFile>,
}

#[derive(Clone)]
struct SnapshotFile {
    target: String,
    snapshot: String,
    missing: bool,
}

#[derive(Clone)]
struct SnapshotPlan {
    id: String,
    version: String,
    services: Vec<String>,
    files: Vec<SnapshotFile>,
}

struct UpdateSignature {
    key: String,
    algorithm: String,
    manifest_sha256: String,
    issued_version: u64,
    signature: String,
}

struct Verification {
    key: String,
    algorithm: String,
    version: u64,
    files: usize,
    manifest_sha256: String,
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

pub fn init() {
    ensure_layout();
}

pub fn stage_text(target: &str, text: &str) -> Result<(), &'static str> {
    let version = next_update_version();
    stage_text_with_version(target, version, text)
}

pub fn stage_text_with_version(target: &str, version: u64, text: &str) -> Result<(), &'static str> {
    validate_target(target)?;
    if version == 0 {
        return Err("update version invalid");
    }
    ensure_layout();
    safe_write(STAGED_PAYLOAD, text.as_bytes())?;
    let mut manifest = String::from("id=manual\nversion=");
    manifest.push_str(&format!("{}", version));
    manifest.push_str("\nmin_os_version=");
    manifest.push_str(&format!("{}", CURRENT_OS_VERSION));
    manifest.push_str("\ntarget_os_version=");
    manifest.push_str(&format!("{}", CURRENT_OS_VERSION));
    manifest.push_str("\nservices=search-index,package-db\n");
    let payload_hash = crate::update_crypto::digest_hex(text.as_bytes());
    manifest.push_str("file target=");
    manifest.push_str(target);
    manifest.push_str(" source=");
    manifest.push_str(STAGED_PAYLOAD);
    manifest.push_str(" sha256=");
    manifest.push_str(&payload_hash);
    manifest.push('\n');
    safe_write(STAGED_MANIFEST, manifest.as_bytes())?;
    let plan = parse_update_plan(&manifest)?;
    write_signature(manifest.as_bytes(), &plan, TRUST_KEY_ID)?;
    append_log(
        "stage",
        &[format!(
            "target={} version={} bytes={} sha256={} signed_by={}",
            target,
            version,
            text.len(),
            payload_hash,
            TRUST_KEY_ID
        )],
    );
    crate::event_bus::emit("updates", "stage", target);
    Ok(())
}

pub fn apply() -> Result<(), &'static str> {
    ensure_layout();
    let (manifest, plan) = staged_plan_with_manifest()?;
    let verification = match verify_plan(&plan, &manifest, true) {
        Ok(verification) => verification,
        Err(err) => {
            append_log("verify-failed", &[format!("error={}", err)]);
            return Err(err);
        }
    };
    append_log(
        "verify-ok",
        &[format!(
            "key={} algorithm={} version={} files={} manifest_sha256={}",
            verification.key,
            verification.algorithm,
            verification.version,
            verification.files,
            verification.manifest_sha256
        )],
    );
    let snapshot = create_snapshot(&plan)?;
    append_log(
        "apply-start",
        &[format!(
            "id={} version={} files={} services={}",
            plan.id,
            plan.version,
            plan.files.len(),
            join_csv(&plan.services)
        )],
    );
    stop_services(&plan.services);
    let result = apply_files(&plan);
    if let Err(err) = result {
        let _ = restore_snapshot(&snapshot);
        start_services(&plan.services);
        append_log(
            "apply-failed",
            &[format!("error={} rollback=attempted", err)],
        );
        return Err(err);
    }
    safe_write(
        APPLIED_MANIFEST,
        applied_manifest(&plan, &verification).as_bytes(),
    )?;
    start_services(&plan.services);
    append_log(
        "apply-ok",
        &[format!("id={} files={}", plan.id, plan.files.len())],
    );
    crate::boot_health::mark_update_pending(&plan.id);
    crate::event_bus::emit("updates", "apply", &plan.id);
    let _ = crate::writeback::barrier();
    Ok(())
}

pub fn rollback() -> Result<(), &'static str> {
    ensure_layout();
    let snapshot = snapshot_plan()?;
    append_log(
        "rollback-start",
        &[format!(
            "id={} files={} services={}",
            snapshot.id,
            snapshot.files.len(),
            join_csv(&snapshot.services)
        )],
    );
    stop_services(&snapshot.services);
    let result = restore_snapshot(&snapshot);
    start_services(&snapshot.services);
    match result {
        Ok(()) => {
            append_log(
                "rollback-ok",
                &[format!("id={} files={}", snapshot.id, snapshot.files.len())],
            );
            crate::event_bus::emit("updates", "rollback", &snapshot.id);
            let _ = crate::writeback::barrier();
            Ok(())
        }
        Err(err) => {
            append_log("rollback-failed", &[format!("error={}", err)]);
            Err(err)
        }
    }
}

pub fn sign_staged() -> Result<(), &'static str> {
    sign_staged_as(TRUST_KEY_ID)
}

pub fn sign_staged_as(key_id: &str) -> Result<(), &'static str> {
    ensure_layout();
    let manifest = staged_manifest_bytes()?;
    let plan = parse_update_plan(
        core::str::from_utf8(&manifest).map_err(|_| "staged manifest is not UTF-8")?,
    )?;
    for file in &plan.files {
        if file.sha256.is_none() {
            return Err("payload hash missing");
        }
    }
    write_signature(&manifest, &plan, key_id)?;
    append_log(
        "sign",
        &[format!(
            "key={} version={} manifest={}",
            key_id, plan.version, STAGED_MANIFEST
        )],
    );
    Ok(())
}

pub fn corrupt_staged_payload(text: &str) -> Result<(), &'static str> {
    ensure_layout();
    if crate::vfs::vfs_kernel_read_file(STAGED_MANIFEST).is_none() {
        return Err("no staged update");
    }
    safe_write(STAGED_PAYLOAD, text.as_bytes())?;
    append_log(
        "corrupt-payload",
        &[format!("bytes={} signature=preserved", text.len())],
    );
    Ok(())
}

pub fn remove_staged_signature() -> Result<(), &'static str> {
    ensure_layout();
    match crate::vfs::vfs_kernel_delete(STAGED_SIGNATURE) {
        Ok(()) => {
            append_log("unsign", &[format!("signature={}", STAGED_SIGNATURE)]);
            Ok(())
        }
        Err(crate::fat32::FsError::NotFound) => Err("staged update is unsigned"),
        Err(_) => Err("signature delete failed"),
    }
}

pub fn verify_lines() -> Vec<String> {
    match staged_plan_with_manifest()
        .and_then(|(manifest, plan)| verify_plan(&plan, &manifest, true))
    {
        Ok(verification) => alloc::vec![
            format!(
                "trust=ok key={} algorithm={} version={} files={}",
                verification.key, verification.algorithm, verification.version, verification.files
            ),
            format!("manifest_sha256={}", verification.manifest_sha256),
            format!("signature={}", STAGED_SIGNATURE),
        ],
        Err(err) => alloc::vec![format!("trust=failed error={}", err)],
    }
}

pub fn trust_key_lines() -> Vec<String> {
    ensure_layout();
    let mut lines = alloc::vec![
        format!(
            "keys={} built_in=4 signature_required=yes rotation=yes anti_rollback=yes",
            TRUST_KEYS_PATH
        ),
        format!("signature={}", STAGED_SIGNATURE),
    ];
    for key in trusted_keys() {
        lines.push(format!(
            "key={} algorithm={} status={} scope=staged-updates not_before={} not_after={} generation={} public={}",
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

pub fn status_lines() -> Vec<String> {
    ensure_layout();
    let mut lines = alloc::vec![
        format!(
            "layout={} staged={} snapshots={}",
            UPDATE_DIR, STAGED_DIR, SNAPSHOT_DIR
        ),
        format!("journal={}", LOG_PATH),
    ];
    match staged_plan_with_manifest() {
        Ok((manifest, plan)) => {
            lines.push(format!(
                "staged=yes id={} version={} files={} services={}",
                plan.id,
                plan.version,
                plan.files.len(),
                join_csv(&plan.services)
            ));
            match verify_plan(&plan, &manifest, false) {
                Ok(verification) => lines.push(format!(
                    "trust=ok key={} algorithm={} version={} files={} manifest_sha256={}",
                    verification.key,
                    verification.algorithm,
                    verification.version,
                    verification.files,
                    verification.manifest_sha256
                )),
                Err(err) => lines.push(format!("trust=failed error={}", err)),
            }
        }
        Err(_) => lines.push(String::from("staged=no")),
    }
    match snapshot_plan() {
        Ok(snapshot) => lines.push(format!(
            "snapshot=yes id={} version={} files={} services={}",
            snapshot.id,
            snapshot.version,
            snapshot.files.len(),
            join_csv(&snapshot.services)
        )),
        Err(_) => lines.push(String::from("snapshot=no")),
    }
    if crate::vfs::vfs_kernel_read_file(APPLIED_MANIFEST).is_some() {
        lines.push(format!("applied={}", APPLIED_MANIFEST));
    } else {
        lines.push(String::from("applied=none"));
    }
    lines
}

pub fn recovery_lines() -> Vec<String> {
    let staged = if crate::vfs::vfs_kernel_read_file(STAGED_MANIFEST).is_some() {
        "yes"
    } else {
        "no"
    };
    let snapshot = if crate::vfs::vfs_kernel_read_file(SNAPSHOT_MANIFEST).is_some() {
        "yes"
    } else {
        "no"
    };
    let trust = if staged == "yes" {
        match staged_plan_with_manifest()
            .and_then(|(manifest, plan)| verify_plan(&plan, &manifest, false))
        {
            Ok(verification) => format!(
                "update_trust=ok key={} algorithm={} version={} files={}",
                verification.key, verification.algorithm, verification.version, verification.files
            ),
            Err(err) => format!("update_trust=failed error={}", err),
        }
    } else {
        String::from("update_trust=none reason=no-staged-update")
    };
    alloc::vec![
        format!(
            "updates staged={} snapshot={} journal={}",
            staged, snapshot, LOG_PATH
        ),
        trust,
        format!(
            "rollback_command=recovery rollback manifest={}",
            SNAPSHOT_MANIFEST
        ),
    ]
}

pub fn history_lines() -> Vec<String> {
    let Some(data) = crate::vfs::vfs_kernel_read_file(LOG_PATH) else {
        return alloc::vec![format!("history={} missing", LOG_PATH)];
    };
    let Ok(text) = core::str::from_utf8(&data) else {
        return alloc::vec![format!("history={} unreadable", LOG_PATH)];
    };
    let mut lines = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            lines.push(String::from(line));
        }
    }
    if lines.is_empty() {
        lines.push(format!("history={} empty", LOG_PATH));
    }
    lines
}

fn staged_plan_with_manifest() -> Result<(Vec<u8>, UpdatePlan), &'static str> {
    let bytes = staged_manifest_bytes()?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "staged manifest is not UTF-8")?;
    let plan = parse_update_plan(text)?;
    Ok((bytes, plan))
}

fn staged_manifest_bytes() -> Result<Vec<u8>, &'static str> {
    crate::vfs::vfs_kernel_read_file(STAGED_MANIFEST).ok_or("no staged update")
}

fn snapshot_plan() -> Result<SnapshotPlan, &'static str> {
    let bytes =
        crate::vfs::vfs_kernel_read_file(SNAPSHOT_MANIFEST).ok_or("no rollback snapshot")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "snapshot manifest is not UTF-8")?;
    parse_snapshot_plan(text)
}

fn parse_update_plan(text: &str) -> Result<UpdatePlan, &'static str> {
    let mut plan = UpdatePlan {
        id: String::from("staged"),
        version: 1,
        min_os_version: CURRENT_OS_VERSION,
        target_os_version: CURRENT_OS_VERSION,
        services: Vec::new(),
        files: Vec::new(),
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("id=") {
            plan.id = String::from(value);
        } else if let Some(value) = line.strip_prefix("version=") {
            plan.version = parse_u64(value).ok_or("update version invalid")?;
        } else if let Some(value) = line.strip_prefix("min_os_version=") {
            plan.min_os_version = parse_u64(value).ok_or("update minimum OS invalid")?;
        } else if let Some(value) = line.strip_prefix("target_os_version=") {
            plan.target_os_version = parse_u64(value).ok_or("update target OS invalid")?;
        } else if let Some(value) = line.strip_prefix("services=") {
            plan.services = split_csv(value);
        } else if let Some(rest) = line.strip_prefix("file ") {
            let target = token_value(rest, "target=").ok_or("update file missing target")?;
            let source = token_value(rest, "source=").ok_or("update file missing source")?;
            let sha256 = token_value(rest, "sha256=").map(String::from);
            validate_target(target)?;
            plan.files.push(UpdateFile {
                target: String::from(target),
                source: String::from(source),
                sha256,
            });
        }
    }
    if plan.files.is_empty() {
        return Err("staged update has no files");
    }
    if plan.version == 0 {
        return Err("update version invalid");
    }
    if plan.services.is_empty() {
        plan.services.push(String::from("search-index"));
    }
    Ok(plan)
}

fn parse_snapshot_plan(text: &str) -> Result<SnapshotPlan, &'static str> {
    let mut plan = SnapshotPlan {
        id: String::from("snapshot"),
        version: String::from("1"),
        services: Vec::new(),
        files: Vec::new(),
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("id=") {
            plan.id = String::from(value);
        } else if let Some(value) = line.strip_prefix("version=") {
            plan.version = String::from(value);
        } else if let Some(value) = line.strip_prefix("services=") {
            plan.services = split_csv(value);
        } else if let Some(rest) = line.strip_prefix("file ") {
            let target = token_value(rest, "target=").ok_or("snapshot file missing target")?;
            let snapshot = token_value(rest, "snapshot=").ok_or("snapshot file missing path")?;
            let missing = token_value(rest, "missing=")
                .map(|value| value == "true" || value == "1")
                .unwrap_or(false);
            validate_target(target)?;
            plan.files.push(SnapshotFile {
                target: String::from(target),
                snapshot: String::from(snapshot),
                missing,
            });
        }
    }
    if plan.files.is_empty() {
        return Err("rollback snapshot has no files");
    }
    Ok(plan)
}

fn create_snapshot(plan: &UpdatePlan) -> Result<SnapshotPlan, &'static str> {
    ensure_layout();
    let mut snapshot = SnapshotPlan {
        id: plan.id.clone(),
        version: format!("{}", plan.version),
        services: plan.services.clone(),
        files: Vec::new(),
    };
    let mut manifest = String::new();
    manifest.push_str("id=");
    manifest.push_str(&plan.id);
    manifest.push_str("\nversion=");
    manifest.push_str(&format!("{}", plan.version));
    manifest.push_str("\nservices=");
    manifest.push_str(&join_csv(&plan.services));
    manifest.push('\n');
    for file in &plan.files {
        let snapshot_path = format!("{}/{}.SNAP", LAST_SNAPSHOT_DIR, snapshot_name(&file.target));
        let missing = match crate::vfs::vfs_kernel_read_file(&file.target) {
            Some(data) => {
                safe_write(&snapshot_path, &data)?;
                false
            }
            None => {
                let _ = crate::vfs::vfs_kernel_delete(&snapshot_path);
                true
            }
        };
        manifest.push_str("file target=");
        manifest.push_str(&file.target);
        manifest.push_str(" snapshot=");
        manifest.push_str(&snapshot_path);
        manifest.push_str(" missing=");
        manifest.push_str(if missing { "true" } else { "false" });
        manifest.push('\n');
        snapshot.files.push(SnapshotFile {
            target: file.target.clone(),
            snapshot: snapshot_path,
            missing,
        });
    }
    safe_write(SNAPSHOT_MANIFEST, manifest.as_bytes())?;
    Ok(snapshot)
}

fn apply_files(plan: &UpdatePlan) -> Result<(), &'static str> {
    for file in &plan.files {
        let data = crate::vfs::vfs_kernel_read_file(&file.source).ok_or("payload missing")?;
        safe_write(&file.target, &data)?;
    }
    Ok(())
}

fn verify_plan(
    plan: &UpdatePlan,
    manifest: &[u8],
    enforce_rollback: bool,
) -> Result<Verification, &'static str> {
    let signature = staged_signature()?;
    if signature.algorithm != TRUST_ALGORITHM {
        return Err("unsupported update signature");
    }
    if signature.issued_version != plan.version {
        return Err("signature version mismatch");
    }
    if plan.min_os_version > CURRENT_OS_VERSION {
        return Err("update requires newer OS");
    }
    if plan.target_os_version > CURRENT_OS_VERSION {
        return Err("update target OS unsupported");
    }
    if enforce_rollback && plan.version <= applied_version() {
        return Err("update version rollback");
    }

    let key = find_trusted_key(&signature.key).ok_or("update key not trusted")?;
    if key.algorithm != signature.algorithm {
        return Err("unsupported update signature");
    }
    if key.status == "revoked" {
        return Err("update key revoked");
    }
    if key.status != "trusted" {
        return Err("update key not trusted");
    }
    if plan.version < key.not_before {
        return Err("update key not yet valid");
    }
    if plan.version > key.not_after {
        return Err("update key expired");
    }

    let manifest_digest = crate::update_crypto::sha256(manifest);
    if !crate::update_crypto::hex_matches_digest(&signature.manifest_sha256, &manifest_digest) {
        return Err("manifest hash mismatch");
    }
    let signature_bytes =
        crate::update_crypto::hex_to_64(&signature.signature).ok_or("signature invalid")?;
    if !crate::update_crypto::ed25519_verify(&key.public_key, manifest, &signature_bytes) {
        return Err("signature invalid");
    }

    for file in &plan.files {
        let expected_hash = file.sha256.as_deref().ok_or("payload hash missing")?;
        let data = crate::vfs::vfs_kernel_read_file(&file.source).ok_or("payload missing")?;
        let actual_hash = crate::update_crypto::sha256(&data);
        if !crate::update_crypto::hex_matches_digest(expected_hash, &actual_hash) {
            return Err("payload hash mismatch");
        }
    }

    Ok(Verification {
        key: signature.key,
        algorithm: signature.algorithm,
        version: plan.version,
        files: plan.files.len(),
        manifest_sha256: crate::update_crypto::hex(&manifest_digest),
    })
}

fn staged_signature() -> Result<UpdateSignature, &'static str> {
    let bytes =
        crate::vfs::vfs_kernel_read_file(STAGED_SIGNATURE).ok_or("staged update is unsigned")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "signature is not UTF-8")?;
    let mut signature = UpdateSignature {
        key: String::new(),
        algorithm: String::new(),
        manifest_sha256: String::new(),
        issued_version: 0,
        signature: String::new(),
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line == "coolOS update signature" {
            continue;
        }
        if let Some(value) = line.strip_prefix("key=") {
            signature.key = String::from(value);
        } else if let Some(value) = line.strip_prefix("algorithm=") {
            signature.algorithm = String::from(value);
        } else if let Some(value) = line.strip_prefix("manifest_sha256=") {
            signature.manifest_sha256 = String::from(value);
        } else if let Some(value) = line.strip_prefix("issued_version=") {
            signature.issued_version = parse_u64(value).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("signature=") {
            signature.signature = String::from(value);
        }
    }
    if signature.key.is_empty()
        || signature.algorithm.is_empty()
        || signature.manifest_sha256.is_empty()
        || signature.issued_version == 0
        || signature.signature.is_empty()
    {
        return Err("signature incomplete");
    }
    Ok(signature)
}

fn write_signature(manifest: &[u8], plan: &UpdatePlan, key_id: &str) -> Result<(), &'static str> {
    let seed = signing_seed(key_id).ok_or("signing key unavailable")?;
    let manifest_sha256 = crate::update_crypto::digest_hex(manifest);
    let signature = crate::update_crypto::hex(&crate::update_crypto::ed25519_sign(seed, manifest));
    let mut out = String::from("coolOS update signature\n");
    out.push_str("key=");
    out.push_str(key_id);
    out.push_str("\nalgorithm=");
    out.push_str(TRUST_ALGORITHM);
    out.push_str("\nmanifest_sha256=");
    out.push_str(&manifest_sha256);
    out.push_str("\nissued_version=");
    out.push_str(&format!("{}", plan.version));
    out.push_str("\nsignature=");
    out.push_str(&signature);
    out.push('\n');
    safe_write(STAGED_SIGNATURE, out.as_bytes())
}

fn restore_snapshot(snapshot: &SnapshotPlan) -> Result<(), &'static str> {
    for file in &snapshot.files {
        if file.missing {
            let _ = crate::vfs::vfs_kernel_delete(&file.target);
            continue;
        }
        let data = crate::vfs::vfs_kernel_read_file(&file.snapshot).ok_or("snapshot missing")?;
        safe_write(&file.target, &data)?;
    }
    Ok(())
}

fn applied_manifest(plan: &UpdatePlan, verification: &Verification) -> String {
    let mut out = String::new();
    out.push_str("id=");
    out.push_str(&plan.id);
    out.push_str("\nversion=");
    out.push_str(&format!("{}", plan.version));
    out.push_str("\nmin_os_version=");
    out.push_str(&format!("{}", plan.min_os_version));
    out.push_str("\ntarget_os_version=");
    out.push_str(&format!("{}", plan.target_os_version));
    out.push_str("\ntick=");
    out.push_str(&format!("{}", crate::interrupts::ticks()));
    out.push_str("\nfiles=");
    out.push_str(&format!("{}", plan.files.len()));
    out.push_str("\nverified_by=");
    out.push_str(&verification.key);
    out.push_str("\nalgorithm=");
    out.push_str(&verification.algorithm);
    out.push_str("\nmanifest_sha256=");
    out.push_str(&verification.manifest_sha256);
    out.push('\n');
    out
}

fn append_log(action: &str, details: &[String]) {
    ensure_layout();
    let mut log = match crate::vfs::vfs_kernel_read_file(LOG_PATH) {
        Some(bytes) => core::str::from_utf8(&bytes)
            .map(String::from)
            .unwrap_or_else(|_| String::from("coolOS update journal\n")),
        None => String::from("coolOS update journal\n"),
    };
    if log.len() > 8192 {
        log = String::from("coolOS update journal\n");
        log.push_str("trimmed=true\n");
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
    let _ = safe_write(LOG_PATH, log.as_bytes());
}

fn stop_services(services: &[String]) {
    for service in services {
        let _ = crate::services::stop(service);
    }
}

fn start_services(services: &[String]) {
    for service in services {
        let _ = crate::services::start(service);
    }
}

fn next_update_version() -> u64 {
    applied_version().saturating_add(1).max(1)
}

fn applied_version() -> u64 {
    let Some(bytes) = crate::vfs::vfs_kernel_read_file(APPLIED_MANIFEST) else {
        return 0;
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return 0;
    };
    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("version=") {
            return parse_u64(value).unwrap_or(0);
        }
    }
    0
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
                .unwrap_or(MAX_UPDATE_VERSION),
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
            id: String::from("phase68-root-a"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: ROOT_A_PUBLIC,
            not_before: 1,
            not_after: MAX_UPDATE_VERSION,
            generation: 1,
        },
        TrustedKey {
            id: String::from("phase68-root-b"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: ROOT_B_PUBLIC,
            not_before: 1,
            not_after: MAX_UPDATE_VERSION,
            generation: 2,
        },
        TrustedKey {
            id: String::from("phase68-revoked"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("revoked"),
            public_key: REVOKED_PUBLIC,
            not_before: 1,
            not_after: MAX_UPDATE_VERSION,
            generation: 0,
        },
        TrustedKey {
            id: String::from("phase68-expired"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: EXPIRED_PUBLIC,
            not_before: 1,
            not_after: 1,
            generation: 0,
        },
    ]
}

fn signing_seed(key_id: &str) -> Option<&'static [u8; 32]> {
    match key_id {
        "phase68-root-a" => Some(&ROOT_A_SEED),
        "phase68-root-b" => Some(&ROOT_B_SEED),
        "phase68-revoked" => Some(&REVOKED_SEED),
        "phase68-expired" => Some(&EXPIRED_SEED),
        "phase68-unknown" => Some(&ROOT_B_SEED),
        _ => None,
    }
}

fn ensure_layout() {
    let _ = crate::vfs::vfs_kernel_create_dir(UPDATE_DIR);
    let _ = crate::vfs::vfs_kernel_create_dir(STAGED_DIR);
    let _ = crate::vfs::vfs_kernel_create_dir(SNAPSHOT_DIR);
    let _ = crate::vfs::vfs_kernel_create_dir(LAST_SNAPSHOT_DIR);
    let _ = crate::vfs::vfs_kernel_create_dir("/CONFIG");
    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
    ensure_trust_keys_file();
}

fn ensure_trust_keys_file() {
    if let Some(bytes) = crate::vfs::vfs_kernel_read_file(TRUST_KEYS_PATH) {
        if core::str::from_utf8(&bytes)
            .map(|text| text.contains("key=phase68-root-a") && text.contains("algorithm=ed25519"))
            .unwrap_or(false)
        {
            return;
        }
    }
    let mut out = String::from("coolOS update trust keys\n");
    out.push_str("# public keys only; signing seeds are not stored in this metadata file\n");
    for key in built_in_trusted_keys() {
        out.push_str("key=");
        out.push_str(&key.id);
        out.push_str(" algorithm=");
        out.push_str(&key.algorithm);
        out.push_str(" status=");
        out.push_str(&key.status);
        out.push_str(" scope=staged-updates public=");
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

fn safe_write(path: &str, data: &[u8]) -> Result<(), &'static str> {
    match crate::vfs::vfs_kernel_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return Err("create failed"),
    }
    crate::vfs::vfs_kernel_safe_write_file(path, data).map_err(|_| "write failed")
}

fn validate_target(target: &str) -> Result<(), &'static str> {
    if !target.starts_with('/') || target == "/" {
        return Err("invalid update target");
    }
    if target.starts_with(UPDATE_DIR) || target == LOG_PATH {
        return Err("refusing to update update metadata");
    }
    Ok(())
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

fn snapshot_name(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        match b {
            b'/' => out.push('_'),
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'_' => out.push(b as char),
            _ => out.push('_'),
        }
    }
    if out.is_empty() {
        String::from("_ROOT")
    } else {
        out
    }
}
