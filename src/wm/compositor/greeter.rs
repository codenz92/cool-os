extern crate alloc;

use super::icons::*;
use super::primitives::*;
use super::*;
use alloc::{format, string::String};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GreeterFocus {
    User,
    Password,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstBootFocus {
    Owner,
    Password,
    Confirm,
    Device,
}

#[derive(Clone, Copy)]
pub(super) struct GreeterLayout {
    pub(super) panel_x: i32,
    pub(super) panel_y: i32,
    pub(super) panel_w: i32,
    pub(super) panel_h: i32,
    pub(super) avatar_x: i32,
    pub(super) avatar_y: i32,
    pub(super) avatar_size: i32,
    pub(super) title_x: i32,
    pub(super) title_y: i32,
    pub(super) user_x: i32,
    pub(super) user_y: i32,
    pub(super) field_w: i32,
    pub(super) pass_y: i32,
    pub(super) button_y: i32,
    pub(super) message_y: i32,
    pub(super) users_y: i32,
    pub(super) row_w: i32,
}

#[derive(Clone, Copy)]
pub(super) struct FirstBootLayout {
    pub(super) panel_x: i32,
    pub(super) panel_y: i32,
    pub(super) panel_w: i32,
    pub(super) panel_h: i32,
    pub(super) avatar_x: i32,
    pub(super) avatar_y: i32,
    pub(super) avatar_size: i32,
    pub(super) title_x: i32,
    pub(super) title_y: i32,
    pub(super) field_x: i32,
    pub(super) field_w: i32,
    pub(super) owner_y: i32,
    pub(super) pass_y: i32,
    pub(super) confirm_y: i32,
    pub(super) device_y: i32,
    pub(super) button_y: i32,
    pub(super) message_y: i32,
}

// ── AppWindow ─────────────────────────────────────────────────────────────────

pub(super) fn draw_greeter_overlay(
    s: &mut [u32],
    sw: usize,
    taskbar_y: i32,
    user: &str,
    password_len: usize,
    focus: GreeterFocus,
    message: &str,
    error: bool,
    attempts: u32,
    mx: i32,
    my: i32,
    _uptime_ticks: u64,
) {
    let sw_i = sw as i32;
    let sh_i = if sw > 0 {
        (s.len() / sw) as i32
    } else {
        taskbar_y
    };
    let layout = greeter_layout(sw_i, taskbar_y);

    draw_greeter_backdrop(s, sw, sw_i, sh_i);

    fill_vertical_gradient(
        s,
        sw,
        layout.panel_x,
        layout.panel_y,
        layout.panel_w,
        layout.panel_h,
        GREETER_PANEL_BG,
        GREETER_PANEL_BG_2,
    );
    s_fill(
        s,
        sw,
        layout.panel_x + 2,
        layout.panel_y + 2,
        layout.panel_w - 4,
        1,
        0x00_1A_2A_38,
    );
    draw_glass_panel_outline(
        s,
        sw,
        layout.panel_x,
        layout.panel_y,
        layout.panel_w,
        layout.panel_h,
        ACCENT,
    );

    draw_greeter_avatar(s, sw, layout.avatar_x, layout.avatar_y, layout.avatar_size);
    s_draw_str_scaled_with_tracking(
        s,
        sw,
        layout.title_x,
        layout.title_y,
        "coolOS",
        GREETER_TITLE,
        3,
        0,
    );

    s_draw_str_small_transparent(
        s,
        sw,
        layout.user_x,
        layout.user_y - 17,
        "User",
        0x00_88_CC_EE,
        layout.user_x + layout.field_w,
    );
    draw_greeter_field(
        s,
        sw,
        layout.user_x,
        layout.user_y,
        layout.field_w,
        user,
        focus == GreeterFocus::User,
        GREETER_FIELD_BG,
    );

    let mut masked = String::new();
    for _ in 0..password_len.min(28) {
        masked.push('*');
    }
    if password_len > 28 {
        masked.push_str("...");
    }
    s_draw_str_small_transparent(
        s,
        sw,
        layout.user_x,
        layout.pass_y - 17,
        "Password",
        0x00_88_CC_EE,
        layout.user_x + layout.field_w,
    );
    draw_greeter_field(
        s,
        sw,
        layout.user_x,
        layout.pass_y,
        layout.field_w,
        &masked,
        focus == GreeterFocus::Password,
        GREETER_FIELD_BG,
    );

    let button_hot = rect_contains(
        layout.user_x,
        layout.button_y,
        layout.field_w,
        GREETER_FIELD_H,
        mx,
        my,
    );
    let button_bg = if button_hot {
        blend_color(0x00_0D_17_24, ACCENT, 42)
    } else {
        0x00_0D_17_24
    };
    fill_vertical_gradient(
        s,
        sw,
        layout.user_x,
        layout.button_y,
        layout.field_w,
        GREETER_FIELD_H,
        button_bg,
        blend_color(button_bg, 0x00_02_08_12, 80),
    );
    s_fill(
        s,
        sw,
        layout.user_x,
        layout.button_y,
        layout.field_w,
        2,
        if button_hot { ACCENT_HOV } else { ACCENT },
    );
    s_fill(
        s,
        sw,
        layout.user_x + 1,
        layout.button_y + GREETER_FIELD_H - 2,
        layout.field_w - 2,
        1,
        0x00_05_0B_12,
    );
    draw_rect_border(
        s,
        sw,
        layout.user_x,
        layout.button_y,
        layout.field_w,
        GREETER_FIELD_H,
        if button_hot {
            ACCENT_HOV
        } else {
            0x00_2A_5F_78
        },
    );
    let sign_in = "Sign in";
    let sign_w = sign_in.chars().count() as i32 * 8;
    s_draw_str_small_transparent(
        s,
        sw,
        layout.user_x + (layout.field_w - sign_w) / 2,
        layout.button_y + 11,
        sign_in,
        WHITE,
        layout.user_x + layout.field_w,
    );

    if !message.is_empty() {
        let msg_color = if error { 0x00_FF_88_88 } else { 0x00_88_DD_CC };
        s_draw_str_small_transparent(
            s,
            sw,
            layout.user_x,
            layout.message_y,
            message,
            msg_color,
            layout.user_x + layout.field_w,
        );
    }
    if attempts > 0 {
        let attempt_text = format!("attempts {}", attempts);
        s_draw_str_small_transparent(
            s,
            sw,
            layout.user_x + layout.field_w - 112,
            layout.message_y,
            &attempt_text,
            0x00_66_AA_DD,
            layout.user_x + layout.field_w,
        );
    }

    s_draw_str_small_transparent(
        s,
        sw,
        layout.user_x,
        layout.users_y - 16,
        "Accounts",
        0x00_66_AA_DD,
        layout.user_x + layout.field_w,
    );
    let accounts_bottom = layout.panel_y + layout.panel_h - 8;
    let mut row = 0i32;
    for account in crate::security::users()
        .into_iter()
        .filter(|account| account.login_enabled)
    {
        if row >= 4 {
            break;
        }
        let y = layout.users_y + row * GREETER_USER_ROW_H;
        if y + GREETER_USER_ROW_H > accounts_bottom {
            break;
        }
        let selected = account.name.eq_ignore_ascii_case(user);
        let hot = rect_contains(
            layout.user_x,
            y,
            layout.row_w,
            GREETER_USER_ROW_H - 3,
            mx,
            my,
        );
        draw_greeter_account_chip(
            s,
            sw,
            layout.user_x,
            y,
            layout.row_w,
            &account.name,
            selected,
            hot,
        );
        row += 1;
    }
}

pub(super) fn draw_first_boot_overlay(
    s: &mut [u32],
    sw: usize,
    taskbar_y: i32,
    owner: &str,
    password_len: usize,
    confirm_len: usize,
    device: &str,
    focus: FirstBootFocus,
    message: &str,
    error: bool,
    mx: i32,
    my: i32,
) {
    let sw_i = sw as i32;
    let sh_i = if sw > 0 {
        (s.len() / sw) as i32
    } else {
        taskbar_y
    };
    let layout = first_boot_layout(sw_i, taskbar_y);

    draw_greeter_backdrop(s, sw, sw_i, sh_i);
    fill_vertical_gradient(
        s,
        sw,
        layout.panel_x,
        layout.panel_y,
        layout.panel_w,
        layout.panel_h,
        GREETER_PANEL_BG,
        GREETER_PANEL_BG_2,
    );
    s_fill(
        s,
        sw,
        layout.panel_x + 2,
        layout.panel_y + 2,
        layout.panel_w - 4,
        1,
        0x00_1A_2A_38,
    );
    draw_glass_panel_outline(
        s,
        sw,
        layout.panel_x,
        layout.panel_y,
        layout.panel_w,
        layout.panel_h,
        ACCENT,
    );

    draw_greeter_avatar(s, sw, layout.avatar_x, layout.avatar_y, layout.avatar_size);
    s_draw_str_scaled_with_tracking(
        s,
        sw,
        layout.title_x,
        layout.title_y,
        "Set up coolOS",
        GREETER_TITLE,
        2,
        0,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        layout.title_y + 34,
        "Create the owner account for this install.",
        0x00_9A_C9_D8,
        layout.field_x + layout.field_w,
    );

    draw_first_boot_label(s, sw, &layout, layout.owner_y, "Owner name");
    draw_greeter_field(
        s,
        sw,
        layout.field_x,
        layout.owner_y,
        layout.field_w,
        owner,
        focus == FirstBootFocus::Owner,
        GREETER_FIELD_BG,
    );

    let password_text = masked_password(password_len);
    draw_first_boot_label(s, sw, &layout, layout.pass_y, "Password");
    draw_greeter_field(
        s,
        sw,
        layout.field_x,
        layout.pass_y,
        layout.field_w,
        &password_text,
        focus == FirstBootFocus::Password,
        GREETER_FIELD_BG,
    );

    let masked_confirm = masked_password(confirm_len);
    draw_first_boot_label(s, sw, &layout, layout.confirm_y, "Confirm password");
    draw_greeter_field(
        s,
        sw,
        layout.field_x,
        layout.confirm_y,
        layout.field_w,
        &masked_confirm,
        focus == FirstBootFocus::Confirm,
        GREETER_FIELD_BG,
    );

    draw_first_boot_label(s, sw, &layout, layout.device_y, "Device name");
    draw_greeter_field(
        s,
        sw,
        layout.field_x,
        layout.device_y,
        layout.field_w,
        device,
        focus == FirstBootFocus::Device,
        GREETER_FIELD_BG,
    );

    let button_hot = rect_contains(
        layout.field_x,
        layout.button_y,
        layout.field_w,
        GREETER_FIELD_H,
        mx,
        my,
    );
    let button_bg = if button_hot {
        blend_color(0x00_0D_17_24, ACCENT, 42)
    } else {
        0x00_0D_17_24
    };
    fill_vertical_gradient(
        s,
        sw,
        layout.field_x,
        layout.button_y,
        layout.field_w,
        GREETER_FIELD_H,
        button_bg,
        blend_color(button_bg, 0x00_02_08_12, 80),
    );
    s_fill(
        s,
        sw,
        layout.field_x,
        layout.button_y,
        layout.field_w,
        2,
        if button_hot { ACCENT_HOV } else { ACCENT },
    );
    draw_rect_border(
        s,
        sw,
        layout.field_x,
        layout.button_y,
        layout.field_w,
        GREETER_FIELD_H,
        if button_hot {
            ACCENT_HOV
        } else {
            0x00_2A_5F_78
        },
    );
    let label = "Create account";
    let label_w = label.chars().count() as i32 * 8;
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x + (layout.field_w - label_w) / 2,
        layout.button_y + 11,
        label,
        WHITE,
        layout.field_x + layout.field_w,
    );

    let msg = if message.is_empty() {
        "Tab moves fields. Enter creates the account."
    } else {
        message
    };
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        layout.message_y,
        msg,
        if error { 0x00_FF_88_88 } else { 0x00_88_DD_CC },
        layout.field_x + layout.field_w,
    );
}

pub(super) fn draw_installer_overlay(s: &mut [u32], sw: usize, taskbar_y: i32) {
    let sw_i = sw as i32;
    let sh_i = if sw > 0 {
        (s.len() / sw) as i32
    } else {
        taskbar_y
    };
    let layout = first_boot_layout(sw_i, taskbar_y);

    draw_greeter_backdrop(s, sw, sw_i, sh_i);
    fill_vertical_gradient(
        s,
        sw,
        layout.panel_x,
        layout.panel_y,
        layout.panel_w,
        layout.panel_h,
        GREETER_PANEL_BG,
        GREETER_PANEL_BG_2,
    );
    s_fill(
        s,
        sw,
        layout.panel_x + 2,
        layout.panel_y + 2,
        layout.panel_w - 4,
        1,
        0x00_1A_2A_38,
    );
    draw_glass_panel_outline(
        s,
        sw,
        layout.panel_x,
        layout.panel_y,
        layout.panel_w,
        layout.panel_h,
        ACCENT,
    );

    draw_greeter_avatar(s, sw, layout.avatar_x, layout.avatar_y, layout.avatar_size);
    let title = "Install coolOS";
    let title_w = s_text_width_scaled_with_tracking(title, 2, 0);
    s_draw_str_scaled_with_tracking(
        s,
        sw,
        layout.panel_x + (layout.panel_w - title_w) / 2,
        layout.title_y,
        title,
        GREETER_TITLE,
        2,
        0,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        layout.title_y + 34,
        "Copy live image to blank QEMU disk.",
        0x00_9A_C9_D8,
        layout.field_x + layout.field_w,
    );

    let source = crate::installer::source_device().name();
    let source_line = format!("Source: {} (live root)", source);
    let target_line = "Target: ide1-master (96 MiB+ writable)";
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        layout.owner_y,
        &source_line,
        0x00_C8_D8_E8,
        layout.field_x + layout.field_w,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        layout.owner_y + 24,
        target_line,
        0x00_88_DD_CC,
        layout.field_x + layout.field_w,
    );

    let button_y = layout.owner_y + 70;
    fill_vertical_gradient(
        s,
        sw,
        layout.field_x,
        button_y,
        layout.field_w,
        GREETER_FIELD_H,
        0x00_0D_17_24,
        0x00_03_09_14,
    );
    s_fill(s, sw, layout.field_x, button_y, layout.field_w, 2, ACCENT);
    draw_rect_border(
        s,
        sw,
        layout.field_x,
        button_y,
        layout.field_w,
        GREETER_FIELD_H,
        0x00_2A_5F_78,
    );
    let command = "install disk ide1-master";
    let command_w = command.chars().count() as i32 * 8;
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x + (layout.field_w - command_w) / 2,
        button_y + 11,
        command,
        WHITE,
        layout.field_x + layout.field_w,
    );
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        button_y + GREETER_FIELD_H + 14,
        "Writes a self-booting BIOS target disk.",
        0x00_88_DD_CC,
        layout.field_x + layout.field_w,
    );
}

fn draw_first_boot_label(
    s: &mut [u32],
    sw: usize,
    layout: &FirstBootLayout,
    field_y: i32,
    label: &str,
) {
    s_draw_str_small_transparent(
        s,
        sw,
        layout.field_x,
        field_y - 15,
        label,
        0x00_88_CC_EE,
        layout.field_x + layout.field_w,
    );
}

fn masked_password(len: usize) -> String {
    let mut masked = String::new();
    for _ in 0..len.min(28) {
        masked.push('*');
    }
    if len > 28 {
        masked.push_str("...");
    }
    masked
}

pub(super) fn draw_greeter_backdrop(s: &mut [u32], sw: usize, w: i32, h: i32) {
    if sw == 0 || s.is_empty() {
        return;
    }
    let w = w.max(1).min(sw as i32);
    let h = h.max(1).min((s.len() / sw) as i32);

    for y in 0..h {
        let row = blend_color(
            GREETER_BG_TOP,
            GREETER_BG_BOTTOM,
            (y as u32).saturating_mul(255) / (h - 1).max(1) as u32,
        );
        s_fill(s, sw, 0, y, w, 1, row);
    }

    let glow_cy = h / 2;
    let glow_ry = (h * 7 / 20).max(1);
    for y in (glow_cy - glow_ry)..=(glow_cy + glow_ry) {
        if y < 0 || y >= h {
            continue;
        }
        let dy = (y - glow_cy).abs();
        let remaining = (glow_ry - dy).max(0);
        let strength = (16 * remaining / glow_ry).max(0) as u32;
        let base = blend_color(
            GREETER_BG_TOP,
            GREETER_BG_BOTTOM,
            (y as u32).saturating_mul(255) / (h - 1).max(1) as u32,
        );
        let color = blend_color(base, GREETER_BLOOM, strength);
        s_fill(s, sw, 0, y, w, 1, color);
    }
}

pub(super) fn draw_greeter_avatar(s: &mut [u32], sw: usize, x: i32, y: i32, size: i32) {
    fill_vertical_gradient(s, sw, x, y, size, size, 0x00_12_24_32, 0x00_08_11_1B);
    draw_glass_panel_outline(s, sw, x, y, size, size, ACCENT);
    let logo_scale = 2;
    let logo_size = 18 * logo_scale;
    draw_snowflake_logo(
        s,
        sw,
        x + (size - logo_size) / 2,
        y + (size - logo_size) / 2,
        logo_scale,
        ACCENT,
        ACCENT_HOV,
    );
}

pub(super) fn draw_greeter_account_chip(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    name: &str,
    selected: bool,
    hot: bool,
) {
    let h = GREETER_USER_ROW_H - 3;
    let row_bg = if selected {
        0x00_0D_24_35
    } else if hot {
        0x00_12_1C_29
    } else {
        0x00_0A_11_1A
    };
    s_fill(s, sw, x, y, w, h, row_bg);
    if selected || hot {
        draw_rect_border(
            s,
            sw,
            x,
            y,
            w,
            h,
            if selected { ACCENT } else { 0x00_2B_3A_4A },
        );
    }
    if selected {
        s_fill(s, sw, x, y, 3, h, ACCENT);
    }

    let avatar_x = x + 9;
    let avatar_y = y + 5;
    let avatar_bg = if selected {
        blend_color(0x00_0A_11_1A, ACCENT, 110)
    } else {
        0x00_13_1D_29
    };
    s_fill(s, sw, avatar_x, avatar_y, 16, 16, avatar_bg);
    draw_rect_border(
        s,
        sw,
        avatar_x,
        avatar_y,
        16,
        16,
        if selected { ACCENT_HOV } else { 0x00_2B_3A_4A },
    );
    let initial = name.chars().next().unwrap_or('?').to_ascii_uppercase();
    let mut initial_text = String::new();
    initial_text.push(initial);
    s_draw_str_small(
        s,
        sw,
        avatar_x + 4,
        avatar_y + 5,
        &initial_text,
        WHITE,
        avatar_bg,
        avatar_x + 14,
    );
    s_draw_str_small(
        s,
        sw,
        x + 34,
        y + 9,
        name,
        if selected { WHITE } else { 0x00_D2_E8_F2 },
        row_bg,
        x + w - 10,
    );
}

pub(super) fn draw_greeter_field(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    value: &str,
    active: bool,
    bg: u32,
) {
    let field_bg = if active {
        blend_color(bg, ACCENT, 18)
    } else {
        bg
    };
    fill_vertical_gradient(
        s,
        sw,
        x,
        y,
        w,
        GREETER_FIELD_H,
        field_bg,
        blend_color(field_bg, 0x00_02_06_0C, 70),
    );
    s_fill(
        s,
        sw,
        x,
        y + GREETER_FIELD_H - 2,
        w,
        2,
        if active { ACCENT } else { 0x00_2B_3A_4A },
    );
    draw_rect_border(
        s,
        sw,
        x,
        y,
        w,
        GREETER_FIELD_H,
        if active { ACCENT_HOV } else { 0x00_24_32_42 },
    );
    s_draw_str_small_transparent(s, sw, x + 12, y + 11, value, WHITE, x + w - 12);
}

pub(super) fn greeter_layout(sw: i32, taskbar_y: i32) -> GreeterLayout {
    let panel_w = GREETER_PANEL_W.min((sw - 32).max(300));
    let panel_h = GREETER_PANEL_H.min((taskbar_y - 28).max(320));
    let panel_x = ((sw - panel_w) / 2).max(8);
    let panel_y = ((taskbar_y - panel_h) / 2).max(14);
    let avatar_size = 54;
    let avatar_x = panel_x + (panel_w - avatar_size) / 2;
    let avatar_y = panel_y + 28;
    let title_w = s_text_width_scaled_with_tracking("coolOS", 3, 0);
    let title_x = panel_x + (panel_w - title_w) / 2;
    let title_y = avatar_y + avatar_size + 14;
    let field_w = (panel_w - 92).min(348).max(252);
    let user_x = panel_x + (panel_w - field_w) / 2;
    let user_y = panel_y + 140;
    let pass_y = user_y + 50;
    let button_y = pass_y + 50;
    let message_y = button_y + GREETER_FIELD_H + 7;
    let users_y = (panel_y + panel_h - GREETER_USER_ROW_H * 4 - 8).max(message_y + 24);
    GreeterLayout {
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        avatar_x,
        avatar_y,
        avatar_size,
        title_x,
        title_y,
        user_x,
        user_y,
        field_w,
        pass_y,
        button_y,
        message_y,
        users_y,
        row_w: field_w,
    }
}

pub(super) fn first_boot_layout(sw: i32, taskbar_y: i32) -> FirstBootLayout {
    let panel_w = 500.min((sw - 32).max(320));
    let panel_h = 500.min((taskbar_y - 28).max(420));
    let panel_x = ((sw - panel_w) / 2).max(8);
    let panel_y = ((taskbar_y - panel_h) / 2).max(14);
    let avatar_size = 48;
    let avatar_x = panel_x + (panel_w - avatar_size) / 2;
    let avatar_y = panel_y + 26;
    let title = "Set up coolOS";
    let title_w = s_text_width_scaled_with_tracking(title, 2, 0);
    let title_x = panel_x + (panel_w - title_w) / 2;
    let title_y = avatar_y + avatar_size + 12;
    let field_w = (panel_w - 96).min(372).max(260);
    let field_x = panel_x + (panel_w - field_w) / 2;
    let owner_y = title_y + 70;
    let pass_y = owner_y + 48;
    let confirm_y = pass_y + 48;
    let device_y = confirm_y + 48;
    let button_y = device_y + 48;
    let message_y = button_y + GREETER_FIELD_H + 10;
    FirstBootLayout {
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        avatar_x,
        avatar_y,
        avatar_size,
        title_x,
        title_y,
        field_x,
        field_w,
        owner_y,
        pass_y,
        confirm_y,
        device_y,
        button_y,
        message_y,
    }
}

pub(super) fn rect_contains(x: i32, y: i32, w: i32, h: i32, px: i32, py: i32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}
