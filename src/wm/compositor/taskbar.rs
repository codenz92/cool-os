use super::icons::*;
use super::primitives::*;
use super::*;

pub(super) struct TaskbarMenu {
    pub(super) window: usize,
    pub(super) x: i32,
    pub(super) y: i32,
}

#[derive(Clone, Copy)]
pub(super) struct TaskbarTrayLayout {
    pub(super) tray_y: i32,
    pub(super) tray_h: i32,
    pub(super) icons_x: i32,
    pub(super) icons_y: i32,
    pub(super) icons_w: i32,
    pub(super) icons_h: i32,
    pub(super) clock_x: i32,
    pub(super) clock_y: i32,
    pub(super) clock_w: i32,
    pub(super) clock_h: i32,
    pub(super) usb_x: i32,
    pub(super) kbd_x: i32,
    pub(super) mouse_x: i32,
    pub(super) icon_y: i32,
}

impl TaskbarTrayLayout {
    pub(super) fn icons_contains(self, px: i32, py: i32) -> bool {
        rect_contains(
            self.icons_x,
            self.icons_y,
            self.icons_w,
            self.icons_h,
            px,
            py,
        )
    }

    pub(super) fn clock_contains(self, px: i32, py: i32) -> bool {
        rect_contains(
            self.clock_x,
            self.clock_y,
            self.clock_w,
            self.clock_h,
            px,
            py,
        )
    }
}

pub(super) fn draw_taskbar_tray(
    s: &mut [u32],
    sw: usize,
    taskbar_y: i32,
    uptime_ticks: u64,
    mx: i32,
    my: i32,
) {
    let layout = taskbar_tray_layout(sw as i32, taskbar_y);
    let icons_hot = layout.icons_contains(mx, my);
    let clock_hot = layout.clock_contains(mx, my);
    if icons_hot {
        fill_vertical_gradient(
            s,
            sw,
            layout.icons_x,
            layout.tray_y + 1,
            layout.icons_w,
            layout.tray_h - 2,
            0x00_15_22_2E,
            0x00_0A_11_1A,
        );
        draw_rect_border(
            s,
            sw,
            layout.icons_x,
            layout.tray_y + 1,
            layout.icons_w,
            layout.tray_h - 2,
            0x00_24_35_43,
        );
        s_fill(
            s,
            sw,
            layout.icons_x + 8,
            layout.tray_y + layout.tray_h - 3,
            layout.icons_w - 16,
            1,
            ACCENT,
        );
    }
    if clock_hot {
        fill_vertical_gradient(
            s,
            sw,
            layout.clock_x,
            layout.tray_y + 1,
            layout.clock_w,
            layout.tray_h - 2,
            0x00_15_22_2E,
            0x00_0A_11_1A,
        );
        draw_rect_border(
            s,
            sw,
            layout.clock_x,
            layout.tray_y + 1,
            layout.clock_w,
            layout.tray_h - 2,
            0x00_24_35_43,
        );
        s_fill(
            s,
            sw,
            layout.clock_x + 8,
            layout.tray_y + layout.tray_h - 3,
            layout.clock_w - 16,
            1,
            ACCENT,
        );
    }

    let usb_lines = crate::usb::status_lines();
    let (usb_keyboard, usb_mouse) = crate::usb::input_presence();
    let usb_present = !usb_lines.is_empty();
    let usb_active = usb_lines
        .iter()
        .any(|line| line.contains("active init ready"));
    let pulse_step = (crate::interrupts::TIMER_HZ / 8).max(1) as u64;
    let pulse = ((uptime_ticks / pulse_step) % 28) as u32;
    let usb_col = if usb_active {
        blend_color(0x00_67_C8_DA, ACCENT_HOV, pulse * 3)
    } else if usb_present {
        0x00_77_AE_BE
    } else {
        0x00_5A_65_6E
    };
    let kbd_col = if usb_keyboard {
        0x00_74_C8_9F
    } else {
        0x00_62_73_6A
    };
    let mouse_col = if usb_mouse {
        0x00_D6_BA_70
    } else {
        0x00_75_70_5E
    };

    draw_usb_tray_icon(s, sw, layout.usb_x, layout.icon_y, usb_col);
    draw_keyboard_tray_icon(s, sw, layout.kbd_x, layout.icon_y, kbd_col);
    draw_mouse_tray_icon(s, sw, layout.mouse_x, layout.icon_y, mouse_col);

    let (time, date) = taskbar_clock_lines(uptime_ticks);
    let text_center = layout.clock_x + layout.clock_w / 2;
    let time_w = time.len() as i32 * 8;
    let time_x = text_center - time_w / 2;
    s_draw_str_small_transparent(
        s,
        sw,
        time_x + 1,
        layout.tray_y + 5,
        &time,
        0x00_04_09_0F,
        layout.clock_x + layout.clock_w,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        time_x,
        layout.tray_y + 4,
        &time,
        0x00_E4_EF_F6,
        layout.clock_x + layout.clock_w,
    );

    let date_w = date.len() as i32 * 8;
    let date_x = text_center - date_w / 2;
    s_draw_str_small_transparent(
        s,
        sw,
        date_x,
        layout.tray_y + 18,
        &date,
        0x00_A9_B7_C4,
        layout.clock_x + layout.clock_w,
    );

    let unread = crate::notifications::unread_count();
    if unread > 0 {
        let dot_x = layout.clock_x + layout.clock_w - 5;
        let dot_y = layout.tray_y + 4;
        s_fill(s, sw, dot_x + 1, dot_y, 4, 1, 0x00_FF_8F_99);
        s_fill(s, sw, dot_x, dot_y + 1, 6, 4, 0x00_DD_3F_4E);
        s_fill(s, sw, dot_x + 1, dot_y + 5, 4, 1, 0x00_BA_2B_39);
        s_fill(s, sw, dot_x + 2, dot_y + 1, 1, 1, 0x00_FF_A4_AD);
    }
}

pub(super) fn draw_taskbar_preview(
    s: &mut [u32],
    sw: usize,
    taskbar_y: i32,
    button_x: i32,
    app: &AppWindow,
) {
    let win = app.window();
    let preview_w = 208i32;
    let preview_h = 74i32;
    let x = (button_x + BUTTON_W / 2 - preview_w / 2)
        .max(4)
        .min(sw as i32 - preview_w - 4);
    let y = (taskbar_y - preview_h - 8).max(0);
    let accent = window_accent(win.title);
    s_fill(s, sw, x, y, preview_w, preview_h, 0x00_00_08_18);
    s_fill(s, sw, x, y, preview_w, 3, accent);
    draw_glass_panel_outline(s, sw, x, y, preview_w, preview_h, accent);
    draw_live_window_thumbnail(s, sw, x + 10, y + 16, 92, 40, win);
    draw_shell_app_icon(s, sw, x + 120, y + 20, 24, desktop_icon_kind(win.title));
    s_draw_str_small(
        s,
        sw,
        x + 10,
        y + 7,
        win.title,
        WHITE,
        0x00_00_08_18,
        x + 108,
    );
    let state = if win.minimized { "minimized" } else { "open" };
    s_draw_str_small(
        s,
        sw,
        x + 116,
        y + 32,
        state,
        0x00_66_AA_DD,
        0x00_00_08_18,
        x + preview_w - 10,
    );
    let bounds = format!("{}x{} @ {},{}", win.width, win.height, win.x, win.y);
    s_draw_str_small(
        s,
        sw,
        x + 10,
        y + 60,
        &bounds,
        0x00_44_88_BB,
        0x00_00_08_18,
        x + preview_w - 10,
    );
}

pub(super) fn draw_taskbar_menu(
    s: &mut [u32],
    sw: usize,
    menu: &TaskbarMenu,
    windows: &[AppWindow],
    mx: i32,
    my: i32,
) {
    if menu.window >= windows.len() {
        return;
    }
    let bg = 0x00_00_08_18;
    s_fill(s, sw, menu.x, menu.y, TASKBAR_MENU_W, TASKBAR_MENU_H, bg);
    s_fill(s, sw, menu.x, menu.y, TASKBAR_MENU_W, 3, ACCENT);
    draw_glass_panel_outline(
        s,
        sw,
        menu.x,
        menu.y,
        TASKBAR_MENU_W,
        TASKBAR_MENU_H,
        ACCENT,
    );
    let labels = [
        if windows[menu.window].is_minimized() {
            "Restore"
        } else {
            "Minimize"
        },
        "Maximize",
        "Close",
    ];
    for (i, label) in labels.iter().enumerate() {
        let row_y = menu.y + 5 + i as i32 * TASKBAR_MENU_ROW_H;
        let hot = mx >= menu.x + 4
            && mx < menu.x + TASKBAR_MENU_W - 4
            && my >= row_y
            && my < row_y + TASKBAR_MENU_ROW_H;
        let row_bg = if hot { 0x00_00_18_34 } else { bg };
        if hot {
            s_fill(
                s,
                sw,
                menu.x + 3,
                row_y,
                TASKBAR_MENU_W - 6,
                TASKBAR_MENU_ROW_H,
                row_bg,
            );
            s_fill(
                s,
                sw,
                menu.x + 4,
                row_y + 5,
                2,
                TASKBAR_MENU_ROW_H - 10,
                ACCENT,
            );
        }
        s_draw_str_small(
            s,
            sw,
            menu.x + 14,
            row_y + 8,
            label,
            if hot { WHITE } else { 0x00_88_CC_FF },
            row_bg,
            menu.x + TASKBAR_MENU_W - 10,
        );
    }
}

pub(super) fn taskbar_tray_layout(sw: i32, taskbar_y: i32) -> TaskbarTrayLayout {
    let tray_w = TASKBAR_CLOCK_W.min((sw - 12).max(132));
    let tray_h = TASKBAR_H - 8;
    let tray_x = (sw - tray_w - 4).max(START_BTN_W + 8);
    let tray_y = taskbar_y + 4;
    let icons_x = tray_x + 4;
    let icons_y = tray_y + 2;
    let icons_w = TASKBAR_TRAY_W;
    let icons_h = tray_h - 4;
    let icon_span = 14 * 3 + 8 * 2;
    let icon_start = icons_x + (icons_w - icon_span) / 2;
    let icon_y = tray_y + (tray_h - 14) / 2;
    let clock_x = icons_x + icons_w + 8;
    let clock_w = (tray_x + tray_w - clock_x - 4).max(64);
    TaskbarTrayLayout {
        tray_y,
        tray_h,
        icons_x,
        icons_y,
        icons_w,
        icons_h,
        clock_x,
        clock_y: tray_y + 2,
        clock_w,
        clock_h: tray_h - 4,
        usb_x: icon_start,
        kbd_x: icon_start + 22,
        mouse_x: icon_start + 44,
        icon_y,
    }
}

pub(super) fn draw_usb_tray_icon(s: &mut [u32], sw: usize, x: i32, y: i32, color: u32) {
    s_fill(s, sw, x + 6, y + 1, 2, 10, color);
    s_fill(s, sw, x + 4, y + 1, 6, 1, blend_color(color, WHITE, 96));
    s_fill(s, sw, x + 5, y + 3, 4, 1, color);
    s_fill(s, sw, x + 2, y + 5, 5, 1, color);
    s_fill(s, sw, x + 7, y + 5, 5, 1, color);
    s_fill(s, sw, x + 2, y + 6, 1, 3, color);
    s_fill(s, sw, x + 11, y + 6, 1, 3, color);
    s_fill(s, sw, x + 5, y + 11, 4, 2, blend_color(color, WHITE, 70));
}

pub(super) fn draw_keyboard_tray_icon(s: &mut [u32], sw: usize, x: i32, y: i32, color: u32) {
    draw_rect_border(s, sw, x, y + 3, 14, 8, color);
    s_fill(s, sw, x + 2, y + 5, 10, 1, blend_color(color, WHITE, 70));
    s_fill(s, sw, x + 2, y + 7, 2, 1, color);
    s_fill(s, sw, x + 5, y + 7, 2, 1, color);
    s_fill(s, sw, x + 8, y + 7, 2, 1, color);
    s_fill(s, sw, x + 3, y + 12, 8, 1, blend_color(color, WHITE, 56));
}

pub(super) fn draw_mouse_tray_icon(s: &mut [u32], sw: usize, x: i32, y: i32, color: u32) {
    draw_rect_border(s, sw, x + 2, y + 1, 10, 13, color);
    s_fill(s, sw, x + 7, y + 3, 1, 3, blend_color(color, WHITE, 90));
    s_fill(s, sw, x + 3, y + 7, 8, 1, color);
    s_fill(s, sw, x + 5, y + 13, 4, 1, blend_color(color, WHITE, 56));
}
