extern crate alloc;

use alloc::{format, string::String, vec::Vec};

pub const ROOT_UID: u32 = 0;
pub const ROOT_GID: u32 = 0;
pub const USER_UID: u32 = 1000;
pub const USER_GID: u32 = 1000;

pub const DEFAULT_DIR_MODE: u16 = 0o755;
pub const DEFAULT_FILE_MODE: u16 = 0o644;
pub const DEFAULT_EXEC_MODE: u16 = 0o755;

const CAP_READ_FS: u32 = 1 << 0;
const CAP_WRITE_FS: u32 = 1 << 1;
const CAP_EXEC: u32 = 1 << 2;
const CAP_NETWORK: u32 = 1 << 3;
const CAP_DESKTOP: u32 = 1 << 4;
const CAP_SETTINGS: u32 = 1 << 5;
const CAP_DIAGNOSTICS: u32 = 1 << 6;
const CAP_SHELL: u32 = 1 << 7;

const CAP_ALL_USER: u32 = CAP_READ_FS
    | CAP_WRITE_FS
    | CAP_EXEC
    | CAP_NETWORK
    | CAP_DESKTOP
    | CAP_SETTINGS
    | CAP_DIAGNOSTICS
    | CAP_SHELL;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Credentials {
    pub uid: u32,
    pub gid: u32,
    pub caps: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Execute,
}

pub struct User {
    pub name: &'static str,
    pub role: &'static str,
    pub uid: u32,
    pub gid: u32,
}

#[allow(dead_code)]
pub struct Group {
    pub name: &'static str,
    pub gid: u32,
}

pub fn init() {
    crate::event_bus::emit(
        "security",
        "init",
        "users, file modes, and package grants active",
    );
}

pub fn current_user() -> User {
    User {
        name: "jamie",
        role: "admin",
        uid: USER_UID,
        gid: USER_GID,
    }
}

#[allow(dead_code)]
pub fn groups() -> &'static [Group] {
    &[
        Group {
            name: "root",
            gid: ROOT_GID,
        },
        Group {
            name: "users",
            gid: USER_GID,
        },
        Group {
            name: "wheel",
            gid: 10,
        },
    ]
}

#[allow(dead_code)]
pub const fn kernel_credentials() -> Credentials {
    Credentials {
        uid: ROOT_UID,
        gid: ROOT_GID,
        caps: CAP_ALL_USER,
    }
}

pub const fn interactive_credentials() -> Credentials {
    Credentials {
        uid: USER_UID,
        gid: USER_GID,
        caps: CAP_ALL_USER,
    }
}

pub fn current_credentials() -> Credentials {
    crate::scheduler::current_credentials().unwrap_or_else(interactive_credentials)
}

pub fn package_credentials(permission: &str) -> Credentials {
    Credentials {
        uid: USER_UID,
        gid: USER_GID,
        caps: caps_for_permission(permission),
    }
}

pub fn caps_for_permission(permission: &str) -> u32 {
    let mut caps = CAP_EXEC;
    for token in permission.split(',').map(str::trim) {
        match token {
            "shell" => caps |= CAP_ALL_USER,
            "filesystem" | "files" => caps |= CAP_READ_FS | CAP_WRITE_FS | CAP_DESKTOP,
            "read-files" | "read" => caps |= CAP_READ_FS | CAP_DESKTOP,
            "network" => caps |= CAP_READ_FS | CAP_NETWORK | CAP_DESKTOP,
            "desktop" => caps |= CAP_READ_FS | CAP_DESKTOP,
            "settings" => caps |= CAP_READ_FS | CAP_WRITE_FS | CAP_SETTINGS | CAP_DESKTOP,
            "diagnostics" => caps |= CAP_READ_FS | CAP_DIAGNOSTICS | CAP_DESKTOP,
            "user" | "" => caps |= CAP_READ_FS | CAP_DESKTOP,
            _ => caps |= CAP_READ_FS,
        }
    }
    caps
}

pub fn can_read_files(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_READ_FS != 0
}

pub fn can_write_files(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_WRITE_FS != 0
}

pub fn can_execute_files(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_EXEC != 0
}

pub fn can_network(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_NETWORK != 0 || creds.caps & CAP_SHELL != 0
}

pub fn can_desktop(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_DESKTOP != 0 || creds.caps & CAP_SHELL != 0
}

pub fn can_admin(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_SHELL != 0
}

pub fn can_read_metadata(creds: Credentials, uid: u32, gid: u32, mode: u16) -> bool {
    can_read_files(creds) && mode_allows(creds, uid, gid, mode, Access::Read)
}

pub fn can_write_metadata(creds: Credentials, uid: u32, gid: u32, mode: u16) -> bool {
    can_write_files(creds) && mode_allows(creds, uid, gid, mode, Access::Write)
}

pub fn can_execute_metadata(creds: Credentials, uid: u32, gid: u32, mode: u16) -> bool {
    can_execute_files(creds) && mode_allows(creds, uid, gid, mode, Access::Execute)
}

pub fn mode_allows(creds: Credentials, uid: u32, gid: u32, mode: u16, access: Access) -> bool {
    if creds.uid == ROOT_UID {
        return true;
    }
    let shift = if creds.uid == uid {
        6
    } else if creds.gid == gid {
        3
    } else {
        0
    };
    let mask = match access {
        Access::Read => 0b100,
        Access::Write => 0b010,
        Access::Execute => 0b001,
    };
    ((mode >> shift) & mask) != 0
}

#[allow(dead_code)]
pub fn is_protected_path(path: &str) -> bool {
    let upper = path.to_ascii_uppercase();
    upper == "/"
        || upper == "/CONFIG"
        || upper == "/LOGS"
        || upper == "/DEV"
        || upper == "/APPS"
        || upper.starts_with("/CONFIG/")
        || upper.starts_with("/LOGS/")
        || upper.starts_with("/DEV/")
        || upper.starts_with("/APPS/")
}

#[allow(dead_code)]
pub fn can_write_path(path: &str) -> bool {
    let creds = current_credentials();
    can_write_files(creds) && (creds.uid == ROOT_UID || !is_protected_path(path))
}

#[allow(dead_code)]
pub fn can_read_path(_path: &str) -> bool {
    can_read_files(current_credentials())
}

#[allow(dead_code)]
pub fn can_execute_path(_path: &str) -> bool {
    can_execute_files(current_credentials())
}

pub fn credentials_label(creds: Credentials) -> String {
    format!(
        "uid={} gid={} caps={}",
        creds.uid,
        creds.gid,
        capability_label(creds.caps)
    )
}

pub fn capability_label(caps: u32) -> String {
    if caps & CAP_ALL_USER == CAP_ALL_USER {
        return String::from("all");
    }
    let mut out = String::new();
    push_cap(&mut out, caps, CAP_READ_FS, "read-fs");
    push_cap(&mut out, caps, CAP_WRITE_FS, "write-fs");
    push_cap(&mut out, caps, CAP_EXEC, "exec");
    push_cap(&mut out, caps, CAP_NETWORK, "network");
    push_cap(&mut out, caps, CAP_DESKTOP, "desktop");
    push_cap(&mut out, caps, CAP_SETTINGS, "settings");
    push_cap(&mut out, caps, CAP_DIAGNOSTICS, "diagnostics");
    push_cap(&mut out, caps, CAP_SHELL, "shell");
    if out.is_empty() {
        out.push_str("none");
    }
    out
}

fn push_cap(out: &mut String, caps: u32, bit: u32, label: &str) {
    if caps & bit == 0 {
        return;
    }
    if !out.is_empty() {
        out.push(',');
    }
    out.push_str(label);
}

pub fn format_mode(mode: u16) -> String {
    let mut out = String::new();
    out.push(char::from(b'0' + ((mode >> 6) & 7) as u8));
    out.push(char::from(b'0' + ((mode >> 3) & 7) as u8));
    out.push(char::from(b'0' + (mode & 7) as u8));
    out
}

pub fn parse_mode(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 4 {
        return None;
    }
    let mut mode = 0u16;
    for byte in text.bytes() {
        if !(b'0'..=b'7').contains(&byte) {
            return None;
        }
        mode = (mode << 3) | (byte - b'0') as u16;
    }
    Some(mode & 0o777)
}

pub fn app_permission_lines() -> Vec<String> {
    let mut lines: Vec<String> = crate::app_metadata::APPS
        .iter()
        .map(|app| {
            format!(
                "{} id={} permission={} caps={} command={}",
                app.name,
                app.id,
                app.permission,
                capability_label(caps_for_permission(app.permission)),
                app.command
            )
        })
        .collect();
    for manifest in crate::app_metadata::installed_app_manifests() {
        if crate::app_metadata::is_builtin_id(&manifest.id) {
            continue;
        }
        lines.push(format!(
            "{} id={} permission={} caps={} command={} exec={}",
            manifest.name,
            manifest.id,
            manifest.permission,
            capability_label(caps_for_permission(&manifest.permission)),
            manifest.command,
            manifest.exec_path
        ));
    }
    lines
}

pub fn app_permission_for(name: &str) -> Option<String> {
    if let Some(app) = crate::app_metadata::app_by_name(name) {
        return Some(String::from(app.permission));
    }
    crate::app_metadata::installed_manifest_by_id_or_command(name)
        .map(|manifest| manifest.permission)
}

pub fn lines() -> Vec<String> {
    let user = current_user();
    let creds = current_credentials();
    let mut lines = alloc::vec![
        format!(
            "current user={} uid={} gid={} role={}",
            user.name, user.uid, user.gid, user.role
        ),
        format!("current task {}", credentials_label(creds)),
        String::from("users: root(0), jamie(1000)"),
        String::from("groups: root(0), users(1000), wheel(10)"),
        String::from("filesystem: CoolFS uid/gid/mode enforced by VFS and syscalls"),
        String::from(
            "package grants: manifest permission labels become launch-time task capabilities"
        ),
    ];
    lines.extend(app_permission_lines());
    lines
}
