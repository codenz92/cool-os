impl TerminalApp {
    pub fn print_char(&mut self, c: char) {
        self.refresh_layout();
        if self.ansi_state != AnsiState::Ground {
            self.feed_ansi(c);
            return;
        }
        if c == '\u{001B}' {
            self.ansi_state = AnsiState::Escape;
            return;
        }
        if c == '\r' {
            self.col = 0;
            return;
        }
        if c == '\n' {
            mirror_debug_char(c);
            self.col = 0;
            self.advance_row();
            return;
        }
        if c == '\u{0008}' {
            if self.col > 0 {
                self.col -= 1;
                self.draw_char_at(self.col, self.row, ' ');
            }
            return;
        }
        if c == '\t' {
            let spaces = 4 - (self.col % 4);
            for _ in 0..spaces {
                self.print_char(' ');
            }
            return;
        }
        if c.is_control() {
            return;
        }
        if self.col >= self.cols {
            self.col = 0;
            self.advance_row();
        }
        mirror_debug_char(c);
        self.draw_char_at(self.col, self.row, c);
        self.col += 1;
    }

    pub fn print_str(&mut self, s: &str) {
        for c in s.chars() {
            self.print_char(c);
        }
    }

    fn print_u64(&mut self, mut n: u64) {
        if n == 0 {
            self.print_char('0');
            return;
        }
        let mut buf = [0u8; 20];
        let mut i = 20usize;
        while n > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        for &b in &buf[i..] {
            self.print_char(b as char);
        }
    }

    fn print_size(&mut self, bytes: usize) {
        if bytes >= 1024 * 1024 {
            self.print_u64((bytes / (1024 * 1024)) as u64);
            self.print_str(" MB");
        } else if bytes >= 1024 {
            self.print_u64((bytes / 1024) as u64);
            self.print_str(" KB");
        } else {
            self.print_u64(bytes as u64);
            self.print_str(" B");
        }
    }

    fn advance_row(&mut self) {
        self.row += 1;
        if self.row >= self.rows {
            self.scroll_up();
            self.row = self.rows - 1;
        }
    }

    fn scroll_up(&mut self) {
        let stride = self.window.width as usize;
        let text_x = TERM_PAD_X;
        let text_y = TERM_PAD_Y;
        let text_w = self.cols * CHAR_W;
        let text_h = self.rows * LINE_H;

        if text_w == 0 || text_h <= LINE_H {
            return;
        }

        for y in 0..(text_h - LINE_H) {
            let dst_row = text_y + y;
            let src_row = dst_row + LINE_H;
            let dst = dst_row * stride + text_x;
            let src = src_row * stride + text_x;
            self.window.buf.copy_within(src..src + text_w, dst);
        }

        for y in (text_h - LINE_H)..text_h {
            let py = text_y + y;
            let row_start = py * stride + text_x;
            for x in 0..text_w {
                self.window.buf[row_start + x] = Self::bg_at(py);
            }
        }
        self.window
            .mark_dirty(text_x as i32, text_y as i32, text_w as i32, text_h as i32);
    }

    fn feed_ansi(&mut self, c: char) {
        match self.ansi_state {
            AnsiState::Ground => {}
            AnsiState::Escape => match c {
                '[' => {
                    self.reset_csi();
                    self.ansi_state = AnsiState::Csi;
                }
                'c' => {
                    self.reset_terminal();
                    self.ansi_state = AnsiState::Ground;
                }
                '7' => {
                    self.save_cursor();
                    self.ansi_state = AnsiState::Ground;
                }
                '8' => {
                    self.restore_cursor();
                    self.ansi_state = AnsiState::Ground;
                }
                '\u{001B}' => {
                    self.ansi_state = AnsiState::Escape;
                }
                _ => {
                    self.ansi_state = AnsiState::Ground;
                }
            },
            AnsiState::Csi => {
                if c == '?' && self.ansi_param_count == 0 && !self.ansi_param_active {
                    self.ansi_private = true;
                    return;
                }
                if c.is_ascii_digit() {
                    self.ansi_param_active = true;
                    self.ansi_param_value = self
                        .ansi_param_value
                        .saturating_mul(10)
                        .saturating_add(c as u16 - '0' as u16);
                    return;
                }
                if c == ';' {
                    self.push_csi_param();
                    return;
                }
                self.push_csi_param_if_active();
                self.execute_csi(c);
                self.ansi_state = AnsiState::Ground;
            }
        }
    }

    fn reset_csi(&mut self) {
        self.ansi_params = [0; 4];
        self.ansi_param_count = 0;
        self.ansi_param_value = 0;
        self.ansi_param_active = false;
        self.ansi_private = false;
    }

    fn push_csi_param(&mut self) {
        if self.ansi_param_count < self.ansi_params.len() {
            self.ansi_params[self.ansi_param_count] = if self.ansi_param_active {
                self.ansi_param_value
            } else {
                0
            };
            self.ansi_param_count += 1;
        }
        self.ansi_param_value = 0;
        self.ansi_param_active = false;
    }

    fn push_csi_param_if_active(&mut self) {
        if self.ansi_param_active {
            self.push_csi_param();
        }
    }

    fn csi_param(&self, index: usize, default: u16) -> u16 {
        if index < self.ansi_param_count {
            let value = self.ansi_params[index];
            if value == 0 {
                default
            } else {
                value
            }
        } else {
            default
        }
    }

    fn execute_csi(&mut self, command: char) {
        if self.ansi_private {
            return;
        }
        match command {
            'm' => self.apply_sgr(),
            'H' | 'f' => {
                let row = self.csi_param(0, 1) as usize;
                let col = self.csi_param(1, 1) as usize;
                self.set_cursor(row.saturating_sub(1), col.saturating_sub(1));
            }
            'A' => self.move_cursor_rows(-(self.csi_param(0, 1) as isize)),
            'B' => self.move_cursor_rows(self.csi_param(0, 1) as isize),
            'C' => self.move_cursor_cols(self.csi_param(0, 1) as isize),
            'D' => self.move_cursor_cols(-(self.csi_param(0, 1) as isize)),
            'G' => {
                let col = self.csi_param(0, 1) as usize;
                self.set_cursor(self.row, col.saturating_sub(1));
            }
            'J' => match self.csi_param(0, 0) {
                0 => self.clear_to_end_of_screen(),
                1 => self.clear_to_start_of_screen(),
                2 | 3 => self.reset_terminal(),
                _ => {}
            },
            'K' => match self.csi_param(0, 0) {
                0 => self.clear_line_range(self.row, self.col, self.cols),
                1 => self.clear_line_range(self.row, 0, self.col.saturating_add(1)),
                2 => self.clear_line_range(self.row, 0, self.cols),
                _ => {}
            },
            's' => self.save_cursor(),
            'u' => self.restore_cursor(),
            _ => {}
        }
    }

    fn apply_sgr(&mut self) {
        if self.ansi_param_count == 0 {
            self.set_fg(FG_OUTPUT);
            return;
        }
        for idx in 0..self.ansi_param_count {
            let code = self.ansi_params[idx];
            match code {
                0 => self.set_fg(FG_OUTPUT),
                1 => {}
                30 => self.set_fg(0x00_20_28_24),
                31 => self.set_fg(FG_ERROR),
                32 => self.set_fg(FG_PROMPT),
                33 => self.set_fg(FG_WARN),
                34 => self.set_fg(FG_DIR),
                35 => self.set_fg(0x00_DD_88_FF),
                36 => self.set_fg(FG_ACCENT),
                37 => self.set_fg(FG_INPUT),
                39 => self.set_fg(FG_OUTPUT),
                90 => self.set_fg(FG_DIM),
                91 => self.set_fg(0x00_FF_A0_A0),
                92 => self.set_fg(0x00_88_FF_BB),
                93 => self.set_fg(0x00_FF_E8_88),
                94 => self.set_fg(0x00_88_DD_FF),
                95 => self.set_fg(0x00_F0_A8_FF),
                96 => self.set_fg(0x00_A0_FF_FF),
                97 => self.set_fg(0x00_FF_FF_FF),
                _ => {}
            }
        }
    }

    fn set_cursor(&mut self, row: usize, col: usize) {
        self.row = row.min(self.rows.saturating_sub(1));
        self.col = col.min(self.cols.saturating_sub(1));
    }

    fn move_cursor_rows(&mut self, delta: isize) {
        let next = if delta.is_negative() {
            self.row.saturating_sub(delta.unsigned_abs())
        } else {
            self.row.saturating_add(delta as usize)
        };
        self.set_cursor(next, self.col);
    }

    fn move_cursor_cols(&mut self, delta: isize) {
        let next = if delta.is_negative() {
            self.col.saturating_sub(delta.unsigned_abs())
        } else {
            self.col.saturating_add(delta as usize)
        };
        self.set_cursor(self.row, next);
    }

    fn save_cursor(&mut self) {
        self.saved_col = self.col;
        self.saved_row = self.row;
    }

    fn restore_cursor(&mut self) {
        self.set_cursor(self.saved_row, self.saved_col);
    }

    fn reset_terminal(&mut self) {
        self.fill_background();
        self.col = 0;
        self.row = 0;
        self.set_fg(FG_OUTPUT);
    }

    fn clear_to_end_of_screen(&mut self) {
        self.clear_line_range(self.row, self.col, self.cols);
        for row in self.row.saturating_add(1)..self.rows {
            self.clear_line_range(row, 0, self.cols);
        }
    }

    fn clear_to_start_of_screen(&mut self) {
        for row in 0..self.row {
            self.clear_line_range(row, 0, self.cols);
        }
        self.clear_line_range(self.row, 0, self.col.saturating_add(1));
    }

    fn clear_line_range(&mut self, row: usize, start_col: usize, end_col: usize) {
        if row >= self.rows || start_col >= end_col {
            return;
        }
        let end_col = end_col.min(self.cols);
        for col in start_col.min(self.cols)..end_col {
            self.draw_char_at(col, row, ' ');
        }
    }

    fn draw_char_at(&mut self, col: usize, row: usize, c: char) {
        let glyph = crate::font::glyph_rows(c, crate::font::UI_FONT);
        let px0 = TERM_PAD_X + col * CHAR_W;
        let py0 = TERM_PAD_Y + row * LINE_H + GLYPH_Y_INSET;
        let stride = self.window.width as usize;
        let large_text = crate::accessibility::snapshot().large_text;
        for (gy, &byte) in glyph.iter().take(CHAR_H).enumerate() {
            for bit in 0..8usize {
                let px = px0 + bit;
                let py = py0 + gy;
                let idx = py * stride + px;
                if idx < self.window.buf.len() {
                    let ink = byte & (1 << bit) != 0;
                    self.window.buf[idx] = if ink { self.fg } else { Self::bg_at(py) };
                    if large_text && ink && idx + 1 < self.window.buf.len() {
                        self.window.buf[idx + 1] = self.fg;
                    }
                }
            }
        }
        self.window
            .mark_dirty(px0 as i32, py0 as i32, CHAR_W as i32 + 1, CHAR_H as i32);
    }

    fn set_fg(&mut self, color: u32) {
        self.fg = color;
    }

    fn print_prompt(&mut self) {
        self.set_fg(FG_PROMPT);
        self.print_str("cool");
        self.set_fg(FG_ACCENT);
        self.print_str("> ");
        self.set_fg(FG_INPUT);
        self.input_start_col = self.col;
    }

    fn fill_background(&mut self) {
        let stride = self.window.width as usize;
        for (idx, pixel) in self.window.buf.iter_mut().enumerate() {
            let py = idx / stride;
            *pixel = Self::bg_at(py);
        }
        self.window.mark_dirty_all();
    }

    fn paint_exposed_background(&mut self, old_width: usize, old_content_h: usize) {
        let new_width = self.window.width.max(0) as usize;
        let new_content_h = (self.window.height - TITLE_H).max(0) as usize;
        let shared_h = old_content_h.min(new_content_h);

        if new_width > old_width {
            let fill_w = new_width - old_width;
            for py in 0..shared_h {
                let row_start = py * new_width + old_width;
                let row_end = row_start + fill_w;
                for idx in row_start..row_end {
                    self.window.buf[idx] = Self::bg_at(py);
                }
            }
        }

        if new_content_h > old_content_h {
            for py in old_content_h..new_content_h {
                let row_start = py * new_width;
                for idx in row_start..row_start + new_width {
                    self.window.buf[idx] = Self::bg_at(py);
                }
            }
        }
        self.window.mark_dirty_all();
    }

    fn refresh_layout(&mut self) {
        let cols = text_cols(self.window.width as usize);
        let rows = text_rows((self.window.height - TITLE_H).max(0) as usize);
        if cols != self.cols || rows != self.rows {
            let _ = crate::tty::set_size(self.tty_id, cols as u16, rows as u16);
        }
        self.cols = cols;
        self.rows = rows;
        self.col = self.col.min(self.cols.saturating_sub(1));
        self.row = self.row.min(self.rows.saturating_sub(1));
        self.input_start_col = self.input_start_col.min(self.cols.saturating_sub(1));
    }

    fn bg_at(py: usize) -> u32 {
        match py % 6 {
            0 => TERM_BG_C,
            1 | 2 => TERM_BG_A,
            _ => TERM_BG_B,
        }
    }
}
