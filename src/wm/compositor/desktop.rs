extern crate alloc;

use super::primitives::*;
use super::*;
use alloc::string::String;

// ── Context menu ──────────────────────────────────────────────────────────────

pub(super) const CTX_W: i32 = 236;
pub(super) const CTX_SUB_W: i32 = 220;
pub(super) const CTX_ITEM_H: i32 = 30;
pub(super) const CTX_SEP_H: i32 = 9;
pub(super) const CTX_HEADER_H: i32 = 0;
pub(super) const CTX_PAD: i32 = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DesktopContextCommand {
    ToggleDesktopIcons,
    ToggleCompactSpacing,
    SortByName,
    SortByType,
    Refresh,
    CreateFolder,
    CreateTextDocument,
    DisplaySettings,
    Personalize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DesktopContextSubmenu {
    View,
    SortBy,
    New,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextEntryKind {
    Action(DesktopContextCommand),
    Submenu(DesktopContextSubmenu),
    Separator,
}

#[derive(Clone, Copy)]
pub(super) struct ContextEntryDef {
    pub(super) label: &'static str,
    pub(super) kind: ContextEntryKind,
    pub(super) enabled: bool,
}

pub(super) const DESKTOP_CONTEXT_MENU: &[ContextEntryDef] = &[
    ContextEntryDef {
        label: "View",
        kind: ContextEntryKind::Submenu(DesktopContextSubmenu::View),
        enabled: true,
    },
    ContextEntryDef {
        label: "Sort by",
        kind: ContextEntryKind::Submenu(DesktopContextSubmenu::SortBy),
        enabled: true,
    },
    ContextEntryDef {
        label: "Refresh",
        kind: ContextEntryKind::Action(DesktopContextCommand::Refresh),
        enabled: true,
    },
    ContextEntryDef {
        label: "",
        kind: ContextEntryKind::Separator,
        enabled: false,
    },
    ContextEntryDef {
        label: "Paste",
        kind: ContextEntryKind::Action(DesktopContextCommand::Refresh),
        enabled: false,
    },
    ContextEntryDef {
        label: "Paste shortcut",
        kind: ContextEntryKind::Action(DesktopContextCommand::Refresh),
        enabled: false,
    },
    ContextEntryDef {
        label: "",
        kind: ContextEntryKind::Separator,
        enabled: false,
    },
    ContextEntryDef {
        label: "New",
        kind: ContextEntryKind::Submenu(DesktopContextSubmenu::New),
        enabled: true,
    },
    ContextEntryDef {
        label: "",
        kind: ContextEntryKind::Separator,
        enabled: false,
    },
    ContextEntryDef {
        label: "Display settings",
        kind: ContextEntryKind::Action(DesktopContextCommand::DisplaySettings),
        enabled: true,
    },
    ContextEntryDef {
        label: "Personalize",
        kind: ContextEntryKind::Action(DesktopContextCommand::Personalize),
        enabled: true,
    },
];

pub(super) const CTX_VIEW_MENU: &[ContextEntryDef] = &[
    ContextEntryDef {
        label: "Show desktop icons",
        kind: ContextEntryKind::Action(DesktopContextCommand::ToggleDesktopIcons),
        enabled: true,
    },
    ContextEntryDef {
        label: "Compact icon spacing",
        kind: ContextEntryKind::Action(DesktopContextCommand::ToggleCompactSpacing),
        enabled: true,
    },
];

pub(super) const CTX_SORT_MENU: &[ContextEntryDef] = &[
    ContextEntryDef {
        label: "Name",
        kind: ContextEntryKind::Action(DesktopContextCommand::SortByName),
        enabled: true,
    },
    ContextEntryDef {
        label: "Type",
        kind: ContextEntryKind::Action(DesktopContextCommand::SortByType),
        enabled: true,
    },
];

pub(super) const CTX_NEW_MENU: &[ContextEntryDef] = &[
    ContextEntryDef {
        label: "Folder",
        kind: ContextEntryKind::Action(DesktopContextCommand::CreateFolder),
        enabled: true,
    },
    ContextEntryDef {
        label: "Text Document",
        kind: ContextEntryKind::Action(DesktopContextCommand::CreateTextDocument),
        enabled: true,
    },
];

pub(super) struct ContextMenu {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) submenu: Option<DesktopContextSubmenu>,
}

// ── Desktop icons ──────────────────────────────────────────────────────────────

pub(super) const ICON_SIZE: i32 = 52;
pub(super) const ICON_LABEL_H: i32 = 14;

#[derive(Clone, Copy)]
pub(super) struct DesktopIconSpec {
    pub(super) label: &'static str,
    pub(super) app: &'static str,
    pub(super) type_rank: u8,
}

pub(super) const DESKTOP_ICON_SPECS: [DesktopIconSpec; 9] = [
    DesktopIconSpec {
        label: "Terminal",
        app: "Terminal",
        type_rank: 2,
    },
    DesktopIconSpec {
        label: "Monitor",
        app: "System Mon",
        type_rank: 1,
    },
    DesktopIconSpec {
        label: "Files",
        app: "File Manager",
        type_rank: 0,
    },
    DesktopIconSpec {
        label: "Viewer",
        app: "Text Viewer",
        type_rank: 3,
    },
    DesktopIconSpec {
        label: "Browser",
        app: "Web Browser",
        type_rank: 1,
    },
    DesktopIconSpec {
        label: "Colors",
        app: "Color Pick",
        type_rank: 4,
    },
    DesktopIconSpec {
        label: "Notes",
        app: "Notes",
        type_rank: 3,
    },
    DesktopIconSpec {
        label: "Shot",
        app: "Screenshot",
        type_rank: 4,
    },
    DesktopIconSpec {
        label: "Trash",
        app: "Trash Bin",
        type_rank: 0,
    },
];

pub(super) struct DesktopIcon {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) label: &'static str,
    pub(super) app: &'static str,
}

impl DesktopIcon {
    pub(super) fn hit(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + ICON_SIZE
            && py >= self.y
            && py < self.y + ICON_SIZE + ICON_LABEL_H
    }
}

pub(super) fn ctx_submenu_entries(submenu: DesktopContextSubmenu) -> &'static [ContextEntryDef] {
    match submenu {
        DesktopContextSubmenu::View => CTX_VIEW_MENU,
        DesktopContextSubmenu::SortBy => CTX_SORT_MENU,
        DesktopContextSubmenu::New => CTX_NEW_MENU,
    }
}

pub(super) fn ctx_entry_h(entry: ContextEntryDef) -> i32 {
    match entry.kind {
        ContextEntryKind::Separator => CTX_SEP_H,
        _ => CTX_ITEM_H,
    }
}

pub(super) fn ctx_menu_height(entries: &[ContextEntryDef]) -> i32 {
    CTX_HEADER_H + CTX_PAD * 2 + entries.iter().map(|entry| ctx_entry_h(*entry)).sum::<i32>()
}

pub(super) fn ctx_entry_y(entries: &[ContextEntryDef], menu_y: i32, target_idx: usize) -> i32 {
    let mut y = menu_y + CTX_HEADER_H + CTX_PAD;
    for (idx, entry) in entries.iter().enumerate() {
        if idx == target_idx {
            return y;
        }
        y += ctx_entry_h(*entry);
    }
    y
}

pub(super) fn ctx_menu_hit_index(
    entries: &[ContextEntryDef],
    menu_x: i32,
    menu_y: i32,
    menu_w: i32,
    px: i32,
    py: i32,
) -> Option<usize> {
    if px < menu_x
        || px >= menu_x + menu_w
        || py < menu_y
        || py >= menu_y + ctx_menu_height(entries)
    {
        return None;
    }

    let mut y = menu_y + CTX_HEADER_H + CTX_PAD;
    for (idx, entry) in entries.iter().enumerate() {
        let h = ctx_entry_h(*entry);
        if py >= y && py < y + h {
            return match entry.kind {
                ContextEntryKind::Separator => None,
                _ => Some(idx),
            };
        }
        y += h;
    }
    None
}

pub(super) fn ctx_submenu_rect(
    menu_x: i32,
    menu_y: i32,
    submenu: DesktopContextSubmenu,
    sw: i32,
    taskbar_y: i32,
) -> (i32, i32, i32, i32) {
    let parent_idx = DESKTOP_CONTEXT_MENU
        .iter()
        .position(|entry| entry.kind == ContextEntryKind::Submenu(submenu))
        .unwrap_or(0);
    let entries = ctx_submenu_entries(submenu);
    let h = ctx_menu_height(entries);
    let parent_y = ctx_entry_y(DESKTOP_CONTEXT_MENU, menu_y, parent_idx);
    let mut x = menu_x + CTX_W - 6;
    if x + CTX_SUB_W > sw {
        x = (menu_x - CTX_SUB_W + 6).max(0);
    }
    let mut y = (parent_y - 4).max(0);
    if y + h > taskbar_y {
        y = (taskbar_y - h).max(0);
    }
    (x, y, CTX_SUB_W, h)
}

pub(super) fn create_root_item(
    prefix: &str,
    ext: Option<&str>,
    is_dir: bool,
) -> Result<String, crate::fat32::FsError> {
    let entries = crate::vfs::vfs_list_dir("/").unwrap_or_default();
    for n in 1..10_000usize {
        let mut name = String::from(prefix);
        push_decimal(&mut name, n as u64);
        if let Some(ext) = ext {
            name.push('.');
            name.push_str(ext);
        }
        if entries
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(&name))
        {
            continue;
        }
        let mut path = String::from("/");
        path.push_str(&name);
        if is_dir {
            crate::vfs::vfs_create_dir(&path)?;
        } else {
            crate::vfs::vfs_create_file(&path)?;
        }
        return Ok(path);
    }
    Err(crate::fat32::FsError::NoSpace)
}

pub(super) fn push_decimal(out: &mut String, mut n: u64) {
    if n == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    while n > 0 {
        digits[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for idx in (0..len).rev() {
        out.push(digits[idx] as char);
    }
}

#[derive(Clone, Copy)]
pub(super) struct WallpaperPalette {
    pub(super) tl: u32,
    pub(super) tr: u32,
    pub(super) bl: u32,
    pub(super) br: u32,
    pub(super) bloom: u32,
    pub(super) surface: u32,
}

pub(super) fn wallpaper_palette(preset: WallpaperPreset) -> WallpaperPalette {
    match preset {
        WallpaperPreset::Phosphor => WallpaperPalette {
            tl: DESK_TL,
            tr: DESK_TR,
            bl: DESK_BL,
            br: DESK_BR,
            bloom: BLOOM_1,
            surface: 0x00_24_31_3E,
        },
        WallpaperPreset::Aurora => WallpaperPalette {
            tl: 0x00_0A_14_18,
            tr: 0x00_10_1B_20,
            bl: 0x00_04_0F_12,
            br: 0x00_12_1E_24,
            bloom: 0x00_35_C7_AE,
            surface: 0x00_29_44_48,
        },
        WallpaperPreset::Midnight => WallpaperPalette {
            tl: 0x00_13_10_20,
            tr: 0x00_0E_12_22,
            bl: 0x00_07_0A_14,
            br: 0x00_17_12_1A,
            bloom: 0x00_66_7A_DA,
            surface: 0x00_2F_2B_43,
        },
    }
}

pub(super) fn build_wallpaper(
    w: usize,
    taskbar_y: usize,
    preset: WallpaperPreset,
    show_progress: bool,
) -> Vec<u32> {
    let palette = wallpaper_palette(preset);
    if show_progress {
        crate::boot_splash::show(
            "allocating desktop buffers",
            15,
            crate::boot_splash::BOOT_PROGRESS_TOTAL,
        );
    }
    let mut wallpaper = alloc::vec![0u32; w * crate::framebuffer::height()];
    if show_progress {
        crate::boot_splash::show(
            "painting desktop background",
            16,
            crate::boot_splash::BOOT_PROGRESS_TOTAL,
        );
    }

    if w > 0 && taskbar_y > 0 {
        let (fw, fh) = (w as f32, taskbar_y as f32);
        let band_mark = taskbar_y / 3;
        let texture_mark = taskbar_y * 2 / 3;
        let mut band_stage_shown = false;
        let mut texture_stage_shown = false;
        let noise_seed = match preset {
            WallpaperPreset::Phosphor => 0xC001_D00D,
            WallpaperPreset::Aurora => 0xA11E_7A1A,
            WallpaperPreset::Midnight => 0x0B5C_0DED,
        };
        for y in 0..taskbar_y {
            if show_progress && !band_stage_shown && y >= band_mark {
                crate::boot_splash::show(
                    "adding soft desktop bands",
                    17,
                    crate::boot_splash::BOOT_PROGRESS_TOTAL,
                );
                band_stage_shown = true;
            }
            if show_progress && !texture_stage_shown && y >= texture_mark {
                crate::boot_splash::show(
                    "adding desktop surface texture",
                    18,
                    crate::boot_splash::BOOT_PROGRESS_TOTAL,
                );
                texture_stage_shown = true;
            }

            let ty = y as f32 / fh;
            for x in 0..w {
                let tx = x as f32 / fw;
                let r = bilinear_u8(
                    (palette.tl >> 16) as u8,
                    (palette.tr >> 16) as u8,
                    (palette.bl >> 16) as u8,
                    (palette.br >> 16) as u8,
                    tx,
                    ty,
                );
                let g = bilinear_u8(
                    (palette.tl >> 8) as u8,
                    (palette.tr >> 8) as u8,
                    (palette.bl >> 8) as u8,
                    (palette.br >> 8) as u8,
                    tx,
                    ty,
                );
                let b = bilinear_u8(
                    palette.tl as u8,
                    palette.tr as u8,
                    palette.bl as u8,
                    palette.br as u8,
                    tx,
                    ty,
                );
                let diag = tx * 0.72 + ty * 0.42;
                let band_delta = if diag >= 0.58 {
                    diag - 0.58
                } else {
                    0.58 - diag
                };
                let band = 1.0f32 - (band_delta / 0.34f32).min(1.0f32);
                let band = band * band * 0.38f32;
                let secondary_delta = if (tx - ty) >= 0.18 {
                    (tx - ty) - 0.18
                } else {
                    0.18 - (tx - ty)
                };
                let secondary = 1.0f32 - (secondary_delta / 0.44f32).min(1.0f32);
                let secondary = secondary * secondary * 0.16f32;
                let vignette =
                    1.0f32 - ((tx - 0.48) * (tx - 0.48) + (ty - 0.52) * (ty - 0.52)) * 0.55;
                let shade = vignette.max(0.74f32).min(1.0f32);

                let br = (r as f32 * shade
                    + band * ((palette.bloom >> 16) as u8 as f32)
                    + secondary * ((palette.surface >> 16) as u8 as f32))
                    .min(255.0) as i32;
                let bg = (g as f32 * shade
                    + band * ((palette.bloom >> 8) as u8 as f32)
                    + secondary * ((palette.surface >> 8) as u8 as f32))
                    .min(255.0) as i32;
                let bb = (b as f32 * shade
                    + band * (palette.bloom as u8 as f32)
                    + secondary * (palette.surface as u8 as f32))
                    .min(255.0) as i32;

                let noise = (((x as u32).wrapping_mul(73_856_093)
                    ^ (y as u32).wrapping_mul(19_349_663)
                    ^ noise_seed)
                    & 7) as i32
                    - 3;
                let fr = (br + noise).clamp(0, 255) as u32;
                let fg = (bg + noise).clamp(0, 255) as u32;
                let fb = (bb + noise).clamp(0, 255) as u32;
                wallpaper[y * w + x] = (fr << 16) | (fg << 8) | fb;
            }
        }
    }

    if show_progress {
        crate::boot_splash::show(
            "finishing wallpaper",
            19,
            crate::boot_splash::BOOT_PROGRESS_TOTAL,
        );
    }

    wallpaper
}

pub(super) fn draw_desktop_context_menu(
    s: &mut [u32],
    sw: usize,
    cm: &ContextMenu,
    mx: i32,
    my: i32,
    show_desktop_icons: bool,
    compact_spacing: bool,
    desktop_sort: DesktopSortMode,
    screen_w: i32,
    taskbar_y: i32,
) {
    draw_context_panel(
        s,
        sw,
        cm.x,
        cm.y,
        CTX_W,
        DESKTOP_CONTEXT_MENU,
        ctx_menu_hit_index(DESKTOP_CONTEXT_MENU, cm.x, cm.y, CTX_W, mx, my),
        None,
        show_desktop_icons,
        compact_spacing,
        desktop_sort,
    );

    if let Some(submenu) = cm.submenu {
        let (sub_x, sub_y, sub_w, _sub_h) =
            ctx_submenu_rect(cm.x, cm.y, submenu, screen_w, taskbar_y);
        let entries = ctx_submenu_entries(submenu);
        draw_context_panel(
            s,
            sw,
            sub_x,
            sub_y,
            sub_w,
            entries,
            ctx_menu_hit_index(entries, sub_x, sub_y, sub_w, mx, my),
            Some(submenu),
            show_desktop_icons,
            compact_spacing,
            desktop_sort,
        );
    }
}

pub(super) fn draw_context_panel(
    s: &mut [u32],
    sw: usize,
    menu_x: i32,
    menu_y: i32,
    menu_w: i32,
    entries: &[ContextEntryDef],
    hovered: Option<usize>,
    submenu: Option<DesktopContextSubmenu>,
    show_desktop_icons: bool,
    compact_spacing: bool,
    desktop_sort: DesktopSortMode,
) {
    let menu_h = ctx_menu_height(entries);
    let bg = 0x00_0B_13_1D;
    let bg_alt = 0x00_0F_19_25;
    let border_hot = 0x00_53_D5_F0;
    let top_glow = 0x00_19_3A_47;
    let hover_bg = 0x00_18_2D_3A;
    let hover_border = 0x00_2F_87_9F;
    let text = 0x00_D7_E5_EE;
    let text_hot = WHITE;
    let muted = 0x00_6D_7F_8D;
    let icon = 0x00_8C_A3_B4;
    let sep = 0x00_24_34_42;
    let rail = 0x00_0A_11_1A;

    s_fill(s, sw, menu_x, menu_y, menu_w, menu_h, bg);
    s_fill(s, sw, menu_x + 1, menu_y + 1, 34, menu_h - 2, rail);
    s_fill(
        s,
        sw,
        menu_x + 35,
        menu_y + 1,
        menu_w - 36,
        menu_h - 2,
        bg_alt,
    );
    s_fill(s, sw, menu_x + 1, menu_y + 1, menu_w - 2, 2, top_glow);
    s_fill(
        s,
        sw,
        menu_x + 35,
        menu_y + CTX_PAD,
        1,
        menu_h - CTX_PAD * 2,
        sep,
    );
    s_fill(
        s,
        sw,
        menu_x + 1,
        menu_y + menu_h - 2,
        menu_w - 2,
        1,
        0x00_00_05_10,
    );
    draw_glass_panel_outline(s, sw, menu_x, menu_y, menu_w, menu_h, border_hot);

    let mut row_y = menu_y + CTX_HEADER_H + CTX_PAD;
    for (idx, entry) in entries.iter().enumerate() {
        match entry.kind {
            ContextEntryKind::Separator => {
                s_fill(
                    s,
                    sw,
                    menu_x + 42,
                    row_y + CTX_SEP_H / 2,
                    menu_w - 56,
                    1,
                    sep,
                );
                row_y += CTX_SEP_H;
            }
            _ => {
                let hot = hovered == Some(idx) && entry.enabled;
                let text_y = row_y + (CTX_ITEM_H - 8) / 2;
                let mark_y = row_y + (CTX_ITEM_H - 8) / 2 - 1;
                let row_bg = if hot { hover_bg } else { bg_alt };
                if hot {
                    s_fill(s, sw, menu_x + 4, row_y, menu_w - 8, CTX_ITEM_H, hover_bg);
                    draw_rect_border(
                        s,
                        sw,
                        menu_x + 4,
                        row_y + 1,
                        menu_w - 8,
                        CTX_ITEM_H - 2,
                        hover_border,
                    );
                    s_fill(s, sw, menu_x + 5, row_y + 5, 2, CTX_ITEM_H - 10, ACCENT_HOV);
                }

                let mark = ctx_menu_mark(
                    entry.kind,
                    submenu,
                    show_desktop_icons,
                    compact_spacing,
                    desktop_sort,
                );
                if let Some(mark) = mark {
                    match mark {
                        MenuMark::Check => draw_menu_check(s, sw, menu_x + 13, mark_y, ACCENT_HOV),
                        MenuMark::Dot => s_fill(s, sw, menu_x + 16, mark_y + 2, 5, 5, ACCENT_HOV),
                    }
                } else if let Some(icon_kind) = ctx_menu_icon(*entry) {
                    draw_context_menu_icon(
                        s,
                        sw,
                        menu_x + 10,
                        row_y + (CTX_ITEM_H - 16) / 2,
                        icon_kind,
                        if !entry.enabled {
                            muted
                        } else if hot {
                            ACCENT_HOV
                        } else {
                            icon
                        },
                    );
                }

                if let Some(shortcut) = ctx_menu_shortcut(*entry) {
                    let shortcut_w = shortcut.chars().count() as i32 * 8;
                    let shortcut_x = menu_x + menu_w - 16 - shortcut_w;
                    if shortcut_x > menu_x + 46 {
                        s_draw_str_small(
                            s,
                            sw,
                            shortcut_x,
                            text_y,
                            shortcut,
                            if hot { 0x00_B9_C8_D4 } else { muted },
                            row_bg,
                            menu_x + menu_w - 12,
                        );
                    }
                }

                let fg = if !entry.enabled {
                    muted
                } else if hot {
                    text_hot
                } else {
                    text
                };
                s_draw_str_small(
                    s,
                    sw,
                    menu_x + 44,
                    text_y,
                    entry.label,
                    fg,
                    row_bg,
                    if ctx_menu_shortcut(*entry).is_some() {
                        menu_x + menu_w - 54
                    } else {
                        menu_x + menu_w - 24
                    },
                );

                if let ContextEntryKind::Submenu(_) = entry.kind {
                    draw_menu_chevron(
                        s,
                        sw,
                        menu_x + menu_w - 16,
                        text_y + 1,
                        if !entry.enabled {
                            muted
                        } else if hot {
                            text_hot
                        } else {
                            text
                        },
                    );
                }

                row_y += CTX_ITEM_H;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum MenuMark {
    Check,
    Dot,
}

#[derive(Clone, Copy)]
pub(super) enum ContextMenuIcon {
    View,
    Spacing,
    Sort,
    Refresh,
    Paste,
    NewItem,
    Folder,
    TextDocument,
    Display,
    Personalize,
}

pub(super) fn ctx_menu_icon(entry: ContextEntryDef) -> Option<ContextMenuIcon> {
    if entry.label == "Paste" || entry.label == "Paste shortcut" {
        return Some(ContextMenuIcon::Paste);
    }
    match entry.kind {
        ContextEntryKind::Action(DesktopContextCommand::ToggleDesktopIcons) => {
            Some(ContextMenuIcon::View)
        }
        ContextEntryKind::Action(DesktopContextCommand::ToggleCompactSpacing) => {
            Some(ContextMenuIcon::Spacing)
        }
        ContextEntryKind::Action(DesktopContextCommand::SortByName)
        | ContextEntryKind::Action(DesktopContextCommand::SortByType)
        | ContextEntryKind::Submenu(DesktopContextSubmenu::SortBy) => Some(ContextMenuIcon::Sort),
        ContextEntryKind::Action(DesktopContextCommand::Refresh) => Some(ContextMenuIcon::Refresh),
        ContextEntryKind::Action(DesktopContextCommand::CreateFolder) => {
            Some(ContextMenuIcon::Folder)
        }
        ContextEntryKind::Action(DesktopContextCommand::CreateTextDocument) => {
            Some(ContextMenuIcon::TextDocument)
        }
        ContextEntryKind::Action(DesktopContextCommand::DisplaySettings) => {
            Some(ContextMenuIcon::Display)
        }
        ContextEntryKind::Action(DesktopContextCommand::Personalize) => {
            Some(ContextMenuIcon::Personalize)
        }
        ContextEntryKind::Submenu(DesktopContextSubmenu::View) => Some(ContextMenuIcon::View),
        ContextEntryKind::Submenu(DesktopContextSubmenu::New) => Some(ContextMenuIcon::NewItem),
        ContextEntryKind::Separator => None,
    }
}

pub(super) fn ctx_menu_shortcut(entry: ContextEntryDef) -> Option<&'static str> {
    match (entry.label, entry.kind) {
        ("Refresh", ContextEntryKind::Action(DesktopContextCommand::Refresh)) if entry.enabled => {
            Some("F5")
        }
        _ => None,
    }
}

pub(super) fn ctx_menu_mark(
    kind: ContextEntryKind,
    submenu: Option<DesktopContextSubmenu>,
    show_desktop_icons: bool,
    compact_spacing: bool,
    desktop_sort: DesktopSortMode,
) -> Option<MenuMark> {
    match (submenu, kind) {
        (
            Some(DesktopContextSubmenu::View),
            ContextEntryKind::Action(DesktopContextCommand::ToggleDesktopIcons),
        ) if show_desktop_icons => Some(MenuMark::Check),
        (
            Some(DesktopContextSubmenu::View),
            ContextEntryKind::Action(DesktopContextCommand::ToggleCompactSpacing),
        ) if compact_spacing => Some(MenuMark::Check),
        (
            Some(DesktopContextSubmenu::SortBy),
            ContextEntryKind::Action(DesktopContextCommand::SortByName),
        ) if desktop_sort == DesktopSortMode::Name => Some(MenuMark::Dot),
        (
            Some(DesktopContextSubmenu::SortBy),
            ContextEntryKind::Action(DesktopContextCommand::SortByType),
        ) if desktop_sort == DesktopSortMode::Type => Some(MenuMark::Dot),
        _ => None,
    }
}

pub(super) fn draw_context_menu_icon(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    icon: ContextMenuIcon,
    color: u32,
) {
    match icon {
        ContextMenuIcon::View => {
            draw_rect_border(s, sw, x + 1, y + 2, 14, 10, color);
            s_fill(s, sw, x + 5, y + 13, 6, 1, color);
            s_fill(s, sw, x + 7, y + 12, 2, 2, color);
        }
        ContextMenuIcon::Spacing => {
            for &(dx, dy) in &[(2, 3), (10, 3), (2, 11), (10, 11)] {
                draw_rect_border(s, sw, x + dx, y + dy, 4, 4, color);
            }
        }
        ContextMenuIcon::Sort => {
            s_fill(s, sw, x + 2, y + 3, 9, 1, color);
            s_fill(s, sw, x + 2, y + 7, 12, 1, color);
            s_fill(s, sw, x + 2, y + 11, 6, 1, color);
            s_fill(s, sw, x + 12, y + 3, 1, 8, color);
            s_fill(s, sw, x + 10, y + 9, 5, 1, color);
            s_fill(s, sw, x + 11, y + 10, 3, 1, color);
            s_fill(s, sw, x + 12, y + 11, 1, 1, color);
        }
        ContextMenuIcon::Refresh => {
            s_fill(s, sw, x + 5, y + 2, 7, 1, color);
            s_fill(s, sw, x + 3, y + 3, 1, 2, color);
            s_fill(s, sw, x + 2, y + 5, 1, 5, color);
            s_fill(s, sw, x + 3, y + 10, 2, 1, color);
            s_fill(s, sw, x + 10, y + 1, 1, 4, color);
            s_fill(s, sw, x + 9, y + 4, 5, 1, color);
            s_fill(s, sw, x + 11, y + 5, 3, 1, color);
            s_fill(s, sw, x + 12, y + 6, 1, 1, color);
        }
        ContextMenuIcon::Paste => {
            draw_rect_border(s, sw, x + 3, y + 3, 10, 11, color);
            s_fill(s, sw, x + 6, y + 1, 4, 3, color);
            s_fill(s, sw, x + 5, y + 7, 6, 1, color);
            s_fill(s, sw, x + 5, y + 10, 5, 1, color);
        }
        ContextMenuIcon::NewItem => {
            draw_rect_border(s, sw, x + 2, y + 2, 12, 12, color);
            s_fill(s, sw, x + 7, y + 5, 2, 6, color);
            s_fill(s, sw, x + 5, y + 7, 6, 2, color);
        }
        ContextMenuIcon::Folder => {
            s_fill(s, sw, x + 2, y + 5, 12, 8, color);
            s_fill(s, sw, x + 2, y + 3, 5, 3, color);
            s_fill(s, sw, x + 3, y + 8, 10, 1, blend_color(color, BLACK, 130));
        }
        ContextMenuIcon::TextDocument => {
            draw_rect_border(s, sw, x + 4, y + 2, 9, 12, color);
            s_fill(s, sw, x + 6, y + 5, 5, 1, color);
            s_fill(s, sw, x + 6, y + 8, 5, 1, color);
            s_fill(s, sw, x + 6, y + 11, 4, 1, color);
        }
        ContextMenuIcon::Display => {
            draw_rect_border(s, sw, x + 1, y + 2, 14, 9, color);
            s_fill(s, sw, x + 6, y + 12, 4, 2, color);
            s_fill(s, sw, x + 4, y + 14, 8, 1, color);
        }
        ContextMenuIcon::Personalize => {
            s_fill(s, sw, x + 2, y + 3, 5, 5, color);
            s_fill(s, sw, x + 9, y + 3, 5, 5, blend_color(color, WHITE, 70));
            s_fill(s, sw, x + 2, y + 10, 5, 5, blend_color(color, BLACK, 70));
            s_fill(s, sw, x + 9, y + 10, 5, 5, color);
        }
    }
}
