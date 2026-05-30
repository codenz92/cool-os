extern crate alloc;

use super::icons::*;
use super::*;
use alloc::{string::String, vec::Vec};

pub(super) struct StartSearchState {
    pub(super) query: String,
    pub(super) focused: bool,
    pub(super) selected: usize,
    pub(super) show_all: bool,
}

#[derive(Clone)]
pub(super) enum StartSearchKind {
    App(String),
    Path(String),
    Command(String),
    Inline(String),
}

#[derive(Clone)]
pub(super) struct StartSearchResult {
    pub(super) label: String,
    pub(super) detail: String,
    pub(super) kind: StartSearchKind,
    pub(super) score: usize,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct StartMenuEntry {
    pub(super) section: &'static str,
    pub(super) label: String,
    pub(super) detail: String,
    pub(super) kind: StartSearchKind,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(super) struct StartMenuQuickAction {
    pub(super) label: &'static str,
    pub(super) glyph: &'static str,
    pub(super) action: &'static str,
    pub(super) accent: u32,
}

#[derive(Clone, Copy)]
pub(super) struct StartPowerAction {
    pub(super) label: &'static str,
    pub(super) glyph: &'static str,
    pub(super) action: &'static str,
    pub(super) accent: u32,
}

#[derive(Clone, Copy)]
pub(super) enum Win7StartAction {
    App(&'static str),
    Path(&'static str),
}

#[derive(Clone, Copy)]
pub(super) struct Win7StartLink {
    pub(super) label: &'static str,
    pub(super) action: Win7StartAction,
}

#[derive(Clone, Copy)]
pub(super) struct Win7StartMenuLayout {
    pub(super) menu_x: i32,
    pub(super) menu_y: i32,
    pub(super) menu_w: i32,
    pub(super) menu_h: i32,
    pub(super) left_w: i32,
    pub(super) right_x: i32,
    pub(super) right_w: i32,
    pub(super) list_x: i32,
    pub(super) list_y: i32,
    pub(super) list_w: i32,
    pub(super) row_h: i32,
    pub(super) all_x: i32,
    pub(super) all_y: i32,
    pub(super) all_w: i32,
    pub(super) all_h: i32,
    pub(super) search_x: i32,
    pub(super) search_y: i32,
    pub(super) search_w: i32,
    pub(super) search_h: i32,
    pub(super) avatar_x: i32,
    pub(super) avatar_y: i32,
    pub(super) avatar_w: i32,
    pub(super) avatar_h: i32,
    pub(super) links_x: i32,
    pub(super) links_y: i32,
    pub(super) links_w: i32,
    pub(super) link_h: i32,
    pub(super) shutdown_x: i32,
    pub(super) shutdown_y: i32,
    pub(super) shutdown_w: i32,
    pub(super) shutdown_h: i32,
    pub(super) shutdown_arrow_x: i32,
    pub(super) shutdown_arrow_w: i32,
}

impl Win7StartMenuLayout {
    pub(super) fn contains(self, px: i32, py: i32) -> bool {
        rect_contains(self.menu_x, self.menu_y, self.menu_w, self.menu_h, px, py)
    }
}

pub(super) fn start_menu_results(state: &StartSearchState) -> Vec<StartSearchResult> {
    if state.show_all || !state.query.trim().is_empty() {
        start_search_matches(&state.query, state.show_all)
    } else {
        let mut results = Vec::new();
        for &app in win7_start_left_apps().iter() {
            results.push(StartSearchResult {
                label: String::from(app),
                detail: String::from("app"),
                kind: StartSearchKind::App(String::from(app)),
                score: 1,
            });
        }
        results
    }
}

pub(super) fn start_menu_result_at(
    layout: Win7StartMenuLayout,
    state: &StartSearchState,
    px: i32,
    py: i32,
) -> Option<StartSearchResult> {
    let results = start_menu_results(state);
    let visible_rows = start_menu_visible_rows(layout, results.len());
    if !rect_contains(
        layout.list_x,
        layout.list_y,
        layout.list_w,
        layout.row_h * visible_rows as i32,
        px,
        py,
    ) {
        return None;
    }
    let idx = ((py - layout.list_y) / layout.row_h) as usize;
    results.get(idx).cloned()
}

pub(super) fn start_menu_visible_rows(layout: Win7StartMenuLayout, result_count: usize) -> usize {
    let max_rows = ((layout.all_y - layout.list_y - 2).max(0) / layout.row_h.max(1)) as usize;
    result_count.min(max_rows)
}

pub(super) fn start_search_matches(query: &str, show_all: bool) -> Vec<StartSearchResult> {
    let query = query.trim();
    let mut matches = Vec::new();

    for app in crate::app_metadata::APPS {
        if !crate::packages::is_installed(app.id) {
            continue;
        }
        let detail = app_search_detail(app);
        let mut score = if show_all { Some(1) } else { None };
        if let Some(app_score) = start_search_score(app.name, &detail, query) {
            score = Some(score.unwrap_or(0).max(app_score));
        }
        for alias in app.aliases {
            if let Some(alias_score) = start_search_score(alias, &detail, query) {
                score = Some(score.unwrap_or(0).max(alias_score.saturating_sub(1)));
            }
        }
        if let Some(score) = score {
            let exact_boost = if app.command.eq_ignore_ascii_case(query)
                || app.name.eq_ignore_ascii_case(query)
                || app
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(query))
            {
                30
            } else {
                0
            };
            push_unique_start_result(
                &mut matches,
                StartSearchResult {
                    label: String::from(app.name),
                    detail,
                    kind: StartSearchKind::App(String::from(app.name)),
                    score: score + exact_boost + recent_app_boost(app.name),
                },
            );
        }
    }

    for manifest in crate::app_metadata::installed_app_manifests() {
        if crate::app_metadata::is_builtin_id(&manifest.id)
            || !crate::packages::is_installed(&manifest.id)
        {
            continue;
        }
        let detail = manifest_search_detail(&manifest);
        let mut score = if show_all { Some(1) } else { None };
        if let Some(name_score) = start_search_score(&manifest.name, &detail, query) {
            score = Some(score.unwrap_or(0).max(name_score));
        }
        for alias in &manifest.aliases {
            if let Some(alias_score) = start_search_score(alias, &detail, query) {
                score = Some(score.unwrap_or(0).max(alias_score.saturating_sub(1)));
            }
        }
        if let Some(score) = score {
            let exact_boost = if manifest.command.eq_ignore_ascii_case(query)
                || manifest.name.eq_ignore_ascii_case(query)
                || manifest
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(query))
            {
                20
            } else {
                0
            };
            push_unique_start_result(
                &mut matches,
                StartSearchResult {
                    label: manifest.name.clone(),
                    detail,
                    kind: StartSearchKind::App(manifest.name.clone()),
                    score: score + exact_boost,
                },
            );
        }
    }

    for &entry in crate::app_metadata::LAUNCHER_ENTRIES.iter() {
        match entry.kind {
            crate::app_metadata::LauncherKind::App(app) => {
                if let Some(meta) = crate::app_metadata::app_by_name(app) {
                    if !crate::packages::is_installed(meta.id) {
                        continue;
                    }
                }
                let mut score = if show_all { Some(1) } else { None };
                if let Some(match_score) = start_search_score(entry.label, entry.detail, query) {
                    score = Some(score.unwrap_or(0).max(match_score));
                }
                if let Some(score) = score {
                    push_unique_start_result(
                        &mut matches,
                        StartSearchResult {
                            label: String::from(entry.label),
                            detail: String::from(entry.detail),
                            kind: StartSearchKind::App(String::from(app)),
                            score,
                        },
                    );
                }
            }
            crate::app_metadata::LauncherKind::Path(path) => {
                if query.is_empty() {
                    continue;
                }
                if let Some(score) = start_search_score(entry.label, entry.detail, query) {
                    push_unique_start_result(
                        &mut matches,
                        StartSearchResult {
                            label: String::from(entry.label),
                            detail: String::from(entry.detail),
                            kind: StartSearchKind::Path(String::from(path)),
                            score,
                        },
                    );
                }
            }
            crate::app_metadata::LauncherKind::Command => {}
        }
    }

    if !query.is_empty() {
        for app in crate::app_lifecycle::recent_apps().iter() {
            if let Some(score) = start_search_score(app, "recent app", query) {
                push_unique_start_result(
                    &mut matches,
                    StartSearchResult {
                        label: app.clone(),
                        detail: String::from("recent app"),
                        kind: StartSearchKind::App(app.clone()),
                        score: score + 10,
                    },
                );
            }
        }
        for file in crate::app_lifecycle::recent_files().iter() {
            if let Some(score) = start_search_score(file, "recent file", query) {
                push_unique_start_result(
                    &mut matches,
                    StartSearchResult {
                        label: file_name(file),
                        detail: String::from("recent file"),
                        kind: StartSearchKind::Path(file.clone()),
                        score: score + 3,
                    },
                );
            }
        }
        for entry in crate::search_index::search(query, 12).iter() {
            push_unique_start_result(
                &mut matches,
                StartSearchResult {
                    label: entry.name.clone(),
                    detail: entry.path.clone(),
                    kind: StartSearchKind::Path(entry.path.clone()),
                    score: crate::search_index::fuzzy_score(&entry.name, query).unwrap_or(1) + 2,
                },
            );
        }
    }

    matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));
    matches.truncate(48);
    matches
}

pub(super) fn push_unique_start_result(
    results: &mut Vec<StartSearchResult>,
    candidate: StartSearchResult,
) {
    if let Some(existing) = results
        .iter_mut()
        .find(|entry| start_search_kind_eq(&entry.kind, &candidate.kind))
    {
        if candidate.score > existing.score {
            existing.score = candidate.score;
            existing.label = candidate.label;
            existing.detail = candidate.detail;
        }
    } else {
        results.push(candidate);
    }
}

pub(super) fn start_search_kind_eq(a: &StartSearchKind, b: &StartSearchKind) -> bool {
    match (a, b) {
        (StartSearchKind::App(a), StartSearchKind::App(b)) => a.eq_ignore_ascii_case(b),
        (StartSearchKind::Path(a), StartSearchKind::Path(b)) => a == b,
        (StartSearchKind::Command(a), StartSearchKind::Command(b)) => a == b,
        (StartSearchKind::Inline(a), StartSearchKind::Inline(b)) => a == b,
        _ => false,
    }
}

pub(super) fn start_search_icon_kind(result: &StartSearchResult) -> DesktopIconKind {
    match &result.kind {
        StartSearchKind::App(app) => desktop_icon_kind(app),
        StartSearchKind::Path(_) => DesktopIconKind::FileManager,
        StartSearchKind::Command(_) => DesktopIconKind::Terminal,
        StartSearchKind::Inline(action) if action.starts_with("settings:") => {
            DesktopIconKind::DisplaySettings
        }
        StartSearchKind::Inline(_) => DesktopIconKind::Generic,
    }
}

pub(super) fn start_search_score(label: &str, detail: &str, query: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(1);
    }
    let label_score = crate::search_index::fuzzy_score(label, query).unwrap_or(0);
    let detail_score = crate::search_index::fuzzy_score(detail, query)
        .unwrap_or(0)
        .saturating_sub(3);
    let score = label_score.max(detail_score);
    if score == 0 {
        None
    } else {
        Some(score)
    }
}

pub(super) fn app_search_detail(app: &crate::app_metadata::AppMetadata) -> String {
    let mut detail = String::from(app.category.label());
    detail.push_str(" app, permission ");
    detail.push_str(app.permission);
    if !app.associations.is_empty() {
        detail.push_str(", opens ");
        for (idx, assoc) in app.associations.iter().enumerate() {
            if idx > 0 {
                detail.push(',');
            }
            detail.push_str(assoc);
        }
    }
    detail
}

pub(super) fn manifest_search_detail(manifest: &crate::app_metadata::AppManifest) -> String {
    let mut detail = String::from("/APPS manifest ");
    detail.push_str(&manifest.icon);
    detail.push(' ');
    detail.push_str(&manifest.category);
    detail.push_str(", version ");
    detail.push_str(&manifest.version);
    detail.push_str(", permission ");
    detail.push_str(&manifest.permission);
    detail.push_str(", command ");
    detail.push_str(&manifest.command);
    detail.push_str(", exec ");
    detail.push_str(&manifest.exec_path);
    detail.push_str(", id ");
    detail.push_str(&manifest.id);
    if !manifest.associations.is_empty() {
        detail.push_str(", opens ");
        for (idx, assoc) in manifest.associations.iter().enumerate() {
            if idx > 0 {
                detail.push(',');
            }
            detail.push_str(assoc);
        }
    }
    detail
}

pub(super) fn recent_app_boost(app: &str) -> usize {
    crate::app_lifecycle::recent_apps()
        .iter()
        .position(|recent| recent.eq_ignore_ascii_case(app))
        .map(|idx| 10usize.saturating_sub(idx))
        .unwrap_or(0)
}

pub(super) fn settings_shortcuts() -> &'static [(&'static str, &'static str)] {
    &[
        ("Display settings", "desktop"),
        ("Accessibility settings", "accessibility"),
        ("Diagnostics settings", "diagnostics"),
        ("Network settings", "network"),
        ("Storage settings", "storage"),
        ("Accounts settings", "accounts"),
        ("Log viewer settings", "logs"),
        ("Power settings", "power"),
        ("Updates", "logs"),
    ]
}

pub(super) fn power_actions() -> &'static [(&'static str, &'static str)] {
    &[
        ("Shutdown", "shutdown"),
        ("Reboot", "reboot"),
        ("Sleep", "sleep"),
        ("Lock", "lock"),
        ("Logout", "logout"),
        ("Restart desktop", "restart-desktop"),
        ("Restore session", "restore-session"),
    ]
}

pub(super) fn win7_start_left_apps() -> &'static [&'static str; 9] {
    &[
        "Screenshot",
        "Welcome",
        "Display Settings",
        "System Monitor",
        "Notes",
        "Color Picker",
        "Text Viewer",
        "File Manager",
        "Web Browser",
    ]
}

pub(super) fn win7_start_right_links() -> &'static [Win7StartLink; 8] {
    &[
        Win7StartLink {
            label: "Documents",
            action: Win7StartAction::Path("/Documents"),
        },
        Win7StartLink {
            label: "Pictures",
            action: Win7StartAction::Path("/Pictures"),
        },
        Win7StartLink {
            label: "Music",
            action: Win7StartAction::Path("/Music"),
        },
        Win7StartLink {
            label: "Computer",
            action: Win7StartAction::Path("/"),
        },
        Win7StartLink {
            label: "Control Panel",
            action: Win7StartAction::App("Display Settings"),
        },
        Win7StartLink {
            label: "Devices and Printers",
            action: Win7StartAction::App("System Monitor"),
        },
        Win7StartLink {
            label: "Default Programs",
            action: Win7StartAction::App("Display Settings"),
        },
        Win7StartLink {
            label: "Help and Support",
            action: Win7StartAction::App("Welcome"),
        },
    ]
}

pub(super) fn win7_start_menu_layout(sw: i32, taskbar_y: i32) -> Win7StartMenuLayout {
    let max_w = (sw - 8).max(START_MENU_WIN7_MIN_W);
    let max_h = (taskbar_y - 4).max(START_MENU_WIN7_MIN_H);
    let menu_w = START_MENU_WIN7_W.min(max_w).max(START_MENU_WIN7_MIN_W);
    let menu_h = START_MENU_WIN7_H.min(max_h).max(START_MENU_WIN7_MIN_H);
    let menu_x = 0i32;
    let menu_y = (taskbar_y - menu_h).max(0);
    let right_w = START_MENU_WIN7_RIGHT_W
        .min(menu_w / 2)
        .max(132)
        .min(menu_w - 190);
    let left_w = menu_w - right_w;
    let bottom_y = menu_y + menu_h - START_MENU_WIN7_BOTTOM_H;
    let list_x = menu_x + 8;
    let list_y = menu_y + 10;
    let list_w = left_w - 16;
    let search_h = 26i32;
    let search_x = menu_x + 10;
    let search_y = bottom_y + 14;
    let search_w = (left_w - 20).max(120);
    let all_h = START_MENU_WIN7_ROW_H;
    let all_x = menu_x + 8;
    let all_y = bottom_y - all_h;
    let all_w = left_w - 16;
    let right_x = menu_x + left_w;
    let links_x = right_x + 6;
    let avatar_w = 58i32;
    let avatar_h = 58i32;
    let avatar_x = right_x + (right_w - avatar_w) / 2;
    let avatar_y = menu_y - 26;
    let links_y = menu_y + 50;
    let shutdown_w = 108i32;
    let shutdown_h = 22i32;
    let shutdown_x = right_x + (right_w - shutdown_w) / 2;
    let shutdown_y = bottom_y + 16;
    let shutdown_arrow_w = 22i32;

    Win7StartMenuLayout {
        menu_x,
        menu_y,
        menu_w,
        menu_h,
        left_w,
        right_x,
        right_w,
        list_x,
        list_y,
        list_w,
        row_h: START_MENU_WIN7_ROW_H,
        all_x,
        all_y,
        all_w,
        all_h,
        search_x,
        search_y,
        search_w,
        search_h,
        avatar_x,
        avatar_y,
        avatar_w,
        avatar_h,
        links_x,
        links_y,
        links_w: right_w - 12,
        link_h: START_MENU_WIN7_LINK_H,
        shutdown_x,
        shutdown_y,
        shutdown_w,
        shutdown_h,
        shutdown_arrow_x: shutdown_x + shutdown_w - shutdown_arrow_w,
        shutdown_arrow_w,
    }
}

pub(super) fn win7_start_right_action_at(
    layout: Win7StartMenuLayout,
    px: i32,
    py: i32,
) -> Option<Win7StartAction> {
    let link_count = win7_start_right_links().len() + 1;
    if !rect_contains(
        layout.links_x,
        layout.links_y,
        layout.links_w,
        layout.link_h * link_count as i32,
        px,
        py,
    ) {
        return None;
    }
    let idx = ((py - layout.links_y) / layout.link_h) as usize;
    if idx == 0 {
        Some(Win7StartAction::App("Accounts"))
    } else {
        win7_start_right_links()
            .get(idx - 1)
            .map(|link| link.action)
    }
}

pub(super) fn start_menu_quick_actions() -> &'static [StartMenuQuickAction] {
    &[
        StartMenuQuickAction {
            label: "Terminal",
            glyph: "TR",
            action: "app:Terminal",
            accent: 0x00_00_FF_88,
        },
        StartMenuQuickAction {
            label: "Files",
            glyph: "FS",
            action: "path:/",
            accent: 0x00_55_DD_FF,
        },
        StartMenuQuickAction {
            label: "Settings",
            glyph: "DS",
            action: "settings:desktop",
            accent: 0x00_66_CC_FF,
        },
        StartMenuQuickAction {
            label: "Lock",
            glyph: "LK",
            action: "lock",
            accent: 0x00_FF_DD_55,
        },
    ]
}

pub(super) fn start_power_actions() -> &'static [StartPowerAction] {
    &[
        StartPowerAction {
            label: "Sleep",
            glyph: "SL",
            action: "sleep",
            accent: 0x00_66_CC_FF,
        },
        StartPowerAction {
            label: "Lock",
            glyph: "LK",
            action: "lock",
            accent: 0x00_FF_DD_55,
        },
        StartPowerAction {
            label: "Shutdown",
            glyph: "PW",
            action: "shutdown",
            accent: 0x00_FF_88_66,
        },
        StartPowerAction {
            label: "Restart",
            glyph: "RS",
            action: "restart",
            accent: 0x00_AA_DD_FF,
        },
    ]
}

pub(super) fn start_menu_quick_action_rect(index: usize, banner_w: i32) -> (i32, i32, i32, i32) {
    let gap = 5i32;
    let cols = 2i32;
    let tile_w = ((banner_w - 16 - gap) / cols).max(72);
    let tile_h = 22i32;
    let col = index as i32 % cols;
    let row = index as i32 / cols;
    let x = 8 + col * (tile_w + gap);
    let y = 58 + row * (tile_h + gap);
    (x, y, tile_w, tile_h)
}

#[allow(dead_code)]
pub(super) fn start_menu_quick_action_at(
    rel_x: i32,
    rel_y: i32,
    banner_w: i32,
) -> Option<&'static str> {
    for (idx, action) in start_menu_quick_actions().iter().enumerate() {
        let (x, y, w, h) = start_menu_quick_action_rect(idx, banner_w);
        if rel_x >= x && rel_x < x + w && rel_y >= y && rel_y < y + h {
            return Some(action.action);
        }
    }
    None
}

pub(super) fn start_power_menu_height() -> i32 {
    START_POWER_MENU_PAD * 2 + start_power_actions().len() as i32 * START_POWER_MENU_ROW_H
}

pub(super) fn start_power_menu_rect(
    button_x: i32,
    button_y: i32,
    button_w: i32,
    menu_x: i32,
    menu_y: i32,
    menu_w: i32,
) -> (i32, i32, i32, i32) {
    let w = START_POWER_MENU_W.min(menu_w - 16).max(112);
    let h = start_power_menu_height();
    let x = (button_x + button_w - w)
        .min(menu_x + menu_w - w - 8)
        .max(menu_x + 8);
    let y = (button_y - h - 6).max(menu_y + 8);
    (x, y, w, h)
}

pub(super) fn start_power_action_at(
    px: i32,
    py: i32,
    menu_x: i32,
    menu_y: i32,
    menu_w: i32,
) -> Option<&'static str> {
    if px < menu_x + 3 || px >= menu_x + menu_w - 3 {
        return None;
    }
    let rel_y = py - menu_y - START_POWER_MENU_PAD;
    if rel_y < 0 {
        return None;
    }
    let idx = (rel_y / START_POWER_MENU_ROW_H) as usize;
    if idx >= start_power_actions().len() {
        return None;
    }
    let row_y = menu_y + START_POWER_MENU_PAD + idx as i32 * START_POWER_MENU_ROW_H;
    if py < row_y || py >= row_y + START_POWER_MENU_ROW_H {
        return None;
    }
    Some(start_power_actions()[idx].action)
}

pub(super) fn settings_action(page: &str) -> String {
    let mut action = String::from("settings:");
    action.push_str(page);
    action
}

pub(super) fn build_start_menu_entries() -> Vec<StartMenuEntry> {
    let prefs = crate::app_lifecycle::start_menu_prefs();
    let mut out = Vec::new();
    if prefs.show_recent {
        for app in crate::app_lifecycle::recent_apps().iter().take(3) {
            out.push(StartMenuEntry {
                section: "RECENT",
                label: app.clone(),
                detail: String::from("app"),
                kind: StartSearchKind::App(app.clone()),
            });
        }
        for file in crate::app_lifecycle::recent_files().iter().take(3) {
            out.push(StartMenuEntry {
                section: "RECENT",
                label: file_name(file),
                detail: String::from("file"),
                kind: StartSearchKind::Path(file.clone()),
            });
        }
        for command in crate::app_lifecycle::recent_commands().iter().take(2) {
            out.push(StartMenuEntry {
                section: "RECENT",
                label: command.clone(),
                detail: String::from("cmd"),
                kind: StartSearchKind::Command(command.clone()),
            });
        }
    }

    for &place in FileManagerApp::START_MENU_LINKS.iter().take(4) {
        out.push(StartMenuEntry {
            section: "PLACES",
            label: String::from(place),
            detail: String::from("folder"),
            kind: StartSearchKind::Path(FileManagerApp::shell_link_path(place)),
        });
    }

    for shortcut in settings_shortcuts().iter().take(7) {
        out.push(StartMenuEntry {
            section: "SETTINGS",
            label: String::from(shortcut.0),
            detail: String::from("page"),
            kind: StartSearchKind::Inline(settings_action(shortcut.1)),
        });
    }
    for category in crate::app_metadata::APP_CATEGORIES.iter().take(6) {
        let mut action = String::from("category:");
        action.push_str(category.label());
        out.push(StartMenuEntry {
            section: "CATEGORIES",
            label: {
                let mut label = String::from(category.label());
                label.push_str(" apps");
                label
            },
            detail: String::from("apps"),
            kind: StartSearchKind::Inline(action),
        });
    }
    for action in power_actions().iter().take(5) {
        out.push(StartMenuEntry {
            section: "POWER",
            label: String::from(action.0),
            detail: String::from("session"),
            kind: StartSearchKind::Inline(String::from(action.1)),
        });
    }
    out
}

#[allow(dead_code)]
pub(super) fn start_menu_pinned_limit(
    menu_h: i32,
    bottom_h: i32,
    left_hdr_h: i32,
    item_h: i32,
) -> usize {
    ((menu_h - bottom_h - left_hdr_h - item_h - 18).max(0) / item_h.max(1)) as usize
}

#[allow(dead_code)]
pub(super) fn start_menu_entry_at(
    entries: &[StartMenuEntry],
    rel_y: i32,
    item_h: i32,
    max_h: i32,
) -> Option<usize> {
    if rel_y < 0 {
        return None;
    }
    let mut y = 0i32;
    let mut last_section = "";
    for (idx, entry) in entries.iter().enumerate() {
        if entry.section != last_section {
            if y + START_MENU_SECTION_H > max_h {
                return None;
            }
            if rel_y >= y && rel_y < y + START_MENU_SECTION_H {
                return None;
            }
            y += START_MENU_SECTION_H;
            last_section = entry.section;
        }
        if y + item_h > max_h {
            return None;
        }
        if rel_y >= y && rel_y < y + item_h {
            return Some(idx);
        }
        y += item_h;
    }
    None
}

pub(super) fn start_item_kind(item: &str) -> StartSearchKind {
    if let Some(path) = item.strip_prefix("path:") {
        StartSearchKind::Path(String::from(path.trim()))
    } else if let Some(command) = item.strip_prefix("cmd:") {
        StartSearchKind::Command(String::from(command.trim()))
    } else if let Some(action) = item.strip_prefix("setting:") {
        StartSearchKind::Inline(settings_action(action.trim()))
    } else if let Some(action) = item.strip_prefix("inline:") {
        StartSearchKind::Inline(String::from(action.trim()))
    } else if item.starts_with('/') {
        StartSearchKind::Path(String::from(item))
    } else if crate::app_metadata::app_by_id_or_command(item).is_some()
        || crate::app_metadata::app_by_name(item).is_some()
    {
        let app = crate::app_metadata::app_by_id_or_command(item)
            .or_else(|| crate::app_metadata::app_by_name(item));
        StartSearchKind::App(String::from(app.map(|meta| meta.name).unwrap_or(item)))
    } else {
        StartSearchKind::App(String::from(item))
    }
}

#[allow(dead_code)]
pub(super) fn start_item_label(item: &str) -> String {
    if let Some(path) = item.strip_prefix("path:") {
        file_name(path.trim())
    } else if let Some(command) = item.strip_prefix("cmd:") {
        let mut label = String::from("Run ");
        label.push_str(command.trim());
        label
    } else if let Some(page) = item.strip_prefix("setting:") {
        let mut label = String::from(page.trim());
        label.push_str(" settings");
        label
    } else if let Some(action) = item.strip_prefix("inline:") {
        String::from(action.trim())
    } else {
        String::from(item)
    }
}

#[allow(dead_code)]
pub(super) fn start_item_detail(item: &str) -> String {
    match start_item_kind(item) {
        StartSearchKind::App(app) => crate::app_metadata::app_by_id_or_command(&app)
            .or_else(|| crate::app_metadata::app_by_name(&app))
            .map(|meta| {
                let mut detail = String::from(meta.category.label());
                detail.push_str(" app");
                detail
            })
            .unwrap_or_else(|| String::from("app")),
        StartSearchKind::Path(path) => {
            if path.ends_with('/') || path == "/" {
                String::from("folder")
            } else {
                String::from("file or folder")
            }
        }
        StartSearchKind::Command(_) => String::from("terminal command"),
        StartSearchKind::Inline(action) if action.starts_with("settings:") => {
            String::from("settings page")
        }
        StartSearchKind::Inline(action)
            if action == "lock"
                || action == "logout"
                || action == "sleep"
                || action == "shutdown"
                || action == "reboot"
                || action == "restart" =>
        {
            String::from("session action")
        }
        StartSearchKind::Inline(_) => String::from("quick action"),
    }
}

pub(super) fn parent_path(path: &str) -> &str {
    if path == "/" {
        return "/";
    }
    let trimmed = path.trim_end_matches('/');
    match trimmed.rsplit_once('/') {
        Some(("", _)) | None => "/",
        Some((parent, _)) => parent,
    }
}
