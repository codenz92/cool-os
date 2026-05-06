extern crate alloc;

use alloc::{string::String, vec::Vec};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct AppMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub glyph: &'static str,
    pub command: &'static str,
    pub category: AppCategory,
    pub permission: &'static str,
    pub aliases: &'static [&'static str],
    pub associations: &'static [&'static str],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    System,
    Files,
    Network,
    Tools,
    Games,
    Settings,
    Development,
}

impl AppCategory {
    pub const fn label(self) -> &'static str {
        match self {
            AppCategory::System => "System",
            AppCategory::Files => "Files",
            AppCategory::Network => "Network",
            AppCategory::Tools => "Tools",
            AppCategory::Games => "Games",
            AppCategory::Settings => "Settings",
            AppCategory::Development => "Development",
        }
    }
}

#[derive(Clone)]
pub enum Association {
    Directory,
    Executable,
    Text,
    AppShortcut(String),
    Unknown,
}

#[derive(Clone, Copy)]
pub enum LauncherKind {
    App(&'static str),
    Path(&'static str),
    Command(&'static str),
}

#[derive(Clone, Copy)]
pub struct LauncherEntry {
    pub label: &'static str,
    pub detail: &'static str,
    pub kind: LauncherKind,
}

#[derive(Clone)]
pub struct AppManifest {
    pub id: String,
    pub name: String,
    pub command: String,
    pub version: String,
    pub icon: String,
    pub category: String,
    pub permission: String,
    pub exec_path: String,
    pub aliases: Vec<String>,
    pub associations: Vec<String>,
}

pub const APPS: &[AppMetadata] = &[
    AppMetadata {
        id: "app.terminal",
        name: "Terminal",
        glyph: "T>",
        command: "terminal",
        category: AppCategory::System,
        permission: "shell",
        aliases: &["shell", "console", "cmd", "command"],
        associations: &["CMD"],
    },
    AppMetadata {
        id: "app.sysmon",
        name: "System Monitor",
        glyph: "M#",
        command: "sysmon",
        category: AppCategory::System,
        permission: "diagnostics",
        aliases: &["monitor", "tasks", "processes", "performance"],
        associations: &[],
    },
    AppMetadata {
        id: "app.diagnostics",
        name: "Diagnostics",
        glyph: "D!",
        command: "diagnostics",
        category: AppCategory::System,
        permission: "diagnostics",
        aliases: &["diag", "health", "logs", "profiler", "debug"],
        associations: &[],
    },
    AppMetadata {
        id: "app.files",
        name: "File Manager",
        glyph: "FM",
        command: "files",
        category: AppCategory::Files,
        permission: "filesystem",
        aliases: &["files", "folders", "explorer", "documents"],
        associations: &["DIR"],
    },
    AppMetadata {
        id: "app.viewer",
        name: "Text Viewer",
        glyph: "Tx",
        command: "viewer",
        category: AppCategory::Files,
        permission: "read-files",
        aliases: &["text", "notes", "readme", "viewer"],
        associations: &["TXT", "MD", "LOG", "CFG", "RS"],
    },
    AppMetadata {
        id: "app.editor",
        name: "Text Editor",
        glyph: "ED",
        command: "editor",
        category: AppCategory::Files,
        permission: "filesystem",
        aliases: &["edit", "write", "notepad", "document"],
        associations: &[],
    },
    AppMetadata {
        id: "app.notes",
        name: "Notes",
        glyph: "NT",
        command: "notes",
        category: AppCategory::Tools,
        permission: "filesystem",
        aliases: &["note", "memo", "scratchpad", "journal"],
        associations: &[],
    },
    AppMetadata {
        id: "app.trash",
        name: "Trash Bin",
        glyph: "TR",
        command: "trash",
        category: AppCategory::System,
        permission: "filesystem",
        aliases: &["trash", "bin", "deleted", "recycle"],
        associations: &[],
    },
    AppMetadata {
        id: "app.screenshot",
        name: "Screenshot",
        glyph: "SS",
        command: "screenshot",
        category: AppCategory::Tools,
        permission: "desktop",
        aliases: &["screen", "capture", "snapshot", "shot"],
        associations: &[],
    },
    AppMetadata {
        id: "app.browser",
        name: "Web Browser",
        glyph: "WB",
        command: "browser",
        category: AppCategory::Network,
        permission: "network",
        aliases: &["browser", "web", "www", "internet", "http"],
        associations: &["HTML", "HTM", "URL"],
    },
    AppMetadata {
        id: "app.colors",
        name: "Color Picker",
        glyph: "CP",
        command: "colors",
        category: AppCategory::Tools,
        permission: "desktop",
        aliases: &["colors", "palette", "theme"],
        associations: &[],
    },
    AppMetadata {
        id: "app.display",
        name: "Display Settings",
        glyph: "DS",
        command: "display",
        category: AppCategory::Settings,
        permission: "settings",
        aliases: &[
            "settings",
            "display",
            "accessibility",
            "network",
            "storage",
            "power",
        ],
        associations: &[],
    },
    AppMetadata {
        id: "app.personalize",
        name: "Personalize",
        glyph: "P*",
        command: "personalize",
        category: AppCategory::Settings,
        permission: "settings",
        aliases: &["wallpaper", "theme", "desktop"],
        associations: &[],
    },
    AppMetadata {
        id: "app.crash",
        name: "Crash Viewer",
        glyph: "CV",
        command: "crash",
        category: AppCategory::System,
        permission: "diagnostics",
        aliases: &["crash", "dump", "fault", "panic"],
        associations: &["DMP"],
    },
    AppMetadata {
        id: "app.logs",
        name: "Log Viewer",
        glyph: "LV",
        command: "logs",
        category: AppCategory::System,
        permission: "diagnostics",
        aliases: &["logs", "kernel", "services", "events"],
        associations: &["LOG"],
    },
    AppMetadata {
        id: "app.profiler",
        name: "Boot Profiler",
        glyph: "BP",
        command: "profiler",
        category: AppCategory::System,
        permission: "diagnostics",
        aliases: &["boot", "profiler", "startup", "timing"],
        associations: &[],
    },
    AppMetadata {
        id: "app.welcome",
        name: "Welcome",
        glyph: "W?",
        command: "welcome",
        category: AppCategory::System,
        permission: "desktop",
        aliases: &["help", "cheatsheet", "shortcuts"],
        associations: &[],
    },
    AppMetadata {
        id: "app.guidemo",
        name: "GUI Demo",
        glyph: "UG",
        command: "guidemo",
        category: AppCategory::Development,
        permission: "desktop",
        aliases: &["gui", "userspace", "sdk", "window"],
        associations: &[],
    },
];

pub const APP_CATEGORIES: &[AppCategory] = &[
    AppCategory::System,
    AppCategory::Files,
    AppCategory::Network,
    AppCategory::Tools,
    AppCategory::Games,
    AppCategory::Settings,
    AppCategory::Development,
];

pub const LAUNCHER_ENTRIES: &[LauncherEntry] = &[
    LauncherEntry {
        label: "Terminal",
        detail: "open shell",
        kind: LauncherKind::App("Terminal"),
    },
    LauncherEntry {
        label: "Files",
        detail: "open File Manager",
        kind: LauncherKind::App("File Manager"),
    },
    LauncherEntry {
        label: "System Monitor",
        detail: "runtime dashboard",
        kind: LauncherKind::App("System Monitor"),
    },
    LauncherEntry {
        label: "Web Browser",
        detail: "open HTTP web pages",
        kind: LauncherKind::App("Web Browser"),
    },
    LauncherEntry {
        label: "Diagnostics",
        detail: "combined logs and system health",
        kind: LauncherKind::App("Diagnostics"),
    },
    LauncherEntry {
        label: "Display Settings",
        detail: "desktop settings",
        kind: LauncherKind::App("Display Settings"),
    },
    LauncherEntry {
        label: "Personalize",
        detail: "wallpaper presets",
        kind: LauncherKind::App("Personalize"),
    },
    LauncherEntry {
        label: "Text Viewer",
        detail: "open text viewer",
        kind: LauncherKind::App("Text Viewer"),
    },
    LauncherEntry {
        label: "Text Editor",
        detail: "ring-3 editor",
        kind: LauncherKind::App("Text Editor"),
    },
    LauncherEntry {
        label: "Notes",
        detail: "ring-3 notes",
        kind: LauncherKind::App("Notes"),
    },
    LauncherEntry {
        label: "Trash Bin",
        detail: "ring-3 trash utility",
        kind: LauncherKind::App("Trash Bin"),
    },
    LauncherEntry {
        label: "Screenshot",
        detail: "ring-3 capture utility",
        kind: LauncherKind::App("Screenshot"),
    },
    LauncherEntry {
        label: "Color Picker",
        detail: "open palette",
        kind: LauncherKind::App("Color Picker"),
    },
    LauncherEntry {
        label: "GUI Demo",
        detail: "ring-3 window app",
        kind: LauncherKind::App("GUI Demo"),
    },
    LauncherEntry {
        label: "Crash Viewer",
        detail: "open crash reports",
        kind: LauncherKind::App("Crash Viewer"),
    },
    LauncherEntry {
        label: "Log Viewer",
        detail: "kernel/service/filesystem logs",
        kind: LauncherKind::App("Log Viewer"),
    },
    LauncherEntry {
        label: "Boot Profiler",
        detail: "boot phases and service timing",
        kind: LauncherKind::App("Boot Profiler"),
    },
    LauncherEntry {
        label: "Welcome",
        detail: "shortcut cheatsheet",
        kind: LauncherKind::App("Welcome"),
    },
    LauncherEntry {
        label: "hello.txt",
        detail: "/bin/hello.txt",
        kind: LauncherKind::Path("/bin/hello.txt"),
    },
    LauncherEntry {
        label: "Documents",
        detail: "/Documents",
        kind: LauncherKind::Path("/Documents"),
    },
    LauncherEntry {
        label: "Desktop",
        detail: "/Desktop",
        kind: LauncherKind::Path("/Desktop"),
    },
    LauncherEntry {
        label: "Trash",
        detail: "/Trash",
        kind: LauncherKind::Path("/Trash"),
    },
    LauncherEntry {
        label: "Run ps",
        detail: "terminal command",
        kind: LauncherKind::Command("ps"),
    },
    LauncherEntry {
        label: "Run devices",
        detail: "terminal command",
        kind: LauncherKind::Command("devices"),
    },
    LauncherEntry {
        label: "Run net",
        detail: "terminal command",
        kind: LauncherKind::Command("net"),
    },
    LauncherEntry {
        label: "Run fsck",
        detail: "terminal command",
        kind: LauncherKind::Command("fsck"),
    },
    LauncherEntry {
        label: "Run log",
        detail: "terminal command",
        kind: LauncherKind::Command("log"),
    },
];

#[allow(dead_code)]
pub fn app_by_command(command: &str) -> Option<&'static AppMetadata> {
    APPS.iter()
        .find(|app| app.command.eq_ignore_ascii_case(command))
}

pub fn app_by_name(name: &str) -> Option<&'static AppMetadata> {
    APPS.iter().find(|app| app.name.eq_ignore_ascii_case(name))
}

pub fn app_by_id_or_command(value: &str) -> Option<&'static AppMetadata> {
    APPS.iter().find(|app| {
        app.id.eq_ignore_ascii_case(value)
            || app.command.eq_ignore_ascii_case(value)
            || app.name.eq_ignore_ascii_case(value)
    })
}

pub fn installed_app_manifests() -> Vec<AppManifest> {
    let Some(dirs) = crate::vfs::vfs_list_dir("/APPS") else {
        return Vec::new();
    };
    let mut manifests = Vec::new();
    for dir in dirs.iter().filter(|entry| entry.is_dir).take(32) {
        let mut path = String::from("/APPS/");
        path.push_str(&dir.name);
        path.push_str("/APP.CFG");
        let Some(bytes) = crate::vfs::vfs_read_file(&path) else {
            continue;
        };
        let Ok(text) = core::str::from_utf8(&bytes) else {
            continue;
        };
        let id = manifest_value(text, "id").unwrap_or("app.unknown");
        let name = manifest_value(text, "name").unwrap_or(&dir.name);
        let command = manifest_value(text, "command").unwrap_or(&dir.name);
        let version = manifest_value(text, "version").unwrap_or("builtin");
        let icon = manifest_value(text, "icon").unwrap_or("[]");
        let category = manifest_value(text, "category").unwrap_or("Tools");
        let permission = manifest_value(text, "permission").unwrap_or("user");
        let exec_path = manifest_value(text, "exec")
            .map(String::from)
            .unwrap_or_else(|| default_exec_for_command(command));
        let aliases = manifest_value(text, "aliases")
            .map(parse_manifest_list)
            .unwrap_or_default();
        let associations = manifest_value(text, "associations")
            .map(parse_manifest_list)
            .unwrap_or_default();
        manifests.push(AppManifest {
            id: String::from(id),
            name: String::from(name),
            command: String::from(command),
            version: String::from(version),
            icon: String::from(icon),
            category: String::from(category),
            permission: String::from(permission),
            exec_path,
            aliases,
            associations,
        });
    }
    manifests
}

pub fn installed_manifest_by_id_or_command(value: &str) -> Option<AppManifest> {
    installed_app_manifests().into_iter().find(|manifest| {
        manifest.id.eq_ignore_ascii_case(value)
            || manifest.command.eq_ignore_ascii_case(value)
            || manifest.name.eq_ignore_ascii_case(value)
    })
}

pub fn is_builtin_id(id: &str) -> bool {
    APPS.iter().any(|app| app.id.eq_ignore_ascii_case(id))
}

pub fn validate_installed_manifests() -> Result<usize, &'static str> {
    let Some(dirs) = crate::vfs::vfs_list_dir("/APPS") else {
        return Err("missing /APPS");
    };
    let mut count = 0usize;
    for dir in dirs.iter().filter(|entry| entry.is_dir).take(64) {
        let mut path = String::from("/APPS/");
        path.push_str(&dir.name);
        path.push_str("/APP.CFG");
        let Some(bytes) = crate::vfs::vfs_read_file(&path) else {
            return Err("missing APP.CFG");
        };
        let text = core::str::from_utf8(&bytes).map_err(|_| "manifest is not utf8")?;
        validate_manifest_text(text)?;
        let command = manifest_value(text, "command").ok_or("missing command")?;
        if !command.eq_ignore_ascii_case(&dir.name) {
            return Err("manifest command mismatch");
        }
        count += 1;
    }
    if count < APPS.len() {
        return Err("missing built-in manifests");
    }
    Ok(count)
}

pub fn validate_manifest_text(text: &str) -> Result<(), &'static str> {
    let id = manifest_value(text, "id").ok_or("missing id")?;
    let name = manifest_value(text, "name").ok_or("missing name")?;
    let command = manifest_value(text, "command").ok_or("missing command")?;
    let icon = manifest_value(text, "icon").ok_or("missing icon")?;
    let category = manifest_value(text, "category").ok_or("missing category")?;
    let permission = manifest_value(text, "permission").ok_or("missing permission")?;
    let exec_path = manifest_value(text, "exec").ok_or("missing exec")?;

    if !id.starts_with("app.") || !safe_manifest_token(id, true) {
        return Err("invalid id");
    }
    if name.len() > 32 || !safe_manifest_label(name) {
        return Err("invalid name");
    }
    if command.len() > 24 || !safe_manifest_token(command, false) {
        return Err("invalid command");
    }
    if icon.len() > 4 || icon.contains('/') || icon.contains("..") {
        return Err("invalid icon");
    }
    if !APP_CATEGORIES
        .iter()
        .any(|known| known.label().eq_ignore_ascii_case(category))
    {
        return Err("invalid category");
    }
    if permission.len() > 48 {
        return Err("invalid permission");
    }
    for grant in permission.split(',').map(str::trim) {
        if grant.is_empty() || grant.len() > 24 || !safe_manifest_token(grant, false) {
            return Err("invalid permission");
        }
    }
    if !safe_exec_path(exec_path) {
        return Err("invalid exec");
    }
    if let Some(version) = manifest_value(text, "version") {
        if version.len() > 16 || !safe_manifest_token(version, true) {
            return Err("invalid version");
        }
    }
    if let Some(aliases) = manifest_value(text, "aliases") {
        for alias in aliases.split(',').map(str::trim) {
            if !alias.is_empty() && (alias.len() > 24 || !safe_manifest_label(alias)) {
                return Err("invalid alias");
            }
        }
    }
    if let Some(associations) = manifest_value(text, "associations") {
        for assoc in associations.split(',').map(str::trim) {
            if !assoc.is_empty()
                && (assoc.len() > 12 || assoc.contains('/') || assoc.contains(".."))
            {
                return Err("invalid association");
            }
        }
    }
    Ok(())
}

pub fn category_lines() -> Vec<String> {
    let mut lines = Vec::new();
    for category in APP_CATEGORIES {
        let mut line = String::from(category.label());
        line.push_str(": ");
        let mut count = 0usize;
        for app in APPS.iter().filter(|app| app.category == *category) {
            if count > 0 {
                line.push_str(", ");
            }
            line.push_str(app.name);
            count += 1;
        }
        if count == 0 {
            line.push_str("(empty)");
        }
        lines.push(line);
    }
    lines
}

pub fn association_for(path: &str, is_dir: bool) -> Association {
    if is_dir {
        return Association::Directory;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    for app in APPS {
        if !crate::packages::is_installed(app.id) {
            continue;
        }
        if name.eq_ignore_ascii_case(app.name) || name.eq_ignore_ascii_case(app.command) {
            return Association::AppShortcut(String::from(app.name));
        }
    }
    let ext = file_ext(name);
    if ext.eq_ignore_ascii_case("ELF") {
        return Association::Executable;
    }
    if is_text_extension(ext) {
        return Association::Text;
    }
    for app in APPS {
        if crate::packages::is_installed(app.id) && matches_ignore_ascii(ext, app.associations) {
            return Association::AppShortcut(String::from(app.name));
        }
    }
    for manifest in installed_app_manifests() {
        if is_builtin_id(&manifest.id) || !crate::packages::is_installed(&manifest.id) {
            continue;
        }
        if manifest
            .associations
            .iter()
            .any(|assoc| ext.eq_ignore_ascii_case(assoc))
        {
            return Association::AppShortcut(manifest.name);
        }
    }
    Association::Unknown
}

pub fn is_text_extension(ext: &str) -> bool {
    matches_ignore_ascii(ext, &["TXT", "MD", "LOG", "CFG", "RS"])
}

fn file_ext(name: &str) -> &str {
    name.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("")
}

fn matches_ignore_ascii(value: &str, options: &[&str]) -> bool {
    options
        .iter()
        .any(|option| value.eq_ignore_ascii_case(option))
}

fn safe_manifest_label(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('/')
        && !value.contains("..")
        && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

fn safe_manifest_token(value: &str, allow_dot: bool) -> bool {
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

fn safe_exec_path(value: &str) -> bool {
    if value.len() > 96 || value.contains("..") {
        return false;
    }
    if let Some(internal) = value.strip_prefix("internal:") {
        return safe_manifest_token(internal, false);
    }
    value.starts_with('/')
        && value.bytes().all(|byte| {
            (0x21..=0x7e).contains(&byte) && byte != b'\\' && byte != b'"' && byte != b'\''
        })
}

pub fn manifest_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
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

fn parse_manifest_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect()
}

fn default_exec_for_command(command: &str) -> String {
    match command {
        "editor" | "notes" | "trash" | "screenshot" | "guidemo" => {
            let mut path = String::from("/bin/");
            path.push_str(command);
            path
        }
        _ => {
            let mut path = String::from("internal:");
            path.push_str(command);
            path
        }
    }
}
