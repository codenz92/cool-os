extern crate alloc;

use alloc::string::String;

use crate::apps::theme;
use crate::desktop_settings::{self, DesktopSettings, DesktopSortMode};
use crate::framebuffer::WHITE;
use crate::settings_state::SystemSettings;
use crate::wm::window::{Window, TITLE_H};

pub const DISPLAY_SETTINGS_W: i32 = 520;
pub const DISPLAY_SETTINGS_H: i32 = 388;

const BG_A: u32 = theme::BG_TOP;
const BG_B: u32 = theme::BG_BOTTOM;
const PANEL: u32 = theme::CARD_SURFACE;
const PANEL_ALT: u32 = theme::CONTROL_FILL;
const BORDER: u32 = theme::BORDER;
const ACCENT: u32 = theme::ACCENT;
const ACCENT_DIM: u32 = theme::CONTROL_DISABLED;
const DIVIDER: u32 = theme::DIVIDER;
const CARD_HOVER: u32 = theme::CARD_HOVER;
const TEXT_ON_ACCENT: u32 = theme::TEXT_ON_ACCENT;
const LABEL: u32 = theme::TEXT;
const MUTED: u32 = theme::TEXT_MUTED;
const GOOD: u32 = theme::SUCCESS;
const TAB_X: usize = 14;
const TAB_Y: usize = 46;
const TAB_W: usize = 62;
const TAB_H: usize = 22;
const TAB_STEP: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsPage {
    Desktop,
    Accessibility,
    Diagnostics,
    Logs,
    Network,
    Storage,
    Accounts,
}

const SETTINGS_PAGES: [(SettingsPage, &str); 7] = [
    (SettingsPage::Desktop, "Desktop"),
    (SettingsPage::Accessibility, "Access"),
    (SettingsPage::Diagnostics, "Diag"),
    (SettingsPage::Logs, "Logs"),
    (SettingsPage::Network, "Net"),
    (SettingsPage::Storage, "Storage"),
    (SettingsPage::Accounts, "Users"),
];

pub struct DisplaySettingsApp {
    pub window: Window,
    last_width: i32,
    last_height: i32,
    last_settings: DesktopSettings,
    last_system_settings: SystemSettings,
    last_security_revision: u64,
    page: SettingsPage,
    last_page: SettingsPage,
    selected_user: usize,
    account_status: String,
}

impl DisplaySettingsApp {
    pub fn new(x: i32, y: i32) -> Self {
        Self::with_page(x, y, "desktop")
    }

    pub fn with_page(x: i32, y: i32, page_name: &str) -> Self {
        let page = page_from_name(page_name);
        let title = if page == SettingsPage::Accounts {
            "Accounts"
        } else {
            "Display Settings"
        };
        let mut app = DisplaySettingsApp {
            window: Window::new(x, y, DISPLAY_SETTINGS_W, DISPLAY_SETTINGS_H, title),
            last_width: DISPLAY_SETTINGS_W,
            last_height: DISPLAY_SETTINGS_H,
            last_settings: desktop_settings::snapshot(),
            last_system_settings: crate::settings_state::snapshot(),
            last_security_revision: crate::security::revision(),
            page,
            last_page: page,
            selected_user: 0,
            account_status: String::new(),
        };
        app.render();
        app
    }

    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        let settings = desktop_settings::snapshot();

        if let Some(page) = self.hit_page_tab(lx, ly) {
            self.page = page;
        } else if self.page == SettingsPage::Desktop && self.hit_toggle(lx, ly, 120) {
            desktop_settings::set_show_icons(!settings.show_icons);
        } else if self.page == SettingsPage::Desktop && self.hit_toggle(lx, ly, 152) {
            desktop_settings::set_compact_spacing(!settings.compact_spacing);
        } else if self.page == SettingsPage::Desktop && self.hit_toggle(lx, ly, 184) {
            let prefs = crate::app_lifecycle::start_menu_prefs();
            crate::app_lifecycle::set_start_menu_compact(!prefs.compact);
        } else if self.page == SettingsPage::Desktop {
            if let Some(mode) = self.hit_sort_button(lx, ly) {
                desktop_settings::set_sort_mode(mode);
            } else {
                return;
            }
        } else if self.page == SettingsPage::Accessibility && self.hit_toggle(lx, ly, 104) {
            let access = crate::accessibility::snapshot();
            crate::accessibility::set("keyboard_nav", !access.keyboard_nav);
        } else if self.page == SettingsPage::Accessibility && self.hit_toggle(lx, ly, 136) {
            let access = crate::accessibility::snapshot();
            crate::accessibility::set("focus_rings", !access.focus_rings);
        } else if self.page == SettingsPage::Accessibility && self.hit_toggle(lx, ly, 168) {
            let access = crate::accessibility::snapshot();
            crate::accessibility::set("large_text", !access.large_text);
        } else if self.page == SettingsPage::Accessibility && self.hit_toggle(lx, ly, 200) {
            let access = crate::accessibility::snapshot();
            crate::accessibility::set("reduced_motion", !access.reduced_motion);
        } else if self.page == SettingsPage::Diagnostics && self.hit_toggle(lx, ly, 104) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set(
                "diagnostics_task_snapshots",
                !prefs.diagnostics_task_snapshots,
            );
        } else if self.page == SettingsPage::Diagnostics && self.hit_toggle(lx, ly, 136) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set(
                "diagnostics_crash_details",
                !prefs.diagnostics_crash_details,
            );
        } else if self.page == SettingsPage::Logs && self.hit_toggle(lx, ly, 104) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set("logs_include_profiler", !prefs.logs_include_profiler);
        } else if self.page == SettingsPage::Logs && self.hit_toggle(lx, ly, 136) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set("logs_persist_kernel", !prefs.logs_persist_kernel);
        } else if self.page == SettingsPage::Network && self.hit_toggle(lx, ly, 104) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set("network_dns_enabled", !prefs.network_dns_enabled);
        } else if self.page == SettingsPage::Network && self.hit_toggle(lx, ly, 136) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set("network_http_enabled", !prefs.network_http_enabled);
        } else if self.page == SettingsPage::Network && self.hit_toggle(lx, ly, 168) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set("network_offline_api", !prefs.network_offline_api);
        } else if self.page == SettingsPage::Storage && self.hit_toggle(lx, ly, 104) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set(
                "storage_writeback_enabled",
                !prefs.storage_writeback_enabled,
            );
        } else if self.page == SettingsPage::Storage && self.hit_toggle(lx, ly, 136) {
            let prefs = crate::settings_state::snapshot();
            crate::settings_state::set("storage_fsck_on_boot", !prefs.storage_fsck_on_boot);
        } else if self.page == SettingsPage::Accounts {
            self.handle_accounts_click(lx, ly);
        } else {
            return;
        }

        crate::wm::request_repaint();
        self.render();
    }

    pub fn update(&mut self) {
        let settings = desktop_settings::snapshot();
        let system_settings = crate::settings_state::snapshot();
        let security_revision = crate::security::revision();
        if self.window.width != self.last_width
            || self.window.height != self.last_height
            || settings != self.last_settings
            || system_settings != self.last_system_settings
            || security_revision != self.last_security_revision
            || self.page != self.last_page
        {
            self.render();
        }
    }

    fn render(&mut self) {
        let settings = desktop_settings::snapshot();
        let access = crate::accessibility::snapshot();
        let system_settings = crate::settings_state::snapshot();
        self.last_width = self.window.width;
        self.last_height = self.window.height;
        self.last_settings = settings;
        self.last_system_settings = system_settings;
        self.last_security_revision = crate::security::revision();
        self.last_page = self.page;

        let stride = self.window.width.max(0) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        self.fill_background(stride);
        self.window.scroll.content_h = 0;
        self.window.scroll.offset = 0;

        self.fill_rect(stride, 0, 0, stride, 36, PANEL_ALT);
        self.fill_rect(stride, 0, 35, stride, 1, BORDER);
        self.put_str(stride, 18, 12, "SETTINGS", LABEL);
        self.put_str(
            stride,
            18,
            24,
            "desktop, access, diagnostics, logs, network, storage, accounts",
            MUTED,
        );
        self.draw_page_tabs(stride);

        match self.page {
            SettingsPage::Desktop => self.render_desktop_page(stride, content_h, settings),
            SettingsPage::Accessibility => self.render_accessibility_page(stride, access),
            SettingsPage::Diagnostics => self.render_diagnostics_page(stride, system_settings),
            SettingsPage::Logs => self.render_logs_page(stride, system_settings),
            SettingsPage::Network => self.render_network_page(stride, system_settings),
            SettingsPage::Storage => self.render_storage_page(stride, system_settings),
            SettingsPage::Accounts => self.render_accounts_page(stride),
        }
        self.window.mark_dirty_all();
    }

    fn render_desktop_page(&mut self, stride: usize, content_h: usize, settings: DesktopSettings) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 78, panel_w, 54);
        self.draw_panel(stride, 16, 140, panel_w, 84);
        self.draw_panel(stride, 16, 232, panel_w, content_h.saturating_sub(248));

        self.put_str(stride, 28, 92, "CURRENT MODE", LABEL);
        self.put_resolution_line(stride, 28, 108);
        self.put_str(stride, 250, 108, "Sort", MUTED);
        self.put_str(stride, 290, 108, settings.sort_mode.label(), GOOD);

        self.draw_toggle_row(
            stride,
            28,
            120,
            panel_w.saturating_sub(24),
            "Show desktop icons",
            settings.show_icons,
        );
        self.draw_toggle_row(
            stride,
            28,
            152,
            panel_w.saturating_sub(24),
            "Compact icon spacing",
            settings.compact_spacing,
        );
        self.draw_toggle_row(
            stride,
            28,
            184,
            panel_w.saturating_sub(24),
            "Compact Start menu layout",
            crate::app_lifecycle::start_menu_prefs().compact,
        );

        self.put_str(stride, 28, 244, "SORT ORDER", LABEL);
        self.put_str(stride, 28, 258, "controls desktop icon layout", MUTED);
        self.draw_sort_buttons(stride, 170, 240, settings.sort_mode);
    }

    fn render_accessibility_page(
        &mut self,
        stride: usize,
        access: crate::accessibility::AccessibilitySettings,
    ) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 82, panel_w, 156);
        self.put_str(stride, 28, 94, "ACCESSIBILITY", LABEL);
        self.draw_toggle_row(
            stride,
            28,
            104,
            panel_w.saturating_sub(24),
            "Keyboard-only navigation",
            access.keyboard_nav,
        );
        self.draw_toggle_row(
            stride,
            28,
            136,
            panel_w.saturating_sub(24),
            "Focus rings",
            access.focus_rings,
        );
        self.draw_toggle_row(
            stride,
            28,
            168,
            panel_w.saturating_sub(24),
            "Large text across shell/apps",
            access.large_text,
        );
        self.draw_toggle_row(
            stride,
            28,
            200,
            panel_w.saturating_sub(24),
            "Reduced motion / calmer UI",
            access.reduced_motion,
        );
    }

    fn render_diagnostics_page(&mut self, stride: usize, settings: SystemSettings) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 82, panel_w, 86);
        self.draw_panel(stride, 16, 178, panel_w, 82);
        self.draw_panel(stride, 16, 270, panel_w, 90);

        self.put_str(stride, 28, 96, "DIAGNOSTICS CONTROLS", LABEL);
        self.draw_toggle_row(
            stride,
            28,
            104,
            panel_w.saturating_sub(24),
            "Include persistent task snapshots",
            settings.diagnostics_task_snapshots,
        );
        self.draw_toggle_row(
            stride,
            28,
            136,
            panel_w.saturating_sub(24),
            "Show detailed crash register context",
            settings.diagnostics_crash_details,
        );

        let stats = crate::wm::compositor::compositor_stats();
        let service_count = crate::services::lines().len();
        let config_count = crate::config_store::lines().len().saturating_sub(1);
        let crash_count = crate::crashdump::lines()
            .iter()
            .filter(|line| !line.contains("no crash"))
            .count();
        self.put_str(stride, 28, 192, "HEALTH", LABEL);
        self.put_str(stride, 28, 208, "selftests active at boot", WHITE);
        let mut crash_line = String::from("crash reports ");
        push_number(&mut crash_line, crash_count);
        self.put_str(
            stride,
            28,
            224,
            &crash_line,
            if crash_count == 0 { GOOD } else { WHITE },
        );
        let mut service_line = String::from("services tracked ");
        push_number(&mut service_line, service_count);
        self.put_str(stride, 28, 240, &service_line, MUTED);

        let right_x = 236usize;
        self.put_str(stride, right_x, 192, "COMPOSITOR", LABEL);
        let mut fps_line = String::from("fps ");
        push_number(&mut fps_line, stats.fps as usize);
        fps_line.push_str("  frames ");
        push_number(&mut fps_line, stats.frames as usize);
        self.put_str(stride, right_x, 208, &fps_line, WHITE);
        let mut damage_line = String::from("damage rows ");
        push_number(&mut damage_line, stats.damage_rows as usize);
        self.put_str(stride, right_x, 224, &damage_line, MUTED);
        let mut pixels_line = String::from("pixels ");
        push_number(&mut pixels_line, stats.damage_pixels as usize);
        self.put_str(stride, right_x, 240, &pixels_line, MUTED);

        self.put_str(stride, 28, 284, "CONFIG + PACKAGES", LABEL);
        let mut config_line = String::from("config files ");
        push_number(&mut config_line, config_count);
        config_line.push_str("   manifests ");
        let manifests = crate::app_metadata::installed_app_manifests();
        push_number(&mut config_line, manifests.len());
        self.put_str(stride, 28, 300, &config_line, WHITE);
        let validation = if crate::app_metadata::validate_installed_manifests().is_ok() {
            "manifest validation ok"
        } else {
            "manifest validation failed"
        };
        self.put_str(stride, 28, 316, validation, GOOD);
        self.put_str(
            stride,
            28,
            332,
            "events/settings/logs feed Diagnostics app",
            MUTED,
        );
    }

    fn render_logs_page(&mut self, stride: usize, settings: SystemSettings) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 82, panel_w, 86);
        self.draw_panel(stride, 16, 178, panel_w, 180);
        self.put_str(stride, 28, 96, "LOG CONTROLS", LABEL);
        self.draw_toggle_row(
            stride,
            28,
            104,
            panel_w.saturating_sub(24),
            "Include profiler events in log summaries",
            settings.logs_include_profiler,
        );
        self.draw_toggle_row(
            stride,
            28,
            136,
            panel_w.saturating_sub(24),
            "Persist kernel log to /LOGS/KERNEL.TXT",
            settings.logs_persist_kernel,
        );
        self.put_str(stride, 28, 192, "RECENT LOGS", LABEL);
        let mut lines = crate::klog::lines();
        if settings.logs_include_profiler {
            lines.extend(crate::profiler::lines());
        }
        self.put_lines(stride, 28, 212, &lines, 10);
    }

    fn render_network_page(&mut self, stride: usize, settings: SystemSettings) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 82, panel_w, 118);
        self.draw_panel(stride, 16, 210, panel_w, 148);
        self.put_str(stride, 28, 96, "NETWORK CONTROLS", LABEL);
        self.draw_toggle_row(
            stride,
            28,
            104,
            panel_w.saturating_sub(24),
            "Enable DNS resolver syscall/API",
            settings.network_dns_enabled,
        );
        self.draw_toggle_row(
            stride,
            28,
            136,
            panel_w.saturating_sub(24),
            "Enable HTTP client syscall/API",
            settings.network_http_enabled,
        );
        self.draw_toggle_row(
            stride,
            28,
            168,
            panel_w.saturating_sub(24),
            "Allow offline synthetic API without NIC",
            settings.network_offline_api,
        );
        self.put_str(stride, 28, 224, "NETWORK STATUS", LABEL);
        let mut lines = crate::net::status_lines();
        lines.extend(crate::net::protocol_lines());
        self.put_lines(stride, 28, 244, &lines, 8);
    }

    fn render_storage_page(&mut self, stride: usize, settings: SystemSettings) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 82, panel_w, 86);
        self.draw_panel(stride, 16, 178, panel_w, 180);
        self.put_str(stride, 28, 96, "STORAGE CONTROLS", LABEL);
        self.draw_toggle_row(
            stride,
            28,
            104,
            panel_w.saturating_sub(24),
            "Enable deferred writeback queue",
            settings.storage_writeback_enabled,
        );
        self.draw_toggle_row(
            stride,
            28,
            136,
            panel_w.saturating_sub(24),
            "Run fsck repair pass during boot",
            settings.storage_fsck_on_boot,
        );
        self.put_str(stride, 28, 192, "FILESYSTEM STATUS", LABEL);
        let mut lines = crate::vfs::mount_lines();
        lines.extend(crate::writeback::lines());
        lines.extend(crate::fs_hardening::status_lines());
        self.put_lines(stride, 28, 212, &lines, 10);
    }

    fn handle_accounts_click(&mut self, lx: i32, ly: i32) {
        let users = crate::security::users();
        let row_x = 28i32;
        let row_w = 288i32;
        for (idx, _) in users.iter().enumerate().take(7) {
            let y = 116i32 + idx as i32 * 24;
            if lx >= row_x && lx < row_x + row_w && ly >= y && ly < y + 22 {
                self.selected_user = idx;
                self.account_status.clear();
                self.render();
                return;
            }
        }

        let selected = users.get(self.selected_user.min(users.len().saturating_sub(1)));
        if self.hit_button(lx, ly, 336, 104, 150, 22) {
            if crate::security::first_run_required() {
                self.set_account_result(
                    "setup",
                    crate::security::complete_first_run_admin("owner", "ownerpass31"),
                );
            } else {
                self.account_status = String::from("setup already complete");
            }
        } else if self.hit_button(lx, ly, 336, 136, 150, 22) {
            let name = crate::security::suggested_user_name("user");
            self.set_account_result(
                "add",
                crate::security::create_user(&name, "changeme31", "user"),
            );
            self.selected_user = crate::security::users()
                .iter()
                .position(|user| user.name.eq_ignore_ascii_case(&name))
                .unwrap_or(self.selected_user);
        } else if self.hit_button(lx, ly, 336, 168, 150, 22) {
            if let Some(user) = selected {
                let role = if user.role == "admin" {
                    "user"
                } else {
                    "admin"
                };
                self.set_account_result("role", crate::security::set_user_role(&user.name, role));
            }
        } else if self.hit_button(lx, ly, 336, 200, 150, 22) {
            if let Some(user) = selected {
                self.set_account_result(
                    "login",
                    crate::security::set_user_enabled(&user.name, !user.login_enabled),
                );
            }
        } else if self.hit_button(lx, ly, 336, 232, 150, 22) {
            if let Some(user) = selected {
                self.set_account_result(
                    "password",
                    crate::security::reset_user_password(&user.name, "changeme31"),
                );
            }
        } else if self.hit_button(lx, ly, 336, 264, 150, 22) {
            if let Some(user) = selected {
                self.set_account_result("delete", crate::security::delete_user(&user.name));
                self.selected_user = self.selected_user.saturating_sub(1);
            }
        } else {
            return;
        }
        crate::wm::request_repaint();
        self.render();
    }

    fn set_account_result(
        &mut self,
        action: &str,
        result: Result<crate::security::User, crate::security::AccountError>,
    ) {
        self.account_status.clear();
        self.account_status.push_str(action);
        self.account_status.push_str(": ");
        match result {
            Ok(user) => {
                self.account_status.push_str(&user.name);
                self.account_status.push_str(" ok");
            }
            Err(err) => self.account_status.push_str(err.as_str()),
        }
    }

    fn render_accounts_page(&mut self, stride: usize) {
        let panel_w = (self.window.width.max(0) as usize).saturating_sub(32);
        self.draw_panel(stride, 16, 82, 308, 206);
        self.draw_panel(stride, 328, 82, panel_w.saturating_sub(312), 206);
        self.draw_panel(stride, 16, 298, panel_w, 58);

        self.put_str(stride, 28, 96, "ACCOUNTS", LABEL);
        self.put_str(stride, 336, 96, "ACTIONS", LABEL);

        let users = crate::security::users();
        if self.selected_user >= users.len() {
            self.selected_user = users.len().saturating_sub(1);
        }
        for (idx, user) in users.iter().enumerate().take(7) {
            let y = 116usize + idx * 24;
            let selected = idx == self.selected_user;
            self.fill_rect(
                stride,
                28,
                y,
                288,
                22,
                if selected { CARD_HOVER } else { PANEL },
            );
            if selected {
                self.fill_rect(stride, 28, y, 3, 22, ACCENT);
            }
            let mut line = user.name.clone();
            line.push_str(" uid=");
            push_number(&mut line, user.uid as usize);
            line.push(' ');
            line.push_str(&user.role);
            line.push(' ');
            line.push_str(if user.login_enabled { "on" } else { "off" });
            self.put_str(
                stride,
                38,
                y + 7,
                &line,
                if selected { WHITE } else { MUTED },
            );
        }

        self.draw_button(stride, 336, 104, 150, "Complete setup");
        self.draw_button(stride, 336, 136, 150, "Add user");
        self.draw_button(stride, 336, 168, 150, "Toggle role");
        self.draw_button(stride, 336, 200, 150, "Enable/disable");
        self.draw_button(stride, 336, 232, 150, "Reset password");
        self.draw_button(stride, 336, 264, 150, "Delete user");

        let setup = if crate::security::first_run_required() {
            "first-run setup required"
        } else {
            "first-run setup complete"
        };
        self.put_str(stride, 28, 312, setup, LABEL);
        self.put_str(
            stride,
            28,
            328,
            "GUI defaults: owner/ownerpass31, userN/changeme31",
            MUTED,
        );
        if !self.account_status.is_empty() {
            let status = self.account_status.clone();
            self.put_str(stride, 28, 344, &status, GOOD);
        }
    }

    fn put_lines(
        &mut self,
        stride: usize,
        x: usize,
        mut y: usize,
        lines: &[String],
        max_lines: usize,
    ) {
        for line in lines.iter().take(max_lines) {
            self.put_str(stride, x, y, line, WHITE);
            y += 14;
        }
    }

    fn draw_page_tabs(&mut self, stride: usize) {
        for (idx, (page, label)) in SETTINGS_PAGES.iter().enumerate() {
            let x = TAB_X + idx * TAB_STEP;
            let active = *page == self.page;
            self.fill_rect(
                stride,
                x,
                TAB_Y,
                TAB_W,
                TAB_H,
                if active { ACCENT } else { PANEL },
            );
            self.draw_rect_border(
                stride,
                x,
                TAB_Y,
                TAB_W,
                TAB_H,
                if active { WHITE } else { BORDER },
            );
            self.put_str(
                stride,
                x + 4,
                TAB_Y + 7,
                label,
                if active { TEXT_ON_ACCENT } else { LABEL },
            );
        }
    }

    fn hit_page_tab(&self, lx: i32, ly: i32) -> Option<SettingsPage> {
        let tab_y = TAB_Y as i32;
        let tab_h = TAB_H as i32;
        if !(tab_y..tab_y + tab_h).contains(&ly) {
            return None;
        }
        for (idx, (page, _)) in SETTINGS_PAGES.iter().enumerate() {
            let x = TAB_X as i32 + idx as i32 * TAB_STEP as i32;
            if lx >= x && lx < x + TAB_W as i32 {
                return Some(*page);
            }
        }
        None
    }

    fn put_resolution_line(&mut self, stride: usize, x: usize, y: usize) {
        let mut line = String::from("Resolution ");
        push_number(&mut line, crate::framebuffer::width());
        line.push('x');
        push_number(&mut line, crate::framebuffer::height());
        self.put_str(stride, x, y, &line, WHITE);
    }

    fn draw_toggle_row(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        label: &str,
        active: bool,
    ) {
        self.fill_rect(stride, x, y, w, 22, PANEL);
        self.draw_rect_border(stride, x, y, w, 22, BORDER);
        self.put_str(stride, x + 12, y + 7, label, WHITE);
        let pill_x = x + w.saturating_sub(62);
        let pill_bg = if active { ACCENT } else { ACCENT_DIM };
        self.fill_rect(stride, pill_x, y + 4, 46, 14, pill_bg);
        self.draw_rect_border(stride, pill_x, y + 4, 46, 14, WHITE);
        self.put_str(
            stride,
            pill_x + 11,
            y + 7,
            if active { "ON" } else { "OFF" },
            if active { TEXT_ON_ACCENT } else { WHITE },
        );
    }

    fn draw_button(&mut self, stride: usize, x: usize, y: usize, w: usize, label: &str) {
        self.fill_rect(stride, x, y, w, 22, PANEL_ALT);
        self.draw_rect_border(stride, x, y, w, 22, BORDER);
        self.fill_rect(stride, x, y, w, 2, ACCENT);
        self.put_str(stride, x + 8, y + 7, label, WHITE);
    }

    fn hit_button(&self, lx: i32, ly: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
        lx >= x && lx < x + w && ly >= y && ly < y + h
    }

    fn draw_sort_buttons(&mut self, stride: usize, x: usize, y: usize, current: DesktopSortMode) {
        let button_w = 72usize;
        for (idx, mode) in [
            DesktopSortMode::Default,
            DesktopSortMode::Name,
            DesktopSortMode::Type,
        ]
        .iter()
        .enumerate()
        {
            let bx = x + idx * (button_w + 10);
            let active = *mode == current;
            self.fill_rect(
                stride,
                bx,
                y,
                button_w,
                20,
                if active { ACCENT } else { PANEL },
            );
            self.draw_rect_border(
                stride,
                bx,
                y,
                button_w,
                20,
                if active { WHITE } else { BORDER },
            );
            self.put_str(
                stride,
                bx + (button_w.saturating_sub(mode.label().len() * 8)) / 2,
                y + 6,
                mode.label(),
                if active { TEXT_ON_ACCENT } else { WHITE },
            );
        }
    }

    fn hit_toggle(&self, lx: i32, ly: i32, y: i32) -> bool {
        let panel_w = self.window.width.max(0) - 32;
        lx >= 28 && lx < 28 + panel_w - 24 && ly >= y && ly < y + 22
    }

    fn hit_sort_button(&self, lx: i32, ly: i32) -> Option<DesktopSortMode> {
        if ly < 240 || ly >= 260 {
            return None;
        }
        let button_w = 72i32;
        let start_x = 170i32;
        for (idx, mode) in [
            DesktopSortMode::Default,
            DesktopSortMode::Name,
            DesktopSortMode::Type,
        ]
        .iter()
        .enumerate()
        {
            let bx = start_x + idx as i32 * (button_w + 10);
            if lx >= bx && lx < bx + button_w {
                return Some(*mode);
            }
        }
        None
    }

    fn draw_panel(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(stride, x, y, w, h, PANEL);
        self.draw_rect_border(stride, x, y, w, h, BORDER);
        if h > 2 && w > 2 {
            self.draw_rect_border(stride, x + 1, y + 1, w - 2, h - 2, DIVIDER);
        }
    }

    fn fill_background(&mut self, stride: usize) {
        for (idx, pixel) in self.window.buf.iter_mut().enumerate() {
            let py = idx / stride;
            *pixel = if py % 10 < 5 { BG_A } else { BG_B };
        }
    }

    fn fill_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        let width = self.window.width.max(0) as usize;
        for row in y..(y + h).min(content_h) {
            let base = row * stride;
            for col in x..(x + w).min(width) {
                let idx = base + col;
                if idx < self.window.buf.len() {
                    self.window.buf[idx] = color;
                }
            }
        }
    }

    fn draw_rect_border(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: u32,
    ) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(stride, x, y, w, 1, color);
        self.fill_rect(stride, x, y + h - 1, w, 1, color);
        self.fill_rect(stride, x, y, 1, h, color);
        self.fill_rect(stride, x + w - 1, y, 1, h, color);
    }

    fn put_str(&mut self, stride: usize, x: usize, y: usize, s: &str, color: u32) {
        for (i, ch) in s.chars().enumerate() {
            let glyph = crate::font::glyph_rows(ch, crate::font::UI_FONT);
            for (gy, &byte) in glyph.iter().enumerate() {
                for gx in 0..8 {
                    if (byte >> gx) & 1 == 1 {
                        let px = x + i * 8 + gx;
                        let py = y + gy;
                        let idx = py * stride + px;
                        if idx < self.window.buf.len() {
                            self.window.buf[idx] = color;
                        }
                    }
                }
            }
        }
    }
}

fn page_from_name(name: &str) -> SettingsPage {
    match name.to_ascii_lowercase().as_str() {
        "access" | "accessibility" => SettingsPage::Accessibility,
        "diag" | "diagnostics" | "health" => SettingsPage::Diagnostics,
        "logs" | "log" | "profiler" => SettingsPage::Logs,
        "net" | "network" => SettingsPage::Network,
        "storage" | "disk" => SettingsPage::Storage,
        "users" | "accounts" | "account" => SettingsPage::Accounts,
        _ => SettingsPage::Desktop,
    }
}

fn push_number(out: &mut String, mut value: usize) {
    if value == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0u8; 20];
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
