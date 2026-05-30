extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

const USERS_PATH: &str = "/CONFIG/USERS.DB";
const FIRST_BOOT_PATH: &str = "/CONFIG/FIRSTBOOT.CFG";
const MIN_PASSWORD_LEN: usize = 8;
const MAX_LOGIN_FAILURES: u32 = 3;
const LOGIN_LOCKOUT_MS: u64 = 5_000;

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
pub const SHARED_TMP_MODE: u16 = 0o777;

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

#[derive(Clone)]
struct LoginAttempt {
    name: String,
    failures: u32,
    locked_until_tick: u64,
}

struct SecurityState {
    users: Vec<UserRecord>,
    groups: Vec<GroupRecord>,
    login_attempts: Vec<LoginAttempt>,
    session_uid: u32,
    session_gid: u32,
    session_caps: u32,
    umask: u16,
    revision: u64,
}

impl SecurityState {
    const fn empty() -> Self {
        Self {
            users: Vec::new(),
            groups: Vec::new(),
            login_attempts: Vec::new(),
            session_uid: USER_UID,
            session_gid: USER_GID,
            session_caps: CAP_ALL_USER,
            umask: 0o022,
            revision: 0,
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
    LoginThrottled,
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
            AuthError::LoginThrottled => "login temporarily locked",
            AuthError::PermissionDenied => "permission denied",
            AuthError::Io => "could not persist user database",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccountError {
    NoSuchUser,
    DuplicateUser,
    InvalidName,
    InvalidRole,
    PasswordTooShort,
    PermissionDenied,
    LastAdmin,
    ProtectedUser,
    AlreadyConfigured,
    Io,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FirstBootState {
    Required,
    InProgress,
    Complete,
}

impl FirstBootState {
    pub const fn as_str(self) -> &'static str {
        match self {
            FirstBootState::Required => "required",
            FirstBootState::InProgress => "in_progress",
            FirstBootState::Complete => "complete",
        }
    }
}

impl AccountError {
    pub const fn as_str(self) -> &'static str {
        match self {
            AccountError::NoSuchUser => "no such user",
            AccountError::DuplicateUser => "user already exists",
            AccountError::InvalidName => "invalid user name",
            AccountError::InvalidRole => "invalid role",
            AccountError::PasswordTooShort => "password too short",
            AccountError::PermissionDenied => "permission denied",
            AccountError::LastAdmin => "cannot remove the last enabled admin",
            AccountError::ProtectedUser => "protected user",
            AccountError::AlreadyConfigured => "first-run setup already completed",
            AccountError::Io => "could not persist user database",
        }
    }
}

pub fn init() {
    let mut users = load_users_from_disk().unwrap_or_else(default_users);
    normalize_default_admin(&mut users);
    let groups = default_groups();
    let session = default_session_user(&users);
    {
        let mut state = SECURITY.lock();
        state.users = users;
        state.groups = groups;
        state.login_attempts.clear();
        state.session_uid = session.uid;
        state.session_gid = session.gid;
        state.session_caps = caps_for_role(&session.role);
        state.umask = 0o022;
        state.revision = state.revision.wrapping_add(1);
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

pub fn revision() -> u64 {
    SECURITY.lock().revision
}

pub fn first_run_required() -> bool {
    SECURITY.lock().users.iter().any(is_default_admin_password)
}

pub fn first_boot_state() -> FirstBootState {
    if !first_run_required() {
        return FirstBootState::Complete;
    }
    let Some(bytes) = crate::vfs::vfs_kernel_read_file(FIRST_BOOT_PATH) else {
        return FirstBootState::Required;
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return FirstBootState::Required;
    };
    for line in text.lines() {
        let Some(value) = line.trim().strip_prefix("state=") else {
            continue;
        };
        return match value.trim() {
            "in_progress" => FirstBootState::InProgress,
            "complete" => FirstBootState::Complete,
            _ => FirstBootState::Required,
        };
    }
    FirstBootState::Required
}

pub fn mark_first_boot_in_progress() -> Result<(), crate::fat32::FsError> {
    if !first_run_required() {
        return mark_first_boot_complete("", "");
    }
    write_first_boot_state(FirstBootState::InProgress, "", "")
}

pub fn mark_first_boot_complete(user: &str, device: &str) -> Result<(), crate::fat32::FsError> {
    write_first_boot_state(FirstBootState::Complete, user, device)
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
    if login_locked(name) {
        return Err(AuthError::LoginThrottled);
    }
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
            let attempted = user.name.clone();
            drop(state);
            record_login_failure(&attempted);
            return Err(AuthError::BadPassword);
        }
        user
    };
    clear_login_failures(&user.name);
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
    if !valid_password(new_password) {
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
        state.revision = state.revision.wrapping_add(1);
    }
    persist_users().map_err(|_| AuthError::Io)
}

pub fn complete_first_run_admin(name: &str, password: &str) -> Result<User, AccountError> {
    let name = clean_user_name(name)?;
    if name.eq_ignore_ascii_case("guest") {
        return Err(AccountError::ProtectedUser);
    }
    validate_password_for_account(password)?;
    let user = {
        let mut state = SECURITY.lock();
        if !state.users.iter().any(is_default_admin_password) {
            return Err(AccountError::AlreadyConfigured);
        }
        let user = if name.eq_ignore_ascii_case("root") {
            let user = state
                .users
                .iter_mut()
                .find(|user| user.name.eq_ignore_ascii_case(&name))
                .ok_or(AccountError::NoSuchUser)?;
            user.role = String::from("admin");
            user.pass_hash = password_hash(&user.name, password);
            user.login_enabled = true;
            user.clone()
        } else {
            if state
                .users
                .iter()
                .any(|user| user.name.eq_ignore_ascii_case(&name))
            {
                return Err(AccountError::DuplicateUser);
            }
            let new_user = UserRecord {
                name: name.clone(),
                role: String::from("admin"),
                uid: next_available_uid(&state.users),
                gid: USER_GID,
                home: user_home(&name),
                pass_hash: password_hash(&name, password),
                login_enabled: true,
            };
            for user in state.users.iter_mut() {
                if is_default_admin_password(user) {
                    user.login_enabled = false;
                    user.pass_hash = password_hash(&user.name, "disabled");
                }
            }
            state.users.push(new_user.clone());
            new_user
        };
        state.revision = state.revision.wrapping_add(1);
        user
    };
    persist_users().map_err(|_| AccountError::Io)?;
    ensure_home_dirs();
    let _ = mark_first_boot_complete(&user.name, "");
    set_session_from_user(&user);
    crate::event_bus::emit("security", "first-run", &user.name);
    Ok(public_user(&user))
}

pub fn create_user(name: &str, password: &str, role: &str) -> Result<User, AccountError> {
    require_admin_account()?;
    let name = clean_user_name(name)?;
    validate_password_for_account(password)?;
    let role = clean_role(role)?;
    let user = {
        let mut state = SECURITY.lock();
        if state
            .users
            .iter()
            .any(|user| user.name.eq_ignore_ascii_case(&name))
        {
            return Err(AccountError::DuplicateUser);
        }
        let user = UserRecord {
            name: name.clone(),
            role,
            uid: next_available_uid(&state.users),
            gid: USER_GID,
            home: user_home(&name),
            pass_hash: password_hash(&name, password),
            login_enabled: true,
        };
        state.users.push(user.clone());
        state.revision = state.revision.wrapping_add(1);
        user
    };
    persist_users().map_err(|_| AccountError::Io)?;
    ensure_home_dirs();
    crate::event_bus::emit("security", "user-create", &user.name);
    Ok(public_user(&user))
}

pub fn set_user_enabled(name: &str, enabled: bool) -> Result<User, AccountError> {
    require_admin_account()?;
    let user = {
        let mut state = SECURITY.lock();
        let idx = user_index(&state.users, name).ok_or(AccountError::NoSuchUser)?;
        if !enabled && state.users[idx].uid == state.session_uid {
            return Err(AccountError::ProtectedUser);
        }
        if !enabled && is_admin_record(&state.users[idx]) && enabled_admin_count(&state.users) <= 1
        {
            return Err(AccountError::LastAdmin);
        }
        state.users[idx].login_enabled = enabled;
        let user = state.users[idx].clone();
        state.revision = state.revision.wrapping_add(1);
        user
    };
    persist_users().map_err(|_| AccountError::Io)?;
    crate::event_bus::emit(
        "security",
        if enabled {
            "user-enable"
        } else {
            "user-disable"
        },
        &user.name,
    );
    Ok(public_user(&user))
}

pub fn set_user_role(name: &str, role: &str) -> Result<User, AccountError> {
    require_admin_account()?;
    let role = clean_role(role)?;
    let (user, refresh_session) = {
        let mut state = SECURITY.lock();
        let idx = user_index(&state.users, name).ok_or(AccountError::NoSuchUser)?;
        if !is_admin_role(&role)
            && is_admin_record(&state.users[idx])
            && enabled_admin_count(&state.users) <= 1
        {
            return Err(AccountError::LastAdmin);
        }
        let refresh_session = state.users[idx].uid == state.session_uid;
        state.users[idx].role = role;
        let user = state.users[idx].clone();
        state.revision = state.revision.wrapping_add(1);
        (user, refresh_session)
    };
    persist_users().map_err(|_| AccountError::Io)?;
    if refresh_session {
        set_session_from_user(&user);
    }
    crate::event_bus::emit("security", "user-role", &user.name);
    Ok(public_user(&user))
}

pub fn reset_user_password(name: &str, password: &str) -> Result<User, AccountError> {
    require_admin_account()?;
    validate_password_for_account(password)?;
    let user = {
        let mut state = SECURITY.lock();
        let idx = user_index(&state.users, name).ok_or(AccountError::NoSuchUser)?;
        let user_name = state.users[idx].name.clone();
        state.users[idx].pass_hash = password_hash(&user_name, password);
        let user = state.users[idx].clone();
        state.revision = state.revision.wrapping_add(1);
        user
    };
    clear_login_failures(&user.name);
    persist_users().map_err(|_| AccountError::Io)?;
    crate::event_bus::emit("security", "user-password", &user.name);
    Ok(public_user(&user))
}

pub fn delete_user(name: &str) -> Result<User, AccountError> {
    require_admin_account()?;
    let user = {
        let mut state = SECURITY.lock();
        let idx = user_index(&state.users, name).ok_or(AccountError::NoSuchUser)?;
        if state.users[idx].uid == state.session_uid
            || state.users[idx].name.eq_ignore_ascii_case("root")
            || state.users[idx].name.eq_ignore_ascii_case("guest")
        {
            return Err(AccountError::ProtectedUser);
        }
        if is_admin_record(&state.users[idx]) && enabled_admin_count(&state.users) <= 1 {
            return Err(AccountError::LastAdmin);
        }
        let user = state.users.remove(idx);
        state.revision = state.revision.wrapping_add(1);
        user
    };
    persist_users().map_err(|_| AccountError::Io)?;
    crate::event_bus::emit("security", "user-delete", &user.name);
    Ok(public_user(&user))
}

pub fn suggested_user_name(prefix: &str) -> String {
    let base = if prefix.is_empty() { "user" } else { prefix };
    for idx in 1..1000usize {
        let mut name = String::from(base);
        push_u32(&mut name, idx as u32);
        if user_by_name(&name).is_none() {
            return name;
        }
    }
    String::from("user")
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
        format!(
            "first-run setup={}",
            if first_run_required() {
                "required"
            } else {
                "complete"
            }
        ),
        format!(
            "first-boot state={} path={}",
            first_boot_state().as_str(),
            FIRST_BOOT_PATH
        ),
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

fn require_admin_account() -> Result<(), AccountError> {
    require_admin().map_err(|_| AccountError::PermissionDenied)
}

fn default_session_user(users: &[UserRecord]) -> UserRecord {
    users
        .iter()
        .find(|user| user.uid == USER_UID && user.login_enabled)
        .or_else(|| {
            users
                .iter()
                .find(|user| user.login_enabled && (user.role == "admin" || user.role == "root"))
        })
        .or_else(|| users.iter().find(|user| user.login_enabled))
        .cloned()
        .unwrap_or_else(default_admin_user)
}

fn user_index(users: &[UserRecord], name: &str) -> Option<usize> {
    users
        .iter()
        .position(|user| user.name.eq_ignore_ascii_case(name))
}

fn enabled_admin_count(users: &[UserRecord]) -> usize {
    users
        .iter()
        .filter(|user| user.login_enabled && is_admin_record(user))
        .count()
}

fn is_admin_record(user: &UserRecord) -> bool {
    is_admin_role(&user.role)
}

fn is_admin_role(role: &str) -> bool {
    role == "admin" || role == "root"
}

fn is_default_admin_password(user: &UserRecord) -> bool {
    user.name.eq_ignore_ascii_case("root")
        && user.uid == USER_UID
        && user.login_enabled
        && user.pass_hash == password_hash("root", "cool")
}

fn next_available_uid(users: &[UserRecord]) -> u32 {
    let mut uid = users
        .iter()
        .map(|user| user.uid)
        .filter(|uid| *uid >= GUEST_UID)
        .max()
        .unwrap_or(GUEST_UID)
        .saturating_add(1);
    while users.iter().any(|user| user.uid == uid) {
        uid = uid.saturating_add(1);
    }
    uid
}

fn clean_role(role: &str) -> Result<String, AccountError> {
    let role = role.trim();
    if role.eq_ignore_ascii_case("admin") || role.eq_ignore_ascii_case("root") {
        Ok(String::from("admin"))
    } else if role.eq_ignore_ascii_case("user") || role.eq_ignore_ascii_case("guest") {
        Ok(String::from("user"))
    } else {
        Err(AccountError::InvalidRole)
    }
}

fn clean_user_name(name: &str) -> Result<String, AccountError> {
    let name = name.trim();
    let len = name.chars().count();
    if len < 2 || len > 16 {
        return Err(AccountError::InvalidName);
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(AccountError::InvalidName);
    };
    if !first.is_ascii_alphabetic() {
        return Err(AccountError::InvalidName);
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AccountError::InvalidName);
    }
    Ok(String::from(name))
}

fn user_home(name: &str) -> String {
    let mut home = String::from("/Users/");
    home.push_str(name);
    home
}

fn valid_password(password: &str) -> bool {
    password.len() >= MIN_PASSWORD_LEN && !password.contains(':') && !password.contains('\n')
}

fn validate_password_for_account(password: &str) -> Result<(), AccountError> {
    if valid_password(password) {
        Ok(())
    } else {
        Err(AccountError::PasswordTooShort)
    }
}

fn login_locked(name: &str) -> bool {
    let now = crate::interrupts::ticks();
    SECURITY
        .lock()
        .login_attempts
        .iter()
        .find(|attempt| attempt.name.eq_ignore_ascii_case(name))
        .map(|attempt| attempt.locked_until_tick > now)
        .unwrap_or(false)
}

fn record_login_failure(name: &str) {
    let now = crate::interrupts::ticks();
    let lockout_ticks = crate::interrupts::ticks_for_millis(LOGIN_LOCKOUT_MS);
    let mut state = SECURITY.lock();
    let idx = state
        .login_attempts
        .iter()
        .position(|attempt| attempt.name.eq_ignore_ascii_case(name));
    let Some(idx) = idx else {
        state.login_attempts.push(LoginAttempt {
            name: String::from(name),
            failures: 1,
            locked_until_tick: 0,
        });
        return;
    };
    let attempt = &mut state.login_attempts[idx];
    if attempt.locked_until_tick <= now {
        attempt.failures = attempt.failures.saturating_add(1);
    }
    if attempt.failures >= MAX_LOGIN_FAILURES {
        attempt.locked_until_tick = now.wrapping_add(lockout_ticks);
        attempt.failures = 0;
    }
}

fn clear_login_failures(name: &str) {
    SECURITY
        .lock()
        .login_attempts
        .retain(|attempt| !attempt.name.eq_ignore_ascii_case(name));
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

fn normalize_default_admin(users: &mut Vec<UserRecord>) {
    users.retain(|user| {
        !(user.uid == ROOT_UID && user.name.eq_ignore_ascii_case("root") && !user.login_enabled)
    });

    let root_admin = users
        .iter()
        .any(|user| user.uid == USER_UID && user.name.eq_ignore_ascii_case("root"));
    if !root_admin {
        if let Some(user) = users
            .iter_mut()
            .find(|user| user.uid == USER_UID && user.role == "admin")
        {
            user.name = String::from("root");
            user.home = String::from("/Users/root");
            user.pass_hash = password_hash("root", "cool");
            user.login_enabled = true;
        }
    }

    if !users.iter().any(|user| user.uid == USER_UID) {
        users.push(default_admin_user());
    }
    if !users.iter().any(|user| user.uid == GUEST_UID) {
        users.push(default_guest_user());
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

fn write_first_boot_state(
    state: FirstBootState,
    user: &str,
    device: &str,
) -> Result<(), crate::fat32::FsError> {
    let mut out = String::from("# coolOS first boot v1\n");
    out.push_str("version=80\n");
    out.push_str("state=");
    out.push_str(state.as_str());
    out.push('\n');
    if !user.trim().is_empty() {
        out.push_str("user=");
        push_safe_config_value(&mut out, user.trim());
        out.push('\n');
    }
    if !device.trim().is_empty() {
        out.push_str("device=");
        push_safe_config_value(&mut out, device.trim());
        out.push('\n');
    }
    let _ = crate::vfs::vfs_kernel_create_dir("/CONFIG");
    crate::vfs::vfs_kernel_safe_write_file(FIRST_BOOT_PATH, out.as_bytes())
}

fn push_safe_config_value(out: &mut String, value: &str) {
    for ch in value.chars() {
        if ch == '\n' || ch == '\r' || ch == '=' {
            continue;
        }
        out.push(ch);
    }
}

fn default_users() -> Vec<UserRecord> {
    let mut users = Vec::new();
    users.push(default_admin_user());
    users.push(default_guest_user());
    users
}

fn default_admin_user() -> UserRecord {
    UserRecord {
        name: String::from("root"),
        role: String::from("admin"),
        uid: USER_UID,
        gid: USER_GID,
        home: String::from("/Users/root"),
        pass_hash: password_hash("root", "cool"),
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
