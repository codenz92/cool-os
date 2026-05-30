extern crate alloc;

use super::primitives::*;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct NotificationCenterLayout {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) clear_x: i32,
    pub(super) clear_y: i32,
    pub(super) clear_w: i32,
    pub(super) clear_h: i32,
    pub(super) list_x: i32,
    pub(super) list_y: i32,
    pub(super) row_w: i32,
    pub(super) row_h: i32,
    pub(super) row_gap: i32,
}

impl NotificationCenterLayout {
    pub(super) fn contains(self, px: i32, py: i32) -> bool {
        rect_contains(self.x, self.y, self.w, self.h, px, py)
    }

    pub(super) fn clear_contains(self, px: i32, py: i32) -> bool {
        rect_contains(
            self.clear_x,
            self.clear_y,
            self.clear_w,
            self.clear_h,
            px,
            py,
        )
    }

    pub(super) fn row_rect(self, index: usize) -> (i32, i32, i32, i32) {
        let y = self.list_y + index as i32 * (self.row_h + self.row_gap);
        (self.list_x, y, self.row_w, self.row_h)
    }

    pub(super) fn dismiss_rect(self, index: usize) -> (i32, i32, i32, i32) {
        let (row_x, row_y, row_w, _) = self.row_rect(index);
        (row_x + row_w - 28, row_y + 11, 18, 18)
    }

    pub(super) fn dismiss_contains(self, index: usize, px: i32, py: i32) -> bool {
        let (x, y, w, h) = self.dismiss_rect(index);
        rect_contains(x, y, w, h, px, py)
    }

    pub(super) fn max_rows(self) -> usize {
        let available = self.y + self.h - 14 - self.list_y;
        if available <= 0 {
            return 0;
        }
        (available / (self.row_h + self.row_gap)).max(0) as usize
    }
}

pub(super) fn draw_notification_center(s: &mut [u32], sw: usize, taskbar_y: i32, mx: i32, my: i32) {
    let layout = notification_center_layout(sw as i32, taskbar_y);
    let bg_top = 0x00_11_19_24;
    let bg_bottom = 0x00_08_0F_18;
    let border = 0x00_35_45_55;
    let text = 0x00_E8_F0_F7;
    let text_dim = 0x00_A9_B7_C4;
    let text_muted = 0x00_7E_90_A0;

    draw_notification_surface(
        s, sw, layout.x, layout.y, layout.w, layout.h, bg_top, bg_bottom, border, ACCENT,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.x + 18,
        layout.y + 18,
        "Notifications",
        text,
        layout.x + layout.w - 110,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.x + 18,
        layout.y + 35,
        "Recent activity",
        text_muted,
        layout.x + layout.w - 18,
    );

    let rows = layout.max_rows();
    let list = crate::notifications::latest(rows);
    let clear_enabled = !list.is_empty();
    let clear_hot = clear_enabled && layout.clear_contains(mx, my);
    let clear_bg = if clear_hot {
        0x00_1B_2A_36
    } else {
        0x00_12_1E_2A
    };
    draw_notification_surface(
        s,
        sw,
        layout.clear_x,
        layout.clear_y,
        layout.clear_w,
        layout.clear_h,
        clear_bg,
        0x00_0C_15_20,
        if clear_enabled {
            0x00_35_45_55
        } else {
            0x00_22_2F_3A
        },
        if clear_hot { ACCENT_HOV } else { ACCENT },
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.clear_x + 7,
        layout.clear_y + 7,
        "Clear all",
        if clear_enabled {
            text_dim
        } else {
            0x00_5E_6E_7C
        },
        layout.clear_x + layout.clear_w - 5,
    );

    if list.is_empty() {
        let empty_y = layout.y + layout.h / 2 - 8;
        s_draw_str_small_transparent(
            s,
            sw,
            layout.x + 18,
            empty_y,
            "No notifications",
            text_dim,
            layout.x + layout.w - 18,
        );
        s_draw_str_small_transparent(
            s,
            sw,
            layout.x + 18,
            empty_y + 16,
            "System activity will appear here.",
            text_muted,
            layout.x + layout.w - 18,
        );
        return;
    }

    for (row, note) in list.iter().rev().enumerate() {
        let (row_x, row_y, row_w, row_h) = layout.row_rect(row);
        let row_hot = rect_contains(row_x, row_y, row_w, row_h, mx, my);
        let accent = notification_accent(&note.title);
        let row_top = if row_hot {
            0x00_16_25_31
        } else {
            0x00_11_1A_24
        };
        let row_bottom = if row_hot {
            0x00_0D_17_21
        } else {
            0x00_0A_12_1C
        };
        draw_notification_surface(
            s,
            sw,
            row_x,
            row_y,
            row_w,
            row_h,
            row_top,
            row_bottom,
            if note.unread { ACCENT } else { 0x00_26_34_42 },
            accent,
        );
        if note.unread {
            s_fill(s, sw, row_x + 8, row_y + 12, 3, row_h - 24, accent);
        } else {
            s_fill(s, sw, row_x + 9, row_y + 20, 2, 6, 0x00_3B_4A_58);
        }
        draw_notification_glyph(s, sw, row_x + 18, row_y + 13, &note.title, accent);

        let text_x = row_x + 48;
        let text_max = row_x + row_w - 38;
        s_draw_str_small_transparent(
            s,
            sw,
            text_x,
            row_y + 9,
            &note.title,
            if note.unread { text } else { text_dim },
            text_max,
        );
        s_draw_str_small_transparent(s, sw, text_x, row_y + 25, &note.body, text_muted, text_max);

        let (dx, dy, dw, dh) = layout.dismiss_rect(row);
        let dismiss_hot = layout.dismiss_contains(row, mx, my);
        if dismiss_hot {
            s_fill(s, sw, dx, dy, dw, dh, 0x00_1D_2D_39);
            draw_rect_border(s, sw, dx, dy, dw, dh, 0x00_3A_4B_5B);
        }
        let x_col = if dismiss_hot {
            0x00_E8_F0_F7
        } else {
            0x00_7E_90_A0
        };
        draw_icon_line(s, sw, dx + 5, dy + 5, dx + dw - 6, dy + dh - 6, x_col);
        draw_icon_line(s, sw, dx + dw - 6, dy + 5, dx + 5, dy + dh - 6, x_col);
    }
}

pub(super) fn draw_notification_toasts(s: &mut [u32], sw: usize, taskbar_y: i32, ticks: u64) {
    let list = crate::notifications::latest_toasts(2);
    if list.is_empty() {
        return;
    }
    let timeout = crate::interrupts::ticks_for_millis(6200);
    let toast_w = 324i32.min(sw as i32 - 24);
    let toast_h = 58i32;
    let mut drawn = 0i32;
    for note in list.iter() {
        if ticks.wrapping_sub(note.tick) > timeout {
            continue;
        }
        let x = (sw as i32 - toast_w - 12).max(0);
        let y = (taskbar_y - 14 - (drawn + 1) * (toast_h + 10)).max(0);
        let accent = notification_accent(&note.title);
        draw_notification_surface(
            s,
            sw,
            x,
            y,
            toast_w,
            toast_h,
            0x00_13_1D_29,
            0x00_09_11_1B,
            0x00_35_45_55,
            accent,
        );
        draw_notification_glyph(s, sw, x + 14, y + 16, &note.title, accent);
        s_draw_str_small_transparent(
            s,
            sw,
            x + 44,
            y + 12,
            &note.title,
            0x00_E8_F0_F7,
            x + toast_w - 14,
        );
        s_draw_str_small_transparent(
            s,
            sw,
            x + 44,
            y + 30,
            &note.body,
            0x00_A9_B7_C4,
            x + toast_w - 14,
        );
        drawn += 1;
    }
}

pub(super) fn draw_notification_surface(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    top: u32,
    bottom: u32,
    border: u32,
    accent: u32,
) {
    fill_vertical_gradient(s, sw, x + 1, y + 1, w - 2, h - 2, top, bottom);
    s_fill(s, sw, x + 2, y, w - 4, 1, blend_color(accent, WHITE, 70));
    s_fill(
        s,
        sw,
        x + 1,
        y + 1,
        1,
        h - 2,
        blend_color(border, accent, 60),
    );
    s_fill(s, sw, x + w - 2, y + 1, 1, h - 2, border);
    s_fill(s, sw, x + 2, y + h - 1, w - 4, 1, border);
    s_fill(s, sw, x + 1, y, 1, 1, border);
    s_fill(s, sw, x + w - 2, y, 1, 1, border);
    s_fill(s, sw, x + 1, y + h - 1, 1, 1, border);
    s_fill(s, sw, x + w - 2, y + h - 1, 1, 1, border);
}

pub(super) fn draw_notification_glyph(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    title: &str,
    accent: u32,
) {
    let bg = blend_color(0x00_0D_16_20, accent, 52);
    s_fill(s, sw, x + 1, y, 18, 20, bg);
    s_fill(s, sw, x, y + 1, 20, 18, bg);
    draw_rect_border(s, sw, x + 1, y, 18, 20, blend_color(accent, WHITE, 64));
    s_fill(s, sw, x + 5, y + 4, 10, 1, blend_color(accent, WHITE, 120));
    let glyph = notification_glyph(title);
    s_draw_char_small(s, sw, x + 6, y + 7, glyph, 0x00_E8_F0_F7, bg);
}

pub(super) fn notification_glyph(title: &str) -> char {
    if title.contains("failed")
        || title.contains("crashed")
        || title.contains("faulted")
        || title.contains("killed")
    {
        '!'
    } else if title.contains("Clipboard") {
        'C'
    } else if title.contains("Screenshot") {
        'S'
    } else if title.contains("USB") {
        'U'
    } else if title.contains("Power") {
        'P'
    } else {
        'i'
    }
}

pub(super) fn notification_accent(title: &str) -> u32 {
    if title.contains("failed") || title.contains("crashed") || title.contains("faulted") {
        0x00_EA_6B_72
    } else if title.contains("killed") || title.contains("Power") {
        0x00_F5_CB_6B
    } else if title.contains("Clipboard") || title.contains("Screenshot") {
        0x00_4C_DD_A1
    } else {
        ACCENT
    }
}

pub(super) fn notification_center_layout(sw: i32, taskbar_y: i32) -> NotificationCenterLayout {
    let panel_w = 368.min((sw - 24).max(280));
    let available_h = (taskbar_y - 24).max(280);
    let panel_h = 424.min(available_h);
    let x = (sw - panel_w - 12).max(0);
    let y = (taskbar_y - panel_h - 12).max(0);
    NotificationCenterLayout {
        x,
        y,
        w: panel_w,
        h: panel_h,
        clear_x: x + panel_w - 98,
        clear_y: y + 18,
        clear_w: 84,
        clear_h: 22,
        list_x: x + 12,
        list_y: y + 62,
        row_w: panel_w - 24,
        row_h: 46,
        row_gap: 7,
    }
}

#[allow(dead_code)]
pub(super) fn workspace_label(workspace: usize) -> &'static str {
    match workspace {
        0 => "WS1",
        1 => "WS2",
        2 => "WS3",
        3 => "WS4",
        _ => "WS?",
    }
}
