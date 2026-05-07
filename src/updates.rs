extern crate alloc;

use alloc::{format, string::String, vec::Vec};

pub const UPDATE_DIR: &str = "/UPDATES";
pub const STAGED_DIR: &str = "/UPDATES/STAGED";
pub const SNAPSHOT_DIR: &str = "/UPDATES/SNAPSHOTS";
pub const LAST_SNAPSHOT_DIR: &str = "/UPDATES/SNAPSHOTS/LAST";
pub const STAGED_MANIFEST: &str = "/UPDATES/STAGED/UPDATE.MF";
pub const STAGED_PAYLOAD: &str = "/UPDATES/STAGED/PAYLOAD.TXT";
pub const SNAPSHOT_MANIFEST: &str = "/UPDATES/SNAPSHOTS/LAST/MANIFEST.TXT";
pub const APPLIED_MANIFEST: &str = "/UPDATES/APPLIED.MF";
pub const LOG_PATH: &str = "/LOGS/UPDATE.TXT";

#[derive(Clone)]
struct UpdateFile {
    target: String,
    source: String,
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

pub fn init() {
    ensure_layout();
}

pub fn stage_text(target: &str, text: &str) -> Result<(), &'static str> {
    validate_target(target)?;
    ensure_layout();
    safe_write(STAGED_PAYLOAD, text.as_bytes())?;
    let mut manifest = String::from("id=manual\nversion=1\nservices=search-index,package-db\n");
    manifest.push_str("file target=");
    manifest.push_str(target);
    manifest.push_str(" source=");
    manifest.push_str(STAGED_PAYLOAD);
    manifest.push('\n');
    safe_write(STAGED_MANIFEST, manifest.as_bytes())?;
    append_log(
        "stage",
        &[format!("target={} bytes={}", target, text.len())],
    );
    crate::event_bus::emit("updates", "stage", target);
    Ok(())
}

pub fn apply() -> Result<(), &'static str> {
    ensure_layout();
    let plan = staged_plan()?;
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

pub fn status_lines() -> Vec<String> {
    ensure_layout();
    let mut lines = alloc::vec![
        format!(
            "layout={} staged={} snapshots={}",
            UPDATE_DIR, STAGED_DIR, SNAPSHOT_DIR
        ),
        format!("journal={}", LOG_PATH),
    ];
    match staged_plan() {
        Ok(plan) => lines.push(format!(
            "staged=yes id={} version={} files={} services={}",
            plan.id,
            plan.version,
            plan.files.len(),
            join_csv(&plan.services)
        )),
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
    alloc::vec![
        format!(
            "updates staged={} snapshot={} journal={}",
            staged, snapshot, LOG_PATH
        ),
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

fn staged_plan() -> Result<UpdatePlan, &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(STAGED_MANIFEST).ok_or("no staged update")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "staged manifest is not UTF-8")?;
    parse_update_plan(text)
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
            validate_target(target)?;
            plan.files.push(UpdateFile {
                target: String::from(target),
                source: String::from(source),
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
    let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
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
