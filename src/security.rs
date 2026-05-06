extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

const USERS_PATH: &str = "/CONFIG/USERS.DB";

pub const ROOT_UID: u32 = 0;
pub const ROOT_GID: u32 = 0;
pub const USER_UID: u32 = 1000;
pub const USER_GID: u32 = 1000;
pub const GUEST_UID: u32 = 1001;
pub const SERVICE_UID: u32 = 200;
pub const SERVICE_GID: u32 = 200;

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
const CAP_ADMIN: u32 = 1 << 8;

const CAP_INTERACTIVE_USER: u32 = CAP_READ_FS | CAP_WRITE_FS | CAP_EXEC | CAP_NETWORK | CAP_DESKTOP;
const CAP_PACKAGE_SHELL: u32 = CAP_INTERACTIVE_USER | CAP_SETTINGS | CAP_DIAGNOSTICS | CAP_SHELL;
const CAP_ALL_USER: u32 = CAP_PACKAGE_SHELL | CAP_ADMIN;

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

#[derive(Clone)]
pub struct User {
    pub name: String,
    pub role: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub login_enabled: bool,
}

#[derive(Clone)]
pub struct Group {
    pub name: String,
    pub gid: u32,
}

#[derive(Clone)]
struct UserRecord {
    name: String,
    role: String,
    uid: u32,
    gid: u32,
    home: String,
    pass_hash: u32,
    login_enabled: bool,
}

#[derive(Clone)]
struct GroupRecord {
    name: String,
    gid: u32,
}

struct SecurityState {
    users: Vec<UserRecord>,
    groups: Vec<GroupRecord>,
    session_uid: u32,
    session_gid: u32,
    session_caps: u32,
    umask: u16,
}

impl SecurityState {
    const fn empty() -> Self {
        Self {
            users: Vec::new(),
            groups: Vec::new(),
            session_uid: USER_UID,
            session_gid: USER_GID,
            session_caps: CAP_ALL_USER,
            umask: 0o022,
        }
    }
}

static SECURITY: Mutex<SecurityState> = Mutex::new(SecurityState::empty());

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    NoSuchUser,
    LoginDisabled,
    BadPassword,
    PasswordTooShort,
    PermissionDenied,
    Io,
}

impl AuthError {
    pub const fn as_str(self) -> &'static str {
        match self {
            AuthError::NoSuchUser => "no such user",
            AuthError::LoginDisabled => "login disabled",
            AuthError::BadPassword => "bad password",
            AuthError::PasswordTooShort => "password too short",
            AuthError::PermissionDenied => "permission denied",
            AuthError::Io => "could not persist user database",
        }
    }
}

pub fn init() {
    let users = load_users_from_disk().unwrap_or_else(default_users);
    let groups = default_groups();
    let session = users
        .iter()
        .find(|user| user.uid == USER_UID)
        .cloned()
        .unwrap_or_else(default_admin_user);
    {
        let mut state = SECURITY.lock();
        state.users = users;
        state.groups = groups;
        state.session_uid = session.uid;
        state.session_gid = session.gid;
        state.session_caps = caps_for_role(&session.role);
        state.umask = 0o022;
    }
    let _ = persist_users();
    ensure_home_dirs();
    crate::event_bus::emit(
        "security",
        "init",
        "sessions, users, file modes, and package grants active",
    );
}

pub fn current_user() -> User {
    let state = SECURITY.lock();
    let uid = state.session_uid;
    state
        .users
        .iter()
        .find(|user| user.uid == uid)
        .map(public_user)
        .unwrap_or_else(|| public_user(&default_admin_user()))
}

pub fn users() -> Vec<User> {
    SECURITY.lock().users.iter().map(public_user).collect()
}

#[allow(dead_code)]
pub fn groups() -> Vec<Group> {
    SECURITY.lock().groups.iter().map(public_group).collect()
}

pub fn groups_for(name: &str) -> Option<Vec<Group>> {
    let state = SECURITY.lock();
    let user = state
        .users
        .iter()
        .find(|user| user.name.eq_ignore_ascii_case(name))?;
    let mut groups = Vec::new();
    for group in &state.groups {
        if group.gid == user.gid || (user.role == "admin" && group.name == "wheel") {
            groups.push(public_group(group));
        }
    }
    Some(groups)
}

pub fn user_by_name(name: &str) -> Option<User> {
    SECURITY
        .lock()
        .users
        .iter()
        .find(|user| user.name.eq_ignore_ascii_case(name))
        .map(public_user)
}

#[allow(dead_code)]
pub fn user_by_uid(uid: u32) -> Option<User> {
    SECURITY
        .lock()
        .users
        .iter()
        .find(|user| user.uid == uid)
        .map(public_user)
}

#[allow(dead_code)]
pub const fn kernel_credentials() -> Credentials {
    Credentials {
        uid: ROOT_UID,
        gid: ROOT_GID,
        caps: CAP_ALL_USER,
    }
}

pub fn interactive_credentials() -> Credentials {
    let state = SECURITY.lock();
    Credentials {
        uid: state.session_uid,
        gid: state.session_gid,
        caps: state.session_caps,
    }
}

pub fn current_credentials() -> Credentials {
    crate::scheduler::current_credentials().unwrap_or_else(interactive_credentials)
}

pub fn package_credentials(permission: &str) -> Credentials {
    let session = interactive_credentials();
    Credentials {
        uid: session.uid,
        gid: session.gid,
        caps: caps_for_permission(permission),
    }
}

pub fn service_credentials(name: &str) -> Credentials {
    let mut caps = CAP_READ_FS | CAP_WRITE_FS | CAP_EXEC | CAP_DIAGNOSTICS;
    if name == "network-stack" {
        caps |= CAP_NETWORK;
    }
    Credentials {
        uid: SERVICE_UID,
        gid: SERVICE_GID,
        caps,
    }
}

pub fn login(name: &str, password: &str) -> Result<User, AuthError> {
    let user = {
        let state = SECURITY.lock();
        let user = state
            .users
            .iter()
            .find(|user| user.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or(AuthError::NoSuchUser)?;
        if !user.login_enabled {
            return Err(AuthError::LoginDisabled);
        }
        if user.pass_hash != password_hash(&user.name, password) {
            return Err(AuthError::BadPassword);
        }
        user
    };
    set_session_from_user(&user);
    crate::event_bus::emit("security", "login", &user.name);
    Ok(public_user(&user))
}

pub fn logout() -> User {
    let guest = {
        let state = SECURITY.lock();
        state
            .users
            .iter()
            .find(|user| user.uid == GUEST_UID)
            .cloned()
            .unwrap_or_else(default_guest_user)
    };
    set_session_from_user(&guest);
    crate::event_bus::emit("security", "logout", &guest.name);
    public_user(&guest)
}

pub fn set_session_for_test(name: &str) -> bool {
    let user = {
        let state = SECURITY.lock();
        state
            .users
            .iter()
            .find(|user| user.name.eq_ignore_ascii_case(name))
            .cloned()
    };
    let Some(user) = user else {
        return false;
    };
    set_session_from_user(&user);
    true
}

pub fn change_password(old_password: &str, new_password: &str) -> Result<(), AuthError> {
    if new_password.len() < 4 {
        return Err(AuthError::PasswordTooShort);
    }
    {
        let mut state = SECURITY.lock();
        let uid = state.session_uid;
        let user = state
            .users
            .iter_mut()
            .find(|user| user.uid == uid)
            .ok_or(AuthError::NoSuchUser)?;
        if user.pass_hash != password_hash(&user.name, old_password) {
            return Err(AuthError::BadPassword);
        }
        user.pass_hash = password_hash(&user.name, new_password);
    }
    persist_users().map_err(|_| AuthError::Io)
}

pub fn require_admin() -> Result<(), AuthError> {
    if can_admin(current_credentials()) {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied)
    }
}

pub fn umask() -> u16 {
    SECURITY.lock().umask
}

pub fn set_umask(mask: u16) -> u16 {
    let mut state = SECURITY.lock();
    let old = state.umask;
    state.umask = mask & 0o777;
    old
}

pub fn apply_umask(mode: u16) -> u16 {
    mode & !SECURITY.lock().umask & 0o777
}

pub fn home_owner_for_path(path: &str) -> Option<(u32, u32)> {
    if path == "/Users" {
        return Some((ROOT_UID, ROOT_GID));
    }
    let rest = path.strip_prefix("/Users/")?;
    let name = rest.split('/').next().unwrap_or("");
    if name.is_empty() {
        return Some((ROOT_UID, ROOT_GID));
    }
    let state = SECURITY.lock();
    state
        .users
        .iter()
        .find(|user| user.name.eq_ignore_ascii_case(name))
        .map(|user| (user.uid, user.gid))
}

pub fn caps_for_permission(permission: &str) -> u32 {
    let mut caps = CAP_EXEC;
    for token in permission.split(',').map(str::trim) {
        match token {
            "shell" => caps |= CAP_PACKAGE_SHELL,
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
    creds.uid == ROOT_UID || creds.caps & CAP_NETWORK != 0
}

pub fn can_desktop(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_DESKTOP != 0
}

pub fn can_admin(creds: Credentials) -> bool {
    creds.uid == ROOT_UID || creds.caps & CAP_ADMIN != 0
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
    push_cap(&mut out, caps, CAP_ADMIN, "admin");
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
            "session user={} uid={} gid={} role={} home={}",
            user.name, user.uid, user.gid, user.role, user.home
        ),
        format!("current task {}", credentials_label(creds)),
        format!("umask={}", format_mode(umask())),
        String::from("users:"),
    ];
    for user in users() {
        lines.push(format!(
            "  {} uid={} gid={} role={} home={} login={}",
            user.name,
            user.uid,
            user.gid,
            user.role,
            user.home,
            if user.login_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }
    lines.push(String::from("groups:"));
    for group in groups() {
        lines.push(format!("  {} gid={}", group.name, group.gid));
    }
    lines.push(String::from(
        "filesystem: CoolFS uid/gid/mode enforced by VFS and syscalls",
    ));
    lines.push(String::from(
        "package grants: manifest permission labels become launch-time task capabilities",
    ));
    lines.extend(app_permission_lines());
    lines
}

fn set_session_from_user(user: &UserRecord) {
    let creds = Credentials {
        uid: user.uid,
        gid: user.gid,
        caps: caps_for_role(&user.role),
    };
    {
        let mut state = SECURITY.lock();
        state.session_uid = creds.uid;
        state.session_gid = creds.gid;
        state.session_caps = creds.caps;
    }
    crate::scheduler::set_current_credentials(creds);
}

fn caps_for_role(role: &str) -> u32 {
    if role == "admin" || role == "root" {
        CAP_ALL_USER
    } else {
        CAP_INTERACTIVE_USER
    }
}

fn public_user(user: &UserRecord) -> User {
    User {
        name: user.name.clone(),
        role: user.role.clone(),
        uid: user.uid,
        gid: user.gid,
        home: user.home.clone(),
        login_enabled: user.login_enabled,
    }
}

fn public_group(group: &GroupRecord) -> Group {
    Group {
        name: group.name.clone(),
        gid: group.gid,
    }
}

fn ensure_home_dirs() {
    let users = users();
    let _ = crate::vfs::vfs_kernel_create_dir("/Users");
    let _ = crate::vfs::vfs_chown("/Users", ROOT_UID, ROOT_GID);
    let _ = crate::vfs::vfs_chmod("/Users", DEFAULT_DIR_MODE);
    for user in users {
        if user.uid == ROOT_UID {
            continue;
        }
        let _ = crate::vfs::vfs_kernel_create_dir(&user.home);
        let _ = crate::vfs::vfs_chown(&user.home, user.uid, user.gid);
        let _ = crate::vfs::vfs_chmod(&user.home, 0o700);
    }
}

fn load_users_from_disk() -> Option<Vec<UserRecord>> {
    let bytes = crate::vfs::vfs_kernel_read_file(USERS_PATH)?;
    let text = core::str::from_utf8(&bytes).ok()?;
    let mut users = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(user) = parse_user_record(line) {
            users.push(user);
        }
    }
    if users.iter().any(|user| user.uid == USER_UID)
        && users.iter().any(|user| user.uid == GUEST_UID)
    {
        Some(users)
    } else {
        None
    }
}

fn parse_user_record(line: &str) -> Option<UserRecord> {
    let mut parts = line.split(':');
    let name = parts.next()?.trim();
    let uid = parse_u32(parts.next()?.trim())?;
    let gid = parse_u32(parts.next()?.trim())?;
    let role = parts.next()?.trim();
    let home = parts.next()?.trim();
    let pass_hash = parse_u32(parts.next()?.trim())?;
    let login_enabled = match parts.next()?.trim() {
        "enabled" => true,
        "disabled" => false,
        _ => return None,
    };
    if name.is_empty() || role.is_empty() || !home.starts_with('/') {
        return None;
    }
    Some(UserRecord {
        name: String::from(name),
        role: String::from(role),
        uid,
        gid,
        home: String::from(home),
        pass_hash,
        login_enabled,
    })
}

fn persist_users() -> Result<(), crate::fat32::FsError> {
    let users = SECURITY.lock().users.clone();
    let mut out = String::from("# coolOS users v1: name:uid:gid:role:home:passhash:login\n");
    for user in users {
        out.push_str(&user.name);
        out.push(':');
        push_u32(&mut out, user.uid);
        out.push(':');
        push_u32(&mut out, user.gid);
        out.push(':');
        out.push_str(&user.role);
        out.push(':');
        out.push_str(&user.home);
        out.push(':');
        push_u32(&mut out, user.pass_hash);
        out.push(':');
        out.push_str(if user.login_enabled {
            "enabled"
        } else {
            "disabled"
        });
        out.push('\n');
    }
    let _ = crate::vfs::vfs_kernel_create_dir("/CONFIG");
    crate::vfs::vfs_kernel_safe_write_file(USERS_PATH, out.as_bytes())
}

fn default_users() -> Vec<UserRecord> {
    let mut users = Vec::new();
    users.push(UserRecord {
        name: String::from("root"),
        role: String::from("root"),
        uid: ROOT_UID,
        gid: ROOT_GID,
        home: String::from("/root"),
        pass_hash: password_hash("root", "root"),
        login_enabled: false,
    });
    users.push(default_admin_user());
    users.push(default_guest_user());
    users
}

fn default_admin_user() -> UserRecord {
    UserRecord {
        name: String::from("jamie"),
        role: String::from("admin"),
        uid: USER_UID,
        gid: USER_GID,
        home: String::from("/Users/jamie"),
        pass_hash: password_hash("jamie", "cool"),
        login_enabled: true,
    }
}

fn default_guest_user() -> UserRecord {
    UserRecord {
        name: String::from("guest"),
        role: String::from("user"),
        uid: GUEST_UID,
        gid: USER_GID,
        home: String::from("/Users/guest"),
        pass_hash: password_hash("guest", "guest"),
        login_enabled: true,
    }
}

fn default_groups() -> Vec<GroupRecord> {
    alloc::vec![
        GroupRecord {
            name: String::from("root"),
            gid: ROOT_GID,
        },
        GroupRecord {
            name: String::from("services"),
            gid: SERVICE_GID,
        },
        GroupRecord {
            name: String::from("users"),
            gid: USER_GID,
        },
        GroupRecord {
            name: String::from("wheel"),
            gid: 10,
        },
    ]
}

fn password_hash(name: &str, password: &str) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in name
        .bytes()
        .chain(core::iter::once(b':'))
        .chain(password.bytes())
    {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn parse_u32(input: &str) -> Option<u32> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0u32;
    for byte in input.bytes() {
        if !byte.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((byte - b'0') as u32)?;
    }
    Some(out)
}

fn push_u32(out: &mut String, mut value: u32) {
    if value == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0u8; 10];
    let mut len = 0usize;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for idx in (0..len).rev() {
        out.push(digits[idx] as char);
    }
}
