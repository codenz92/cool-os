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

const TRUST_KEY_ID: &str = "coolos-dev";
const TRUST_ALGORITHM: &str = "hmac-sha256";
const TRUST_KEY_BYTES: &[u8] = b"coolOS phase67 built-in update trust key v1";

#[derive(Clone)]
struct UpdateFile {
    target: String,
    source: String,
    sha256: Option<String>,
}

#[derive(Clone)]
struct UpdatePlan {
    id: String,
    version: String,
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
    signature: String,
}

struct Verification {
    key: String,
    algorithm: String,
    files: usize,
    manifest_sha256: String,
}

pub fn init() {
    ensure_layout();
}

pub fn stage_text(target: &str, text: &str) -> Result<(), &'static str> {
    validate_target(target)?;
    ensure_layout();
    safe_write(STAGED_PAYLOAD, text.as_bytes())?;
    let mut manifest = String::from("id=manual\nversion=1\nservices=search-index,package-db\n");
    let payload_hash = crate::update_crypto::digest_hex(text.as_bytes());
    manifest.push_str("file target=");
    manifest.push_str(target);
    manifest.push_str(" source=");
    manifest.push_str(STAGED_PAYLOAD);
    manifest.push_str(" sha256=");
    manifest.push_str(&payload_hash);
    manifest.push('\n');
    safe_write(STAGED_MANIFEST, manifest.as_bytes())?;
    write_signature(manifest.as_bytes())?;
    append_log(
        "stage",
        &[format!(
            "target={} bytes={} sha256={} signed_by={}",
            target,
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
    let verification = match verify_plan(&plan, &manifest) {
        Ok(verification) => verification,
        Err(err) => {
            append_log("verify-failed", &[format!("error={}", err)]);
            return Err(err);
        }
    };
    append_log(
        "verify-ok",
        &[format!(
            "key={} algorithm={} files={} manifest_sha256={}",
            verification.key,
            verification.algorithm,
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
    safe_write(APPLIED_MANIFEST, applied_manifest(&plan).as_bytes())?;
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
    write_signature(&manifest)?;
    append_log(
        "sign",
        &[format!("key={} manifest={}", TRUST_KEY_ID, STAGED_MANIFEST)],
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
    match staged_plan_with_manifest().and_then(|(manifest, plan)| verify_plan(&plan, &manifest)) {
        Ok(verification) => alloc::vec![
            format!(
                "trust=ok key={} algorithm={} files={}",
                verification.key, verification.algorithm, verification.files
            ),
            format!("manifest_sha256={}", verification.manifest_sha256),
            format!("signature={}", STAGED_SIGNATURE),
        ],
        Err(err) => alloc::vec![format!("trust=failed error={}", err)],
    }
}

pub fn trust_key_lines() -> Vec<String> {
    ensure_layout();
    alloc::vec![
        format!("keys={} built_in=1 signature_required=yes", TRUST_KEYS_PATH),
        format!(
            "key={} algorithm={} status=trusted scope=staged-updates",
            TRUST_KEY_ID, TRUST_ALGORITHM
        ),
        format!("signature={}", STAGED_SIGNATURE),
    ]
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
            match verify_plan(&plan, &manifest) {
                Ok(verification) => lines.push(format!(
                    "trust=ok key={} algorithm={} files={} manifest_sha256={}",
                    verification.key,
                    verification.algorithm,
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
        match staged_plan_with_manifest().and_then(|(manifest, plan)| verify_plan(&plan, &manifest))
        {
            Ok(verification) => format!(
                "update_trust=ok key={} algorithm={} files={}",
                verification.key, verification.algorithm, verification.files
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
        version: plan.version.clone(),
        services: plan.services.clone(),
        files: Vec::new(),
    };
    let mut manifest = String::new();
    manifest.push_str("id=");
    manifest.push_str(&plan.id);
    manifest.push_str("\nversion=");
    manifest.push_str(&plan.version);
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

fn verify_plan(plan: &UpdatePlan, manifest: &[u8]) -> Result<Verification, &'static str> {
    let signature = staged_signature()?;
    if signature.key != TRUST_KEY_ID {
        return Err("update key not trusted");
    }
    if signature.algorithm != TRUST_ALGORITHM {
        return Err("unsupported update signature");
    }

    let manifest_digest = crate::update_crypto::sha256(manifest);
    if !crate::update_crypto::hex_matches_digest(&signature.manifest_sha256, &manifest_digest) {
        return Err("manifest hash mismatch");
    }
    let expected_signature = crate::update_crypto::hmac_sha256(TRUST_KEY_BYTES, manifest);
    if !crate::update_crypto::hex_matches_digest(&signature.signature, &expected_signature) {
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
        } else if let Some(value) = line.strip_prefix("signature=") {
            signature.signature = String::from(value);
        }
    }
    if signature.key.is_empty()
        || signature.algorithm.is_empty()
        || signature.manifest_sha256.is_empty()
        || signature.signature.is_empty()
    {
        return Err("signature incomplete");
    }
    Ok(signature)
}

fn write_signature(manifest: &[u8]) -> Result<(), &'static str> {
    let manifest_sha256 = crate::update_crypto::digest_hex(manifest);
    let signature = crate::update_crypto::hex(&crate::update_crypto::hmac_sha256(
        TRUST_KEY_BYTES,
        manifest,
    ));
    let mut out = String::from("coolOS update signature\n");
    out.push_str("key=");
    out.push_str(TRUST_KEY_ID);
    out.push_str("\nalgorithm=");
    out.push_str(TRUST_ALGORITHM);
    out.push_str("\nmanifest_sha256=");
    out.push_str(&manifest_sha256);
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

fn applied_manifest(plan: &UpdatePlan) -> String {
    let mut out = String::new();
    out.push_str("id=");
    out.push_str(&plan.id);
    out.push_str("\nversion=");
    out.push_str(&plan.version);
    out.push_str("\ntick=");
    out.push_str(&format!("{}", crate::interrupts::ticks()));
    out.push_str("\nfiles=");
    out.push_str(&format!("{}", plan.files.len()));
    out.push_str("\nverified_by=");
    out.push_str(TRUST_KEY_ID);
    out.push_str("\nalgorithm=");
    out.push_str(TRUST_ALGORITHM);
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
    if crate::vfs::vfs_kernel_read_file(TRUST_KEYS_PATH).is_some() {
        return;
    }
    let mut out = String::from("coolOS update trust keys\n");
    out.push_str("key=");
    out.push_str(TRUST_KEY_ID);
    out.push_str(" algorithm=");
    out.push_str(TRUST_ALGORITHM);
    out.push_str(" status=trusted scope=staged-updates source=built-in\n");
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
