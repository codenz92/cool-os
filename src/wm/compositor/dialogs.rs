use super::icons::*;
use super::primitives::*;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellDialogKind {
    Error,
    Crash,
}

#[derive(Clone)]
pub(super) struct ShellDialog {
    pub(super) title: String,
    pub(super) body: String,
    pub(super) kind: ShellDialogKind,
    pub(super) restart_target: Option<String>,
}

pub(super) fn draw_task_switcher_overlay(
    s: &mut [u32],
    sw: usize,
    taskbar_y: i32,
    windows: &[AppWindow],
    window_workspaces: &[usize],
    z_order: &[usize],
    focused: Option<usize>,
    current_workspace: usize,
    query: &str,
) {
    let query_lower = query.to_ascii_lowercase();
    let visible_count = z_order
        .iter()
        .rev()
        .filter(|&&idx| {
            idx < windows.len()
                && window_workspaces
                    .get(idx)
                    .copied()
                    .unwrap_or(0)
                    .min(WORKSPACE_COUNT - 1)
                    == current_workspace
                && !windows[idx].window().minimized
                && (query_lower.is_empty()
                    || windows[idx]
                        .window()
                        .title
                        .to_ascii_lowercase()
                        .contains(&query_lower))
        })
        .count();
    if visible_count == 0 {
        return;
    }

    let shown = visible_count.min(6);
    let item_w = 126i32;
    let item_h = 64i32;
    let gap = 10i32;
    let panel_w = 32 + shown as i32 * item_w + (shown.saturating_sub(1)) as i32 * gap;
    let panel_h = 112i32;
    let panel_x = ((sw as i32 - panel_w) / 2).max(0);
    let panel_y = ((taskbar_y - panel_h) / 2).max(0);

    s_fill(s, sw, panel_x, panel_y, panel_w, panel_h, 0x00_00_07_18);
    s_fill(s, sw, panel_x, panel_y, panel_w, 3, ACCENT);
    draw_glass_panel_outline(s, sw, panel_x, panel_y, panel_w, panel_h, ACCENT);
    s_draw_str_small(
        s,
        sw,
        panel_x + 16,
        panel_y + 12,
        "TASK SWITCHER",
        0x00_CC_EE_FF,
        0x00_00_07_18,
        panel_x + panel_w - 16,
    );
    s_draw_str_small(
        s,
        sw,
        panel_x + 16,
        panel_y + 26,
        "Alt+Tab cycles windows",
        0x00_55_88_AA,
        0x00_00_07_18,
        panel_x + panel_w - 16,
    );
    if !query.is_empty() {
        let mut search = String::from("Search: ");
        search.push_str(query);
        s_draw_str_small(
            s,
            sw,
            panel_x + 170,
            panel_y + 26,
            &search,
            ACCENT_HOV,
            0x00_00_07_18,
            panel_x + panel_w - 16,
        );
    }

    let mut drawn = 0usize;
    for &win_idx in z_order.iter().rev() {
        if drawn >= shown {
            break;
        }
        if win_idx >= windows.len()
            || window_workspaces
                .get(win_idx)
                .copied()
                .unwrap_or(0)
                .min(WORKSPACE_COUNT - 1)
                != current_workspace
            || windows[win_idx].window().minimized
            || (!query_lower.is_empty()
                && !windows[win_idx]
                    .window()
                    .title
                    .to_ascii_lowercase()
                    .contains(&query_lower))
        {
            continue;
        }

        let win = windows[win_idx].window();
        let x = panel_x + 16 + drawn as i32 * (item_w + gap);
        let y = panel_y + 42;
        let selected = focused == Some(win_idx);
        let accent = window_accent(win.title);
        let bg = if selected {
            0x00_00_1E_3C
        } else {
            0x00_00_0B_20
        };
        let border = if selected { ACCENT_HOV } else { 0x00_00_33_66 };

        s_fill(s, sw, x, y, item_w, item_h, bg);
        s_fill(s, sw, x, y, item_w, 3, accent);
        draw_rect_border(s, sw, x, y, item_w, item_h, border);
        draw_live_window_thumbnail(s, sw, x + 8, y + 10, 38, 28, win);

        draw_shell_app_icon(s, sw, x + 15, y + 42, 16, desktop_icon_kind(win.title));

        let title = if win.title.len() > 11 {
            &win.title[..11]
        } else {
            win.title
        };
        s_draw_str_small(
            s,
            sw,
            x + 48,
            y + 18,
            title,
            if selected { WHITE } else { 0x00_88_CC_FF },
            bg,
            x + item_w - 8,
        );
        s_draw_str_small(
            s,
            sw,
            x + 48,
            y + 34,
            if selected { "active" } else { "window" },
            if selected { ACCENT_HOV } else { 0x00_44_77_99 },
            bg,
            x + item_w - 8,
        );

        drawn += 1;
    }
}

pub(super) fn draw_file_drag_badge(s: &mut [u32], sw: usize, x: i32, y: i32, count: usize) {
    let w = 132i32;
    let h = 34i32;
    let x = x.min(sw as i32 - w - 4).max(4);
    let sh = if sw > 0 { s.len() / sw } else { 0 };
    let y = y.min(sh as i32 - h - 4).max(4);
    let bg = 0x00_00_08_18;
    s_fill(s, sw, x, y, w, h, bg);
    s_fill(s, sw, x, y, 3, h, ACCENT);
    draw_glass_panel_outline(s, sw, x, y, w, h, ACCENT);
    s_draw_str_small(s, sw, x + 12, y + 7, "DROP FILES", WHITE, bg, x + w - 8);
    let text = if count == 1 {
        String::from("1 item")
    } else {
        format!("{} items", count)
    };
    s_draw_str_small(s, sw, x + 12, y + 19, &text, 0x00_66_AA_DD, bg, x + w - 8);
}

pub(super) fn draw_live_window_thumbnail(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    win: &Window,
) {
    s_fill(s, sw, x, y, w, h, 0x00_00_04_10);
    draw_rect_border(s, sw, x, y, w, h, 0x00_00_44_88);
    let src_w = win.width.max(1) as usize;
    let src_h = (win.height - TITLE_H).max(1) as usize;
    if win.buf.is_empty() {
        return;
    }
    for ty in 1..h.saturating_sub(1) {
        let sy = (ty as usize * src_h / h.max(1) as usize).min(src_h - 1);
        for tx in 1..w.saturating_sub(1) {
            let sx = (tx as usize * src_w / w.max(1) as usize).min(src_w - 1);
            let src = sy * src_w + sx;
            if src < win.buf.len() {
                s_put(s, sw, usize::MAX, x + tx, y + ty, win.buf[src]);
            }
        }
    }
}

pub(super) fn draw_shell_dialog(s: &mut [u32], sw: usize, taskbar_y: i32, dialog: &ShellDialog) {
    let (x, y, w, h) = shell_dialog_rect(sw as i32, taskbar_y, dialog);
    let bg = 0x00_07_0D_1C;
    s_fill_alpha(s, sw, 0, 0, sw as i32, taskbar_y, 0x44_00_00_00);
    s_fill(s, sw, x, y, w, h, bg);
    s_fill(s, sw, x, y, w, 4, 0x00_FF_66_66);
    draw_glass_panel_outline(s, sw, x, y, w, h, 0x00_FF_88_88);
    s_draw_str_small(s, sw, x + 18, y + 18, &dialog.title, WHITE, bg, x + w - 18);
    s_draw_str_small(
        s,
        sw,
        x + 18,
        y + 44,
        &dialog.body,
        0x00_CC_EE_FF,
        bg,
        x + w - 18,
    );
    s_draw_str_small(
        s,
        sw,
        x + 18,
        y + if dialog.kind == ShellDialogKind::Crash {
            h - 56
        } else {
            h - 26
        },
        if dialog.kind == ShellDialogKind::Crash {
            "app failure captured"
        } else {
            "click anywhere to dismiss"
        },
        0x00_66_99_BB,
        bg,
        x + w - 18,
    );
    if dialog.kind == ShellDialogKind::Crash {
        let button_y = y + h - 34;
        draw_dialog_button(s, sw, x + 18, button_y, "View Dump", 0x00_FF_88_88);
        draw_dialog_button(s, sw, x + 122, button_y, "Restart", 0x00_FF_DD_55);
        draw_dialog_button(s, sw, x + 226, button_y, "Copy", 0x00_55_FF_BB);
        draw_dialog_button(s, sw, x + w - 112, button_y, "Dismiss", 0x00_66_AA_DD);
    }
}

pub(super) fn shell_dialog_rect(
    sw: i32,
    taskbar_y: i32,
    dialog: &ShellDialog,
) -> (i32, i32, i32, i32) {
    let w = 460i32;
    let h = if dialog.kind == ShellDialogKind::Crash {
        168i32
    } else {
        132i32
    };
    let x = ((sw - w) / 2).max(8);
    let y = ((taskbar_y - h) / 2).max(8);
    (x, y, w, h)
}

pub(super) fn draw_dialog_button(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    label: &str,
    accent: u32,
) {
    let w = 94i32;
    let h = 22i32;
    let bg = 0x00_00_0C_20;
    s_fill(s, sw, x, y, w, h, bg);
    s_fill(s, sw, x, y, w, 2, accent);
    draw_rect_border(s, sw, x, y, w, h, blend_color(accent, 0x00_00_08_18, 90));
    let text_w = label.chars().count() as i32 * 8;
    s_draw_str_small(
        s,
        sw,
        x + ((w - text_w) / 2).max(4),
        y + 7,
        label,
        0x00_DD_FF_FF,
        bg,
        x + w - 4,
    );
}
