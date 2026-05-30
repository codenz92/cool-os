use super::primitives::*;
use super::*;

#[derive(Clone, Copy)]
pub(super) enum DesktopIconKind {
    Terminal,
    SystemMonitor,
    FileManager,
    TextViewer,
    WebBrowser,
    ColorPicker,
    Notes,
    Screenshot,
    DisplaySettings,
    Welcome,
    Trash,
    Generic,
}

pub(super) fn desktop_icon_kind(title: &str) -> DesktopIconKind {
    match canonical_app_title(title) {
        "Terminal" => DesktopIconKind::Terminal,
        "System Monitor" => DesktopIconKind::SystemMonitor,
        "File Manager" => DesktopIconKind::FileManager,
        "Text Viewer" | "Text Editor" => DesktopIconKind::TextViewer,
        "Web Browser" => DesktopIconKind::WebBrowser,
        "Color Picker" => DesktopIconKind::ColorPicker,
        "Notes" => DesktopIconKind::Notes,
        "Screenshot" => DesktopIconKind::Screenshot,
        "Display Settings" => DesktopIconKind::DisplaySettings,
        "Welcome" => DesktopIconKind::Welcome,
        "Trash Bin" => DesktopIconKind::Trash,
        _ => DesktopIconKind::Generic,
    }
}

pub(super) fn desktop_icon_accent(kind: DesktopIconKind) -> u32 {
    match kind {
        DesktopIconKind::Terminal => ICON_TERM_ACC,
        DesktopIconKind::SystemMonitor => ICON_MON_ACC,
        DesktopIconKind::FileManager => 0x00_54_C7_EB,
        DesktopIconKind::TextViewer => ICON_TXT_ACC,
        DesktopIconKind::WebBrowser => 0x00_5C_D4_A4,
        DesktopIconKind::ColorPicker => ICON_COL_ACC,
        DesktopIconKind::Notes => 0x00_F5_CB_6B,
        DesktopIconKind::Screenshot => 0x00_78_C8_E8,
        DesktopIconKind::DisplaySettings => 0x00_66_CC_FF,
        DesktopIconKind::Welcome => 0x00_88_CC_FF,
        DesktopIconKind::Trash => 0x00_A8_B5_C2,
        DesktopIconKind::Generic => ACCENT,
    }
}

pub(super) fn canonical_app_title(name: &str) -> &str {
    match name {
        "Terminal" => "Terminal",
        "System Mon" | "System Monitor" => "System Monitor",
        "Diag" | "Diagnostics" => "Diagnostics",
        "Text View" | "Text Viewer" => "Text Viewer",
        "Editor" | "Text Edit" | "Text Editor" => "Text Editor",
        "Note" | "Notes" => "Notes",
        "Trash" | "Trash Bin" => "Trash Bin",
        "Shot" | "Screenshot" => "Screenshot",
        "Browser" | "Web" | "Web Browser" => "Web Browser",
        "Color Pick" | "Color Picker" => "Color Picker",
        "Display Settings" => "Display Settings",
        "Account" | "Accounts" | "Users" => "Accounts",
        "Personalize" => "Personalize",
        "Crash Viewer" => "Crash Viewer",
        "Log Viewer" => "Log Viewer",
        "Boot Profiler" => "Boot Profiler",
        "Welcome" => "Welcome",
        "Gui Demo" | "GUI Demo" | "Userspace GUI" => "GUI Demo",
        "Proc Demo" | "Process Demo" => "Process Demo",
        "File Mgr" | "File Manager" => "File Manager",
        _ => name,
    }
}

pub(super) fn window_accent(title: &str) -> u32 {
    match canonical_app_title(title) {
        "Terminal" => ICON_TERM_ACC,
        "System Monitor" => ICON_MON_ACC,
        "Diagnostics" => 0x00_55_FF_CC,
        "Text Viewer" => ICON_TXT_ACC,
        "Text Editor" => 0x00_88_FF_CC,
        "Notes" => 0x00_FF_DD_66,
        "Trash Bin" => 0x00_99_BB_CC,
        "Screenshot" => 0x00_77_DD_FF,
        "Web Browser" => 0x00_33_CC_99,
        "Color Picker" => ICON_COL_ACC,
        "Display Settings" => 0x00_66_CC_FF,
        "Accounts" => 0x00_00_FF_AA,
        "Personalize" => 0x00_CC_66_FF,
        "Crash Viewer" => 0x00_FF_66_66,
        "Log Viewer" => 0x00_55_FF_BB,
        "Boot Profiler" => 0x00_FF_DD_55,
        "Welcome" => 0x00_88_CC_FF,
        "GUI Demo" => 0x00_88_FF_CC,
        "Process Demo" => 0x00_FF_CC_66,
        "File Manager" => 0x00_55_DD_FF,
        _ => ACCENT,
    }
}

pub(super) fn user_gui_window_title(title: &str) -> &'static str {
    match title {
        "editor" | "Editor" | "Text Edit" | "Text Editor" => "Text Editor",
        "note" | "notes" | "Note" | "Notes" => "Notes",
        "trash" | "Trash" | "Trash Bin" => "Trash Bin",
        "shot" | "screenshot" | "Screenshot" => "Screenshot",
        "guidemo" | "GUI Demo" | "Gui Demo" | "Userspace GUI" => "GUI Demo",
        _ => "Userspace GUI",
    }
}

#[allow(dead_code)]

pub(super) fn draw_snowflake_logo(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    scale: i32,
    primary: u32,
    secondary: u32,
) {
    for rect in crate::branding::SNOWFLAKE_LOGO_RECTS.iter() {
        let color = if rect.highlight { secondary } else { primary };
        s_fill(
            s,
            sw,
            x + rect.x * scale,
            y + rect.y * scale,
            rect.w * scale,
            rect.h * scale,
            color,
        );
    }
}

pub(super) fn draw_desktop_icon_plate(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    selected: bool,
    accent: u32,
) {
    let px = x - 4;
    let py = y - 3;
    let w = ICON_SIZE + 8;
    let h = ICON_SIZE + 6;
    let top = if selected {
        blend_color(0x00_14_21_2E, accent, 34)
    } else {
        0x00_10_18_22
    };
    let bottom = if selected {
        blend_color(0x00_08_0F_18, accent, 14)
    } else {
        0x00_08_0F_18
    };
    fill_vertical_gradient(s, sw, px, py, w, h, top, bottom);
    draw_glass_panel_outline(
        s,
        sw,
        px,
        py,
        w,
        h,
        if selected {
            blend_color(accent, WHITE, 58)
        } else {
            blend_color(accent, 0x00_3B_4C_5E, 72)
        },
    );
    s_fill(
        s,
        sw,
        px + 5,
        py + h - 4,
        w - 10,
        1,
        blend_color(accent, bottom, 116),
    );
}

pub(super) fn draw_start_menu_app_icon(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    kind: DesktopIconKind,
) {
    match kind {
        DesktopIconKind::Terminal => {
            fill_vertical_gradient(s, sw, x + 3, y + 4, 18, 14, 0x00_14_2C_36, 0x00_05_10_16);
            draw_rect_border(
                s,
                sw,
                x + 3,
                y + 4,
                18,
                14,
                blend_color(ICON_TERM_ACC, WHITE, 76),
            );
            s_fill(s, sw, x + 6, y + 10, 3, 1, ICON_TERM_ACC);
            s_fill(s, sw, x + 8, y + 11, 3, 1, ICON_TERM_ACC);
            s_fill(s, sw, x + 6, y + 12, 3, 1, ICON_TERM_ACC);
            s_fill(
                s,
                sw,
                x + 13,
                y + 14,
                5,
                1,
                blend_color(ICON_TERM_ACC, WHITE, 70),
            );
            s_fill(s, sw, x + 9, y + 19, 6, 2, 0x00_18_2A_34);
            s_fill(
                s,
                sw,
                x + 6,
                y + 21,
                12,
                1,
                blend_color(ICON_TERM_ACC, BLACK, 112),
            );
        }
        DesktopIconKind::SystemMonitor => {
            fill_vertical_gradient(s, sw, x + 2, y + 4, 20, 14, 0x00_1E_3A_4A, 0x00_07_13_20);
            draw_rect_border(
                s,
                sw,
                x + 2,
                y + 4,
                20,
                14,
                blend_color(ICON_MON_ACC, WHITE, 64),
            );
            s_fill(s, sw, x + 5, y + 13, 3, 3, 0x00_4C_DD_A1);
            s_fill(s, sw, x + 10, y + 9, 3, 7, ICON_MON_ACC);
            s_fill(s, sw, x + 15, y + 7, 3, 9, 0x00_AA_7C_F7);
            draw_icon_line(s, sw, x + 5, y + 12, x + 10, y + 9, 0x00_E8_FB_FF);
            draw_icon_line(s, sw, x + 10, y + 9, x + 17, y + 11, 0x00_E8_FB_FF);
            s_fill(s, sw, x + 11, y + 18, 2, 2, 0x00_36_58_68);
            s_fill(
                s,
                sw,
                x + 7,
                y + 20,
                10,
                1,
                blend_color(ICON_MON_ACC, BLACK, 92),
            );
        }
        DesktopIconKind::DisplaySettings => {
            fill_vertical_gradient(s, sw, x + 2, y + 4, 19, 13, 0x00_1B_3D_52, 0x00_08_14_20);
            draw_rect_border(s, sw, x + 2, y + 4, 19, 13, 0x00_A6_EB_FF);
            s_fill(s, sw, x + 5, y + 8, 13, 2, 0x00_66_CC_FF);
            s_fill(s, sw, x + 5, y + 12, 8, 2, 0x00_7D_E7_F7);
            s_fill(s, sw, x + 10, y + 17, 3, 2, 0x00_36_58_68);
            s_fill(s, sw, x + 6, y + 20, 11, 1, 0x00_66_CC_FF);
            draw_filled_circle(s, sw, x + 18, y + 17, 4, 0x00_08_14_20);
            draw_circle_outline(s, sw, x + 18, y + 17, 4, 0x00_D8_F7_FF);
            s_fill(s, sw, x + 17, y + 16, 3, 3, 0x00_66_CC_FF);
        }
        DesktopIconKind::FileManager => {
            s_fill(s, sw, x + 4, y + 6, 8, 4, 0x00_78_D7_F2);
            s_fill(s, sw, x + 3, y + 9, 18, 4, 0x00_43_B6_E7);
            fill_vertical_gradient(s, sw, x + 2, y + 12, 20, 10, 0x00_65_D3_F3, 0x00_1B_7E_B8);
            draw_rect_border(s, sw, x + 2, y + 12, 20, 10, 0x00_B8_F4_FF);
            s_fill(s, sw, x + 6, y + 16, 11, 1, 0x00_0D_3B_5F);
            s_fill(s, sw, x + 6, y + 19, 8, 1, 0x00_0D_3B_5F);
        }
        DesktopIconKind::TextViewer | DesktopIconKind::Generic => {
            s_fill(s, sw, x + 7, y + 4, 14, 18, 0x00_05_0B_12);
            fill_vertical_gradient(s, sw, x + 5, y + 2, 14, 18, 0x00_F2_FA_FF, 0x00_B7_C8_E4);
            draw_rect_border(s, sw, x + 5, y + 2, 14, 18, 0x00_6E_90_C6);
            s_fill(s, sw, x + 15, y + 3, 3, 3, 0x00_D7_E4_F7);
            s_fill(s, sw, x + 8, y + 8, 8, 1, 0x00_5D_78_AA);
            s_fill(s, sw, x + 8, y + 12, 8, 1, 0x00_7E_94_BA);
            s_fill(s, sw, x + 8, y + 16, 6, 1, 0x00_7E_94_BA);
        }
        DesktopIconKind::Welcome => {
            fill_vertical_gradient(s, sw, x + 5, y + 3, 14, 18, 0x00_F2_FA_FF, 0x00_B7_DD_F5);
            draw_rect_border(s, sw, x + 5, y + 3, 14, 18, 0x00_88_CC_FF);
            s_fill(s, sw, x + 8, y + 9, 8, 1, 0x00_5D_78_AA);
            s_fill(s, sw, x + 8, y + 13, 6, 1, 0x00_7E_94_BA);
            s_fill(s, sw, x + 16, y + 4, 1, 5, 0x00_FF_EA_84);
            s_fill(s, sw, x + 14, y + 6, 5, 1, 0x00_FF_EA_84);
            s_fill(
                s,
                sw,
                x + 15,
                y + 5,
                3,
                3,
                blend_color(0x00_FF_EA_84, WHITE, 64),
            );
        }
        DesktopIconKind::WebBrowser => {
            draw_filled_circle(s, sw, x + 12, y + 12, 10, 0x00_2B_A8_F2);
            draw_filled_circle(s, sw, x + 9, y + 11, 5, 0x00_36_D0_8C);
            draw_filled_circle(s, sw, x + 16, y + 15, 4, 0x00_2C_D2_B3);
            s_fill(s, sw, x + 4, y + 12, 17, 1, 0x00_DD_FB_FF);
            s_fill(
                s,
                sw,
                x + 11,
                y + 4,
                1,
                17,
                blend_color(WHITE, 0x00_2B_A8_F2, 92),
            );
            draw_circle_outline(s, sw, x + 12, y + 12, 10, 0x00_B8_F4_FF);
        }
        DesktopIconKind::ColorPicker => {
            draw_filled_circle(s, sw, x + 10, y + 12, 9, 0x00_2B_1F_3E);
            draw_circle_outline(
                s,
                sw,
                x + 10,
                y + 12,
                9,
                blend_color(ICON_COL_ACC, WHITE, 74),
            );
            draw_filled_circle(s, sw, x + 7, y + 9, 2, 0x00_FF_5D_5D);
            draw_filled_circle(s, sw, x + 12, y + 8, 2, 0x00_6D_FF_7A);
            draw_filled_circle(s, sw, x + 15, y + 13, 2, 0x00_67_B7_FF);
            draw_filled_circle(s, sw, x + 8, y + 16, 2, 0x00_FF_DD_66);
            draw_icon_line(s, sw, x + 15, y + 6, x + 22, y + 13, 0x00_D9_E6_F2);
            s_fill(s, sw, x + 20, y + 12, 3, 3, ICON_COL_ACC);
        }
        DesktopIconKind::Notes => {
            fill_vertical_gradient(s, sw, x + 5, y + 3, 16, 18, 0x00_FF_EA_84, 0x00_DD_AB_36);
            draw_rect_border(s, sw, x + 5, y + 3, 16, 18, 0x00_FF_F5_B8);
            s_fill(s, sw, x + 8, y + 8, 9, 1, 0x00_76_57_17);
            s_fill(s, sw, x + 8, y + 12, 9, 1, 0x00_76_57_17);
            s_fill(s, sw, x + 8, y + 16, 6, 1, 0x00_76_57_17);
            s_fill(s, sw, x + 17, y + 17, 4, 4, 0x00_BC_88_25);
        }
        DesktopIconKind::Screenshot => {
            fill_vertical_gradient(s, sw, x + 4, y + 8, 17, 11, 0x00_38_BD_EA, 0x00_0C_4D_73);
            s_fill(s, sw, x + 8, y + 5, 8, 3, 0x00_73_DA_F5);
            draw_rect_border(s, sw, x + 4, y + 8, 17, 11, 0x00_B8_F4_FF);
            draw_filled_circle(s, sw, x + 13, y + 14, 5, 0x00_05_13_1C);
            draw_filled_circle(s, sw, x + 13, y + 14, 3, 0x00_8A_EB_F7);
            s_fill(s, sw, x + 3, y + 4, 6, 1, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 3, y + 4, 1, 6, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 17, y + 4, 6, 1, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 22, y + 4, 1, 6, 0x00_EA_FC_FF);
        }
        DesktopIconKind::Trash => {
            s_fill(s, sw, x + 8, y + 4, 8, 3, 0x00_D8_E5_EF);
            s_fill(s, sw, x + 6, y + 7, 13, 3, 0x00_A8_B8_C8);
            fill_vertical_gradient(s, sw, x + 7, y + 10, 11, 12, 0x00_CE_DA_E4, 0x00_6E_83_96);
            draw_rect_border(s, sw, x + 7, y + 10, 11, 12, 0x00_EA_F2_F8);
            s_fill(s, sw, x + 10, y + 12, 1, 8, 0x00_4A_5D_70);
            s_fill(s, sw, x + 14, y + 12, 1, 8, 0x00_4A_5D_70);
        }
    }
}

pub(super) fn draw_shell_app_icon(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    size: i32,
    kind: DesktopIconKind,
) {
    if size >= 24 {
        let offset = (size - 24) / 2;
        draw_start_menu_app_icon(s, sw, x + offset, y + offset, kind);
        return;
    }

    let accent = desktop_icon_accent(kind);
    match kind {
        DesktopIconKind::Terminal => {
            fill_vertical_gradient(
                s,
                sw,
                x + 1,
                y + 2,
                size - 2,
                size - 5,
                0x00_14_2C_36,
                0x00_05_10_16,
            );
            draw_rect_border(
                s,
                sw,
                x + 1,
                y + 2,
                size - 2,
                size - 5,
                blend_color(accent, WHITE, 80),
            );
            s_fill(s, sw, x + 4, y + 7, 3, 1, accent);
            s_fill(s, sw, x + 6, y + 8, 3, 1, accent);
            s_fill(s, sw, x + 4, y + 9, 3, 1, accent);
            s_fill(
                s,
                sw,
                x + size - 8,
                y + size - 6,
                5,
                1,
                blend_color(accent, WHITE, 90),
            );
        }
        DesktopIconKind::SystemMonitor | DesktopIconKind::DisplaySettings => {
            fill_vertical_gradient(
                s,
                sw,
                x + 1,
                y + 2,
                size - 2,
                size - 6,
                0x00_1E_3A_4A,
                0x00_07_13_20,
            );
            draw_rect_border(
                s,
                sw,
                x + 1,
                y + 2,
                size - 2,
                size - 6,
                blend_color(accent, WHITE, 76),
            );
            s_fill(s, sw, x + 4, y + size - 8, 2, 4, 0x00_4C_DD_A1);
            s_fill(s, sw, x + 8, y + size - 11, 2, 7, ICON_MON_ACC);
            s_fill(s, sw, x + 12, y + size - 13, 2, 9, 0x00_AA_7C_F7);
            s_fill(s, sw, x + size / 2 - 1, y + size - 4, 2, 2, 0x00_36_58_68);
        }
        DesktopIconKind::FileManager => {
            s_fill(s, sw, x + 3, y + 3, 6, 3, 0x00_78_D7_F2);
            s_fill(s, sw, x + 2, y + 5, size - 4, 4, 0x00_43_B6_E7);
            fill_vertical_gradient(
                s,
                sw,
                x + 1,
                y + 8,
                size - 2,
                size - 10,
                0x00_65_D3_F3,
                0x00_1B_7E_B8,
            );
            draw_rect_border(s, sw, x + 1, y + 8, size - 2, size - 10, 0x00_B8_F4_FF);
        }
        DesktopIconKind::WebBrowser => {
            let r = (size / 2 - 2).max(5);
            draw_filled_circle(s, sw, x + size / 2, y + size / 2, r, 0x00_2B_A8_F2);
            draw_filled_circle(s, sw, x + size / 2 - 2, y + size / 2, r / 2, 0x00_36_D0_8C);
            s_fill(s, sw, x + 3, y + size / 2, size - 6, 1, 0x00_DD_FB_FF);
            s_fill(
                s,
                sw,
                x + size / 2,
                y + 3,
                1,
                size - 6,
                blend_color(WHITE, 0x00_2B_A8_F2, 92),
            );
            draw_circle_outline(s, sw, x + size / 2, y + size / 2, r, 0x00_B8_F4_FF);
        }
        DesktopIconKind::ColorPicker => {
            draw_filled_circle(
                s,
                sw,
                x + size / 2 - 1,
                y + size / 2,
                size / 2 - 3,
                0x00_2B_1F_3E,
            );
            draw_circle_outline(
                s,
                sw,
                x + size / 2 - 1,
                y + size / 2,
                size / 2 - 3,
                blend_color(accent, WHITE, 80),
            );
            draw_filled_circle(s, sw, x + 5, y + 6, 2, 0x00_FF_5D_5D);
            draw_filled_circle(s, sw, x + 10, y + 5, 2, 0x00_6D_FF_7A);
            draw_filled_circle(s, sw, x + 12, y + 10, 2, 0x00_67_B7_FF);
            draw_filled_circle(s, sw, x + 6, y + 12, 2, 0x00_FF_DD_66);
        }
        DesktopIconKind::Notes => {
            fill_vertical_gradient(
                s,
                sw,
                x + 3,
                y + 2,
                size - 6,
                size - 4,
                0x00_FF_EA_84,
                0x00_DD_AB_36,
            );
            draw_rect_border(s, sw, x + 3, y + 2, size - 6, size - 4, 0x00_FF_F5_B8);
            s_fill(s, sw, x + 6, y + 7, size - 11, 1, 0x00_76_57_17);
            s_fill(s, sw, x + 6, y + 10, size - 11, 1, 0x00_76_57_17);
        }
        DesktopIconKind::Screenshot => {
            fill_vertical_gradient(
                s,
                sw,
                x + 2,
                y + 6,
                size - 4,
                size - 8,
                0x00_38_BD_EA,
                0x00_0C_4D_73,
            );
            s_fill(s, sw, x + 6, y + 4, size - 11, 3, 0x00_73_DA_F5);
            draw_rect_border(s, sw, x + 2, y + 6, size - 4, size - 8, 0x00_B8_F4_FF);
            draw_filled_circle(s, sw, x + size / 2, y + size / 2 + 1, 4, 0x00_05_13_1C);
            draw_filled_circle(s, sw, x + size / 2, y + size / 2 + 1, 2, 0x00_8A_EB_F7);
        }
        DesktopIconKind::Trash => {
            s_fill(s, sw, x + 5, y + 2, size - 10, 3, 0x00_D8_E5_EF);
            s_fill(s, sw, x + 3, y + 5, size - 6, 3, 0x00_A8_B8_C8);
            fill_vertical_gradient(
                s,
                sw,
                x + 4,
                y + 8,
                size - 8,
                size - 10,
                0x00_CE_DA_E4,
                0x00_6E_83_96,
            );
            draw_rect_border(s, sw, x + 4, y + 8, size - 8, size - 10, 0x00_EA_F2_F8);
            s_fill(s, sw, x + 7, y + 10, 1, size - 13, 0x00_4A_5D_70);
            s_fill(s, sw, x + size - 8, y + 10, 1, size - 13, 0x00_4A_5D_70);
        }
        DesktopIconKind::Welcome | DesktopIconKind::TextViewer | DesktopIconKind::Generic => {
            s_fill(s, sw, x + 5, y + 3, size - 7, size - 5, 0x00_05_0B_12);
            fill_vertical_gradient(
                s,
                sw,
                x + 3,
                y + 2,
                size - 7,
                size - 5,
                0x00_F2_FA_FF,
                0x00_B7_C8_E4,
            );
            draw_rect_border(s, sw, x + 3, y + 2, size - 7, size - 5, 0x00_6E_90_C6);
            s_fill(s, sw, x + 6, y + 7, size - 12, 1, 0x00_5D_78_AA);
            s_fill(s, sw, x + 6, y + 10, size - 12, 1, 0x00_7E_94_BA);
            if matches!(kind, DesktopIconKind::Welcome) {
                s_fill(s, sw, x + size - 5, y + 3, 1, 5, 0x00_FF_EA_84);
                s_fill(s, sw, x + size - 7, y + 5, 5, 1, 0x00_FF_EA_84);
            }
        }
    }
}

pub(super) fn draw_desktop_app_icon(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    kind: DesktopIconKind,
) {
    match kind {
        DesktopIconKind::Terminal => {
            fill_vertical_gradient(s, sw, x + 3, y + 6, 34, 27, 0x00_14_2C_36, 0x00_05_10_16);
            draw_rect_border(
                s,
                sw,
                x + 3,
                y + 6,
                34,
                27,
                blend_color(ICON_TERM_ACC, WHITE, 76),
            );
            draw_rect_border(s, sw, x + 5, y + 8, 30, 23, 0x00_13_24_2E);
            s_fill(
                s,
                sw,
                x + 6,
                y + 9,
                28,
                2,
                blend_color(ICON_TERM_ACC, WHITE, 32),
            );
            s_fill(s, sw, x + 10, y + 17, 4, 2, ICON_TERM_ACC);
            s_fill(s, sw, x + 14, y + 19, 4, 2, ICON_TERM_ACC);
            s_fill(s, sw, x + 10, y + 21, 4, 2, ICON_TERM_ACC);
            s_fill(
                s,
                sw,
                x + 21,
                y + 24,
                10,
                2,
                blend_color(ICON_TERM_ACC, WHITE, 70),
            );
            s_fill(s, sw, x + 15, y + 34, 10, 3, 0x00_18_2A_34);
            s_fill(
                s,
                sw,
                x + 10,
                y + 37,
                20,
                2,
                blend_color(ICON_TERM_ACC, BLACK, 112),
            );
        }
        DesktopIconKind::SystemMonitor => {
            fill_vertical_gradient(s, sw, x + 2, y + 7, 36, 25, 0x00_1E_3A_4A, 0x00_07_13_20);
            draw_rect_border(
                s,
                sw,
                x + 2,
                y + 7,
                36,
                25,
                blend_color(ICON_MON_ACC, WHITE, 64),
            );
            fill_vertical_gradient(s, sw, x + 5, y + 10, 30, 18, 0x00_0C_1A_28, 0x00_08_10_18);
            s_fill(s, sw, x + 9, y + 23, 4, 4, 0x00_4C_DD_A1);
            s_fill(s, sw, x + 16, y + 17, 4, 10, ICON_MON_ACC);
            s_fill(s, sw, x + 23, y + 13, 4, 14, 0x00_AA_7C_F7);
            s_fill(s, sw, x + 30, y + 20, 3, 7, 0x00_F5_CB_6B);
            draw_icon_line_thick(s, sw, x + 8, y + 22, x + 16, y + 16, 0x00_E8_FB_FF);
            draw_icon_line_thick(s, sw, x + 16, y + 16, x + 25, y + 19, 0x00_E8_FB_FF);
            draw_icon_line_thick(s, sw, x + 25, y + 19, x + 33, y + 12, 0x00_E8_FB_FF);
            s_fill(s, sw, x + 18, y + 32, 4, 4, 0x00_36_58_68);
            s_fill(
                s,
                sw,
                x + 11,
                y + 36,
                18,
                2,
                blend_color(ICON_MON_ACC, BLACK, 92),
            );
        }
        DesktopIconKind::DisplaySettings => {
            fill_vertical_gradient(s, sw, x + 3, y + 8, 34, 24, 0x00_1B_3D_52, 0x00_08_14_20);
            draw_rect_border(s, sw, x + 3, y + 8, 34, 24, 0x00_A6_EB_FF);
            s_fill(s, sw, x + 8, y + 15, 24, 3, 0x00_66_CC_FF);
            s_fill(s, sw, x + 8, y + 23, 15, 3, 0x00_7D_E7_F7);
            s_fill(s, sw, x + 18, y + 32, 4, 4, 0x00_36_58_68);
            s_fill(
                s,
                sw,
                x + 11,
                y + 37,
                18,
                2,
                blend_color(0x00_66_CC_FF, BLACK, 92),
            );
            draw_filled_circle(s, sw, x + 33, y + 32, 7, 0x00_08_14_20);
            draw_circle_outline(s, sw, x + 33, y + 32, 7, 0x00_D8_F7_FF);
            s_fill(s, sw, x + 31, y + 30, 5, 5, 0x00_66_CC_FF);
        }
        DesktopIconKind::FileManager => {
            s_fill(s, sw, x + 6, y + 9, 14, 7, 0x00_78_D7_F2);
            s_fill(s, sw, x + 5, y + 13, 31, 7, 0x00_43_B6_E7);
            fill_vertical_gradient(s, sw, x + 3, y + 17, 34, 20, 0x00_65_D3_F3, 0x00_1B_7E_B8);
            draw_rect_border(s, sw, x + 3, y + 17, 34, 20, 0x00_B8_F4_FF);
            s_fill(
                s,
                sw,
                x + 6,
                y + 19,
                28,
                2,
                blend_color(WHITE, 0x00_65_D3_F3, 82),
            );
            s_fill(s, sw, x + 9, y + 25, 22, 2, 0x00_0D_3B_5F);
            s_fill(s, sw, x + 9, y + 30, 16, 2, 0x00_0D_3B_5F);
            s_fill(
                s,
                sw,
                x + 5,
                y + 36,
                30,
                2,
                blend_color(0x00_1B_7E_B8, BLACK, 80),
            );
        }
        DesktopIconKind::Welcome => {
            s_fill(s, sw, x + 11, y + 8, 25, 31, 0x00_05_0B_12);
            fill_vertical_gradient(s, sw, x + 8, y + 5, 25, 31, 0x00_F2_FA_FF, 0x00_B7_DD_F5);
            draw_rect_border(s, sw, x + 8, y + 5, 25, 31, 0x00_88_CC_FF);
            s_fill(s, sw, x + 12, y + 15, 16, 2, 0x00_5D_78_AA);
            s_fill(s, sw, x + 12, y + 22, 13, 2, 0x00_7E_94_BA);
            s_fill(s, sw, x + 30, y + 7, 2, 10, 0x00_FF_EA_84);
            s_fill(s, sw, x + 26, y + 11, 10, 2, 0x00_FF_EA_84);
            s_fill(
                s,
                sw,
                x + 28,
                y + 9,
                6,
                6,
                blend_color(0x00_FF_EA_84, WHITE, 64),
            );
        }
        DesktopIconKind::TextViewer | DesktopIconKind::Generic => {
            s_fill(s, sw, x + 11, y + 8, 25, 31, 0x00_05_0B_12);
            fill_vertical_gradient(s, sw, x + 8, y + 5, 25, 31, 0x00_F2_FA_FF, 0x00_B7_C8_E4);
            draw_rect_border(s, sw, x + 8, y + 5, 25, 31, 0x00_6E_90_C6);
            s_fill(s, sw, x + 27, y + 6, 5, 5, 0x00_D7_E4_F7);
            s_fill(s, sw, x + 29, y + 8, 3, 3, 0x00_8F_A8_D5);
            s_fill(s, sw, x + 12, y + 14, 16, 2, 0x00_5D_78_AA);
            s_fill(s, sw, x + 12, y + 20, 17, 2, 0x00_7E_94_BA);
            s_fill(s, sw, x + 12, y + 26, 12, 2, 0x00_7E_94_BA);
            s_fill(s, sw, x + 12, y + 31, 16, 2, 0x00_7E_94_BA);
        }
        DesktopIconKind::WebBrowser => {
            draw_filled_circle(s, sw, x + 20, y + 21, 18, 0x00_04_13_1C);
            draw_filled_circle(s, sw, x + 20, y + 20, 16, 0x00_2B_A8_F2);
            draw_filled_circle(s, sw, x + 15, y + 19, 9, 0x00_36_D0_8C);
            draw_filled_circle(s, sw, x + 27, y + 25, 7, 0x00_2C_D2_B3);
            s_fill(s, sw, x + 7, y + 20, 26, 2, 0x00_DD_FB_FF);
            s_fill(
                s,
                sw,
                x + 10,
                y + 13,
                20,
                1,
                blend_color(WHITE, 0x00_2B_A8_F2, 70),
            );
            s_fill(
                s,
                sw,
                x + 10,
                y + 28,
                20,
                1,
                blend_color(WHITE, 0x00_2B_A8_F2, 70),
            );
            s_fill(
                s,
                sw,
                x + 19,
                y + 7,
                2,
                29,
                blend_color(WHITE, 0x00_2B_A8_F2, 92),
            );
            draw_circle_outline(s, sw, x + 20, y + 20, 16, 0x00_B8_F4_FF);
        }
        DesktopIconKind::ColorPicker => {
            draw_filled_circle(s, sw, x + 18, y + 21, 16, 0x00_2B_1F_3E);
            draw_circle_outline(
                s,
                sw,
                x + 18,
                y + 21,
                16,
                blend_color(ICON_COL_ACC, WHITE, 74),
            );
            draw_filled_circle(s, sw, x + 12, y + 16, 3, 0x00_FF_5D_5D);
            draw_filled_circle(s, sw, x + 21, y + 13, 3, 0x00_6D_FF_7A);
            draw_filled_circle(s, sw, x + 27, y + 20, 3, 0x00_67_B7_FF);
            draw_filled_circle(s, sw, x + 13, y + 27, 3, 0x00_FF_DD_66);
            draw_filled_circle(s, sw, x + 24, y + 29, 5, 0x00_07_10_18);
            draw_icon_line_thick(s, sw, x + 25, y + 10, x + 36, y + 21, 0x00_D9_E6_F2);
            s_fill(s, sw, x + 33, y + 19, 5, 5, ICON_COL_ACC);
        }
        DesktopIconKind::Notes => {
            fill_vertical_gradient(s, sw, x + 7, y + 6, 27, 31, 0x00_FF_EA_84, 0x00_DD_AB_36);
            draw_rect_border(s, sw, x + 7, y + 6, 27, 31, 0x00_FF_F5_B8);
            s_fill(s, sw, x + 10, y + 13, 19, 2, 0x00_76_57_17);
            s_fill(s, sw, x + 10, y + 20, 18, 2, 0x00_76_57_17);
            s_fill(s, sw, x + 10, y + 27, 13, 2, 0x00_76_57_17);
            s_fill(s, sw, x + 27, y + 30, 7, 7, 0x00_BC_88_25);
            s_fill(s, sw, x + 29, y + 32, 5, 5, 0x00_8E_65_1B);
        }
        DesktopIconKind::Screenshot => {
            fill_vertical_gradient(s, sw, x + 6, y + 14, 29, 20, 0x00_38_BD_EA, 0x00_0C_4D_73);
            s_fill(s, sw, x + 12, y + 10, 13, 5, 0x00_73_DA_F5);
            s_fill(s, sw, x + 27, y + 12, 5, 3, 0x00_E8_FB_FF);
            draw_rect_border(s, sw, x + 6, y + 14, 29, 20, 0x00_B8_F4_FF);
            draw_filled_circle(s, sw, x + 21, y + 24, 8, 0x00_05_13_1C);
            draw_filled_circle(s, sw, x + 21, y + 24, 5, 0x00_8A_EB_F7);
            draw_circle_outline(s, sw, x + 21, y + 24, 8, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 5, y + 8, 8, 2, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 5, y + 8, 2, 8, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 31, y + 8, 8, 2, 0x00_EA_FC_FF);
            s_fill(s, sw, x + 37, y + 8, 2, 8, 0x00_EA_FC_FF);
        }
        DesktopIconKind::Trash => {
            s_fill(s, sw, x + 14, y + 6, 12, 4, 0x00_D8_E5_EF);
            s_fill(s, sw, x + 9, y + 10, 24, 4, 0x00_A8_B8_C8);
            fill_vertical_gradient(s, sw, x + 11, y + 15, 20, 23, 0x00_CE_DA_E4, 0x00_6E_83_96);
            draw_rect_border(s, sw, x + 11, y + 15, 20, 23, 0x00_EA_F2_F8);
            s_fill(s, sw, x + 15, y + 19, 2, 15, 0x00_4A_5D_70);
            s_fill(s, sw, x + 20, y + 19, 2, 15, 0x00_4A_5D_70);
            s_fill(s, sw, x + 25, y + 19, 2, 15, 0x00_4A_5D_70);
            s_fill(
                s,
                sw,
                x + 13,
                y + 36,
                16,
                2,
                blend_color(0x00_6E_83_96, BLACK, 76),
            );
        }
    }
}

pub(super) fn draw_circle_outline(s: &mut [u32], sw: usize, cx: i32, cy: i32, r: i32, color: u32) {
    let mut x = r;
    let mut y = 0;
    let mut err = 0;
    let sh = if sw > 0 { s.len() / sw } else { 0 };
    while x >= y {
        s_put(s, sw, sh, cx + x, cy + y, color);
        s_put(s, sw, sh, cx + y, cy + x, color);
        s_put(s, sw, sh, cx - y, cy + x, color);
        s_put(s, sw, sh, cx - x, cy + y, color);
        s_put(s, sw, sh, cx - x, cy - y, color);
        s_put(s, sw, sh, cx - y, cy - x, color);
        s_put(s, sw, sh, cx + y, cy - x, color);
        s_put(s, sw, sh, cx + x, cy - y, color);
        y += 1;
        if err <= 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}
