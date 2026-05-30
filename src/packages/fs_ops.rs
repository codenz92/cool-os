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
