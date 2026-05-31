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
        self.sync_scroll_state();
    }

    fn scroll_up(&mut self) {
        self.ensure_screen_shape();
        if self.screen.is_empty() {
            return;
        }

        let follow = self.is_at_bottom();
        let line = self.screen.remove(0);
        self.push_scrollback_line(line, follow);
        self.screen.push(blank_line(self.cols));
        if follow {
            self.scroll_top = self.bottom_scroll_top();
            self.render_visible();
        } else {
            self.sync_scroll_state();
            self.window.mark_dirty_all();
        }
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
        self.scrollback.clear();
        self.screen = blank_screen(self.rows, self.cols);
        self.scroll_top = 0;
        self.render_visible();
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
        if row >= self.rows || col >= self.cols {
            return;
        }
        self.ensure_screen_shape();
        if let Some(line) = self.screen.get_mut(row) {
            if let Some(cell) = line.get_mut(col) {
                *cell = TerminalCell { ch: c, fg: self.fg };
            }
        }
        let logical_row = self.scrollback.len().saturating_add(row);
        if let Some(screen_row) = self.visible_screen_row(logical_row) {
            let cell = self.screen[row][col];
            self.paint_cell(col, screen_row, cell);
        }
    }

    fn set_fg(&mut self, color: u32) {
        self.fg = color;
    }

    fn print_prompt(&mut self) {
        self.scroll_to_bottom();
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

    fn refresh_layout(&mut self) {
        let cols = text_cols(self.window.width as usize);
        let rows = text_rows((self.window.height - TITLE_H).max(0) as usize);
        let changed = cols != self.cols || rows != self.rows;
        if changed {
            let _ = crate::tty::set_size(self.tty_id, cols as u16, rows as u16);
            let follow = self.is_at_bottom();
            self.resize_terminal_buffers(rows, cols, follow);
            self.last_width = self.window.width;
            self.last_height = self.window.height;
            self.render_visible();
        }
        self.col = self.col.min(self.cols.saturating_sub(1));
        self.row = self.row.min(self.rows.saturating_sub(1));
        self.input_start_col = self.input_start_col.min(self.cols.saturating_sub(1));
        self.sync_scroll_state();
    }

    fn resize_terminal_buffers(&mut self, rows: usize, cols: usize, follow: bool) {
        if cols != self.cols {
            for line in self.scrollback.iter_mut() {
                resize_line(line, cols);
            }
            for line in self.screen.iter_mut() {
                resize_line(line, cols);
            }
        }

        if rows < self.rows {
            let removed = self.rows - rows;
            for _ in 0..removed {
                if self.screen.is_empty() {
                    break;
                }
                let line = self.screen.remove(0);
                self.push_scrollback_line(line, follow);
            }
            self.row = self.row.saturating_sub(removed);
        } else if rows > self.rows {
            for _ in self.rows..rows {
                self.screen.push(blank_line(cols));
            }
        }

        self.cols = cols;
        self.rows = rows;
        self.ensure_screen_shape();
        if follow {
            self.scroll_top = self.bottom_scroll_top();
        } else {
            self.clamp_scroll_top();
        }
    }

    fn ensure_screen_shape(&mut self) {
        while self.screen.len() < self.rows {
            self.screen.push(blank_line(self.cols));
        }
        if self.screen.len() > self.rows {
            self.screen.truncate(self.rows);
        }
        for line in self.screen.iter_mut() {
            resize_line(line, self.cols);
        }
    }

    fn push_scrollback_line(&mut self, mut line: Vec<TerminalCell>, follow: bool) {
        resize_line(&mut line, self.cols);
        if self.scrollback.len() >= SCROLLBACK_MAX_LINES {
            self.scrollback.remove(0);
            if !follow {
                self.scroll_top = self.scroll_top.saturating_sub(1);
            }
        }
        self.scrollback.push(line);
    }

    fn total_lines(&self) -> usize {
        self.scrollback.len().saturating_add(self.rows)
    }

    fn bottom_scroll_top(&self) -> usize {
        self.scrollback.len()
    }

    fn is_at_bottom(&self) -> bool {
        self.scroll_top >= self.bottom_scroll_top()
    }

    fn clamp_scroll_top(&mut self) {
        self.scroll_top = self.scroll_top.min(self.bottom_scroll_top());
    }

    fn sync_scroll_state(&mut self) {
        let view_h = (self.rows * LINE_H) as i32;
        self.window.scroll.content_h = (self.total_lines() * LINE_H) as i32;
        self.window.scroll.offset = (self.scroll_top * LINE_H) as i32;
        self.window.scroll.clamp(view_h);
        self.scroll_top = ((self.window.scroll.offset / LINE_H as i32).max(0) as usize)
            .min(self.bottom_scroll_top());
    }

    fn sync_scrollbar_drag(&mut self) {
        let expected = (self.scroll_top * LINE_H) as i32;
        if self.window.scroll.offset != expected {
            let next = ((self.window.scroll.offset / LINE_H as i32).max(0) as usize)
                .min(self.bottom_scroll_top());
            if next != self.scroll_top {
                self.scroll_top = next;
                self.render_visible();
                return;
            }
        }
        self.sync_scroll_state();
    }

    fn scroll_lines(&mut self, delta: i32) {
        self.refresh_layout();
        let max = self.bottom_scroll_top() as i32;
        let next = (self.scroll_top as i32 + delta).clamp(0, max) as usize;
        if next != self.scroll_top {
            self.scroll_top = next;
            self.render_visible();
        }
    }

    fn scroll_page(&mut self, direction: i32) {
        let page = self.rows.saturating_sub(1).max(1) as i32;
        self.scroll_lines(page.saturating_mul(direction.signum()));
    }

    fn scroll_to_bottom(&mut self) {
        let bottom = self.bottom_scroll_top();
        if self.scroll_top != bottom {
            self.scroll_top = bottom;
            self.render_visible();
        } else {
            self.sync_scroll_state();
        }
    }

    fn visible_screen_row(&self, logical_row: usize) -> Option<usize> {
        if logical_row < self.scroll_top {
            return None;
        }
        let screen_row = logical_row - self.scroll_top;
        (screen_row < self.rows).then_some(screen_row)
    }

    fn display_cell(&self, logical_row: usize, col: usize) -> TerminalCell {
        if logical_row < self.scrollback.len() {
            self.scrollback
                .get(logical_row)
                .and_then(|line| line.get(col))
                .copied()
                .unwrap_or_else(blank_cell)
        } else {
            let row = logical_row - self.scrollback.len();
            self.screen
                .get(row)
                .and_then(|line| line.get(col))
                .copied()
                .unwrap_or_else(blank_cell)
        }
    }

    fn render_visible(&mut self) {
        self.fill_background();
        for screen_row in 0..self.rows {
            let logical_row = self.scroll_top + screen_row;
            for col in 0..self.cols {
                let cell = self.display_cell(logical_row, col);
                if cell.ch != ' ' {
                    self.paint_cell(col, screen_row, cell);
                }
            }
        }
        self.sync_scroll_state();
        self.window.mark_dirty_all();
    }

    fn paint_cell(&mut self, col: usize, screen_row: usize, cell: TerminalCell) {
        let px0 = TERM_PAD_X + col * CHAR_W;
        let py0 = TERM_PAD_Y + screen_row * LINE_H + GLYPH_Y_INSET;
        let stride = self.window.width.max(0) as usize;
        if stride == 0 {
            return;
        }

        for gy in 0..CHAR_H {
            let py = py0 + gy;
            let row_start = py.saturating_mul(stride);
            for dx in 0..=CHAR_W {
                let idx = row_start + px0 + dx;
                if idx < self.window.buf.len() {
                    self.window.buf[idx] = Self::bg_at(py);
                }
            }
        }

        let glyph = crate::font::glyph_rows(cell.ch, crate::font::UI_FONT);
        let large_text = crate::accessibility::snapshot().large_text;
        for (gy, &byte) in glyph.iter().take(CHAR_H).enumerate() {
            for bit in 0..8usize {
                if byte & (1 << bit) == 0 {
                    continue;
                }
                let px = px0 + bit;
                let py = py0 + gy;
                let idx = py * stride + px;
                if idx < self.window.buf.len() {
                    self.window.buf[idx] = cell.fg;
                    if large_text && idx + 1 < self.window.buf.len() {
                        self.window.buf[idx + 1] = cell.fg;
                    }
                }
            }
        }
        self.window
            .mark_dirty(px0 as i32, py0 as i32, CHAR_W as i32 + 1, CHAR_H as i32);
    }

    fn bg_at(py: usize) -> u32 {
        const TOP_SPAN: usize = 180;
        const BOTTOM_SPAN: usize = 300;

        if py < TOP_SPAN {
            mix_rgb(TERM_BG_TOP, TERM_BG_MID, py, TOP_SPAN)
        } else {
            mix_rgb(
                TERM_BG_MID,
                TERM_BG_BOTTOM,
                py.saturating_sub(TOP_SPAN),
                BOTTOM_SPAN,
            )
        }
    }
}

fn mix_rgb(a: u32, b: u32, pos: usize, span: usize) -> u32 {
    if span == 0 {
        return b;
    }
    let pos = pos.min(span) as u32;
    let span = span as u32;
    let ar = (a >> 16) & 0xFF;
    let ag = (a >> 8) & 0xFF;
    let ab = a & 0xFF;
    let br = (b >> 16) & 0xFF;
    let bg = (b >> 8) & 0xFF;
    let bb = b & 0xFF;
    let r = (ar * (span - pos) + br * pos) / span;
    let g = (ag * (span - pos) + bg * pos) / span;
    let blue = (ab * (span - pos) + bb * pos) / span;
    (r << 16) | (g << 8) | blue
}

fn blank_cell() -> TerminalCell {
    TerminalCell {
        ch: ' ',
        fg: FG_OUTPUT,
    }
}

fn blank_line(cols: usize) -> Vec<TerminalCell> {
    alloc::vec![blank_cell(); cols]
}

fn blank_screen(rows: usize, cols: usize) -> Vec<Vec<TerminalCell>> {
    let mut screen = Vec::new();
    for _ in 0..rows {
        screen.push(blank_line(cols));
    }
    screen
}

fn resize_line(line: &mut Vec<TerminalCell>, cols: usize) {
    line.resize(cols, blank_cell());
}
