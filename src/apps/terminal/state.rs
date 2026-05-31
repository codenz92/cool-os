impl TerminalApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, TERM_W, TERM_H, "Terminal");
        let cols = text_cols(TERM_W as usize);
        let rows = text_rows((TERM_H - TITLE_H) as usize);
        let tty_id = crate::tty::create();
        let _ = crate::tty::set_size(tty_id, cols as u16, rows as u16);

        let mut t = TerminalApp {
            window,
            tty_id,
            cmd_buf: String::new(),
            pending_key_sink_fd: None,
            pending_browser_url: None,
            foreground_job: None,
            col: 0,
            row: 0,
            cols,
            rows,
            screen: blank_screen(rows, cols),
            scrollback: Vec::new(),
            scroll_top: 0,
            fg: FG_OUTPUT,
            cwd: String::from("/"),
            cmd_history: Vec::new(),
            history_pos: 0,
            saved_input: String::new(),
            input_start_col: 0,
            last_width: TERM_W,
            last_height: TERM_H,
            ansi_state: AnsiState::Ground,
            ansi_params: [0; 4],
            ansi_param_count: 0,
            ansi_param_value: 0,
            ansi_param_active: false,
            ansi_private: false,
            saved_col: 0,
            saved_row: 0,
        };
        t.fill_background();
        t.refresh_layout();
        t.set_fg(FG_ACCENT);
        t.print_str(" ____            _  ___  ____\n");
        t.print_str("/ ___|___   ___ | |/ _ \\/ ___|\n");
        t.print_str("| |   / _ \\ / _ \\| | | | \\___ \\\n");
        t.print_str("| |__| (_) | (_) | | |_| |___) |\n");
        t.print_str("\\____\\___/ \\___/|_|\\___/|____/\n");
        t.set_fg(FG_DIM);
        t.print_str("      *  modern desktop shell\n");
        t.print_str("      type help for commands\n\n");
        t.print_prompt();
        t
    }

    pub fn update(&mut self) {
        self.drain_tty_output();
        self.poll_foreground_job();
        let size_changed = self.window.width != self.last_width || self.window.height != self.last_height;
        self.refresh_layout();
        if size_changed && (self.window.width != self.last_width || self.window.height != self.last_height) {
            self.last_width = self.window.width;
            self.last_height = self.window.height;
            self.render_visible();
        }
        self.sync_scrollbar_drag();
    }

    pub fn is_busy(&self) -> bool {
        self.foreground_job.is_some()
    }

    pub fn handle_key(&mut self, c: char) {
        if self.foreground_job.is_some() {
            return;
        }
        self.refresh_layout();
        self.scroll_to_bottom();
        match c {
            // Arrow keys (private-use Unicode set by keyboard drivers)
            '\u{F700}' => self.history_up(),
            '\u{F701}' => self.history_down(),
            '\u{F702}' | '\u{F703}' => {} // left/right: ignore (no cursor movement)

            '\n' => {
                self.print_char('\n');
                let cmd = core::mem::take(&mut self.cmd_buf);
                self.push_history(&cmd);
                crate::app_lifecycle::record_command(&cmd);
                self.run_command(&cmd);
            }
            '\u{0008}' => {
                if self.cmd_buf.pop().is_some() && self.col > self.input_start_col {
                    self.col -= 1;
                    self.draw_char_at(self.col, self.row, ' ');
                }
            }
            c if !c.is_control() => {
                let max = self.cols.saturating_sub(self.input_start_col + 1);
                if self.cmd_buf.len() < max {
                    self.cmd_buf.push(c);
                    self.print_char(c);
                }
            }
            _ => {}
        }
    }

    pub fn handle_key_input(&mut self, input: KeyInput) {
        let foreground_signals =
            self.foreground_job.is_some() && crate::tty::signals_enabled(self.tty_id);
        if foreground_signals && input.has_ctrl() && !input.has_alt() {
            match input.key {
                Key::Character('c') | Key::Character('C') => {
                    if self.signal_foreground(crate::process_model::Signal::Int, "^C\n") {
                        return;
                    }
                    return;
                }
                Key::Character('z') | Key::Character('Z') => {
                    if self.signal_foreground(crate::process_model::Signal::Stop, "^Z\n") {
                        return;
                    }
                    return;
                }
                _ => {}
            }
        }
        if self.foreground_job.is_some() {
            self.forward_foreground_input(input);
            return;
        }
        match input.key {
            Key::PageUp => {
                self.scroll_page(-1);
                return;
            }
            Key::PageDown => {
                self.scroll_page(1);
                return;
            }
            _ => {}
        }
        if let Some(c) = input.legacy_char() {
            self.handle_key(c);
        }
    }

    fn forward_foreground_input(&mut self, input: KeyInput) {
        let mode = crate::tty::input_mode(self.tty_id).unwrap_or(crate::tty::TTY_MODE_DEFAULT);
        let raw = mode & crate::tty::TTY_MODE_CANONICAL == 0;
        let signals = mode & crate::tty::TTY_MODE_SIGNALS != 0;
        if signals && input.has_ctrl() && !input.has_alt() {
            match input.key {
                Key::Character('d') | Key::Character('D') => {
                    let _ = crate::tty::submit_eof(self.tty_id);
                }
                _ => {}
            }
            return;
        }
        if input.has_alt() {
            return;
        }
        if raw {
            self.forward_raw_input(input);
            return;
        }
        match input.key {
            Key::Character(c) => {
                let _ = crate::tty::submit_char(self.tty_id, c);
            }
            Key::Space => {
                let _ = crate::tty::submit_char(self.tty_id, ' ');
            }
            Key::Tab => {
                let _ = crate::tty::submit_char(self.tty_id, '\t');
            }
            Key::Enter => {
                let _ = crate::tty::submit_enter(self.tty_id);
            }
            Key::Backspace | Key::Delete => {
                let _ = crate::tty::submit_backspace(self.tty_id);
            }
            Key::Escape
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::F2
            | Key::F4
            | Key::F5 => {}
        }
    }

    fn forward_raw_input(&mut self, input: KeyInput) {
        if input.has_ctrl() && !input.has_alt() {
            if let Some(byte) = ctrl_byte(input.key) {
                let _ = crate::tty::submit_bytes(self.tty_id, &[byte]);
            }
            return;
        }
        match input.key {
            Key::Character(c) => {
                let _ = crate::tty::submit_char(self.tty_id, c);
            }
            Key::Space => {
                let _ = crate::tty::submit_char(self.tty_id, ' ');
            }
            Key::Tab => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\t");
            }
            Key::Enter => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\n");
            }
            Key::Backspace => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x08");
            }
            Key::Delete => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x7f");
            }
            Key::Escape => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b");
            }
            Key::ArrowUp => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[A");
            }
            Key::ArrowDown => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[B");
            }
            Key::ArrowRight => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[C");
            }
            Key::ArrowLeft => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[D");
            }
            Key::Home => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[H");
            }
            Key::End => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[F");
            }
            Key::PageUp => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[5~");
            }
            Key::PageDown => {
                let _ = crate::tty::submit_bytes(self.tty_id, b"\x1b[6~");
            }
            Key::F2 | Key::F4 | Key::F5 => {}
        }
    }

    pub fn execute_command(&mut self, command: &str) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        if self.foreground_job.is_some() {
            crate::wm::queue_startup_command(command);
            return;
        }
        self.refresh_layout();
        self.scroll_to_bottom();
        for c in command.chars() {
            if !c.is_control() {
                self.print_char(c);
            }
        }
        self.print_char('\n');
        self.push_history(command);
        crate::app_lifecycle::record_command(command);
        self.run_command(command);
    }

    pub fn take_pending_key_sink(&mut self) -> Option<usize> {
        self.pending_key_sink_fd.take()
    }

    pub fn take_browser_request(&mut self) -> Option<String> {
        self.pending_browser_url.take()
    }

    pub fn drain_tty_output(&mut self) {
        while let Some(byte) = crate::tty::pop_output_byte(self.tty_id) {
            self.print_char(byte as char);
        }
    }

    pub fn poll_foreground_job(&mut self) {
        let Some(job) = self.foreground_job.as_ref() else {
            return;
        };
        match crate::scheduler::process_group_status(job.group) {
            crate::scheduler::ProcessGroupStatus::Running => {}
            crate::scheduler::ProcessGroupStatus::Stopped => {
                let job = self.foreground_job.take().unwrap();
                crate::tty::set_foreground_group(self.tty_id, None);
                crate::tty::reset_input_mode(self.tty_id);
                self.set_fg(FG_WARN);
                self.print_str("\n[fg stopped] ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&job.title);
                self.print_str(" pgid=");
                self.print_u64(job.group as u64);
                if let Some(job_id) = job.job_id {
                    self.print_str(" job #");
                    self.print_u64(job_id);
                }
                self.print_char('\n');
                self.print_prompt();
            }
            crate::scheduler::ProcessGroupStatus::Exited
            | crate::scheduler::ProcessGroupStatus::Empty => {
                let job = self.foreground_job.take().unwrap();
                crate::tty::set_foreground_group(self.tty_id, None);
                crate::tty::reset_input_mode(self.tty_id);
                self.set_fg(FG_ACCENT);
                self.print_str("\n[fg done] ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&job.title);
                self.print_str(" pid=");
                self.print_u64(job.pid as u64);
                if let Some(code) = crate::scheduler::process_group_exit_code(job.group) {
                    self.print_str(" exit=");
                    self.print_u64(code);
                }
                if let Some(job_id) = job.job_id {
                    self.print_str(" job #");
                    self.print_u64(job_id);
                }
                self.print_char('\n');
                self.print_prompt();
            }
        }
    }

    pub fn handle_scroll(&mut self, delta: i32) {
        self.scroll_lines(delta.signum() * 3);
    }

    fn signal_foreground(&mut self, signal: crate::process_model::Signal, marker: &str) -> bool {
        let Some(group) = crate::tty::foreground_group(self.tty_id) else {
            return false;
        };
        self.set_fg(FG_DIM);
        self.print_str(marker);
        match crate::scheduler::send_signal_to_group(group, signal) {
            Ok(_) => true,
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("signal: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
                true
            }
        }
    }

    // ── History ───────────────────────────────────────────────────────────────

    fn push_history(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return;
        }
        if self.cmd_history.last().map(|s| s.as_str()) == Some(cmd) {
            return;
        }
        if self.cmd_history.len() >= HISTORY_MAX {
            self.cmd_history.remove(0);
        }
        self.cmd_history.push(String::from(cmd));
        self.history_pos = 0;
        self.saved_input.clear();
    }

    fn history_up(&mut self) {
        if self.cmd_history.is_empty() {
            return;
        }
        let new_pos = (self.history_pos + 1).min(self.cmd_history.len());
        if new_pos == self.history_pos {
            return;
        }
        if self.history_pos == 0 {
            self.saved_input = self.cmd_buf.clone();
        }
        self.history_pos = new_pos;
        let entry = self.cmd_history[self.cmd_history.len() - self.history_pos].clone();
        self.erase_input();
        self.cmd_buf = entry.clone();
        self.set_fg(FG_INPUT);
        self.print_str(&entry);
    }

    fn history_down(&mut self) {
        if self.history_pos == 0 {
            return;
        }
        self.history_pos -= 1;
        self.erase_input();
        if self.history_pos == 0 {
            let saved = self.saved_input.clone();
            self.cmd_buf = saved.clone();
            self.set_fg(FG_INPUT);
            self.print_str(&saved);
        } else {
            let entry = self.cmd_history[self.cmd_history.len() - self.history_pos].clone();
            self.cmd_buf = entry.clone();
            self.set_fg(FG_INPUT);
            self.print_str(&entry);
        }
    }

    fn erase_input(&mut self) {
        while self.col > self.input_start_col {
            self.col -= 1;
            self.draw_char_at(self.col, self.row, ' ');
        }
        self.cmd_buf.clear();
    }

    // ── Command dispatch ──────────────────────────────────────────────────────
}
