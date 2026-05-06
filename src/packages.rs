extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

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

static INSTALLED: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn init() {
    let _ = crate::vfs::vfs_kernel_create_dir("/APPS");
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
                let _ = crate::vfs::vfs_kernel_safe_write_file(&path, manifest.as_bytes());
            }
            None => {
                let _ = crate::vfs::vfs_kernel_create_file(&path);
                let _ = crate::vfs::vfs_kernel_write_file(&path, manifest.as_bytes());
            }
        }
    }
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
    if id_or_command.to_ascii_uppercase().ends_with(".PKG") || id_or_command.starts_with('/') {
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
    let _ = crate::vfs::vfs_kernel_create_file(&path);
    let _ = crate::vfs::vfs_kernel_safe_write_file(&path, manifest.as_bytes());
    crate::event_bus::emit("packages", "install", app.id);
    Ok(())
}

pub fn install_archive(path: &str) -> Result<(), &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package file not found")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package is not UTF-8 manifest")?;
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
    if crate::app_metadata::is_builtin_id(id)
        || crate::app_metadata::app_by_id_or_command(command).is_some()
        || crate::app_metadata::app_by_name(name).is_some()
    {
        return Err("package collides with built-in app");
    }
    if !id.starts_with("app.") || command.contains('/') || command.contains("..") {
        return Err("invalid package manifest");
    }
    if !exec_path.starts_with('/') || crate::vfs::vfs_kernel_read_file(exec_path).is_none() {
        return Err("package exec not found");
    }
    let dir = app_dir(command);
    let _ = crate::vfs::vfs_kernel_create_dir(&dir);
    let manifest_path = app_manifest_path(command);
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
    crate::app_metadata::validate_manifest_text(&manifest)?;
    let _ = crate::vfs::vfs_kernel_create_file(&manifest_path);
    crate::vfs::vfs_kernel_safe_write_file(&manifest_path, manifest.as_bytes())
        .map_err(|_| "install write failed")?;
    let mut installed = INSTALLED.lock();
    if !installed.iter().any(|existing| existing == id) {
        installed.push(String::from(id));
    }
    crate::event_bus::emit("packages", "install-pkg", id);
    crate::println!("[pkg] installed {} name={} exec={}", id, name, exec_path);
    Ok(())
}

pub fn uninstall(id_or_command: &str) -> Result<(), &'static str> {
    if let Some(app) = find_app(id_or_command) {
        INSTALLED.lock().retain(|id| id != app.id);
        let _ = delete_app_dir(app.command);
        crate::event_bus::emit("packages", "remove", app.id);
        crate::println!("[pkg] removed {}", app.id);
        return Ok(());
    }
    let manifest = crate::app_metadata::installed_manifest_by_id_or_command(id_or_command)
        .ok_or("unknown package")?;
    INSTALLED
        .lock()
        .retain(|id| !id.eq_ignore_ascii_case(&manifest.id));
    delete_app_dir(&manifest.command).map_err(|_| "remove failed")?;
    crate::event_bus::emit("packages", "remove", &manifest.id);
    crate::println!("[pkg] removed {}", manifest.id);
    Ok(())
}

pub fn launch(id_or_command: &str, args: &[&str]) -> Result<PackageLaunch, &'static str> {
    let manifest = launch_manifest(id_or_command).ok_or("unknown package")?;
    if !is_installed(&manifest.id) {
        return Err("package not installed");
    }
    if !manifest.exec_path.starts_with('/') {
        return Err("package has no userspace executable");
    }
    let pid = crate::elf::spawn_elf_process_with_args(&manifest.exec_path, args)
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
        "editor" | "notes" | "trash" | "screenshot" | "guidemo" => {
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
