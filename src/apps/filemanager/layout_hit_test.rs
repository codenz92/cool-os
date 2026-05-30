use super::*;

impl FileManagerApp {
    pub(super) fn active_tab_rect(&self, layout: Layout) -> Rect {
        Rect {
            x: 10,
            y: 4,
            w: (layout.width / 3).clamp(180, 260),
            h: TABBAR_H - 4,
        }
    }

    pub(super) fn command_button_rect(&self, item: CommandBarItem) -> Rect {
        let mut x = 12;
        for cmd in command_bar_items() {
            let w = cmd.width();
            if cmd == item {
                return Rect {
                    x,
                    y: TABBAR_H + 4,
                    w,
                    h: COMMAND_BAR_H - 8,
                };
            }
            x += w + COMMAND_GAP;
        }
        Rect {
            x: 12,
            y: TABBAR_H + 4,
            w: item.width(),
            h: COMMAND_BAR_H - 8,
        }
    }

    pub(super) fn hit_command_button(&self, lx: i32, ly: i32) -> Option<CommandBarItem> {
        if !(TABBAR_H..TABBAR_H + COMMAND_BAR_H).contains(&ly) {
            return None;
        }
        command_bar_items()
            .into_iter()
            .find(|&cmd| self.command_button_rect(cmd).hit(lx, ly))
    }

    pub(super) fn command_menu_rect(&self, kind: CommandMenuKind) -> Rect {
        let anchor = match kind {
            CommandMenuKind::New => self.command_button_rect(CommandBarItem::New),
            CommandMenuKind::Sort => self.command_button_rect(CommandBarItem::Sort),
            CommandMenuKind::More => self.command_button_rect(CommandBarItem::More),
        };
        let rows = self.command_menu_items(kind).len() as i32;
        Rect {
            x: anchor.x,
            y: anchor.y + anchor.h + 4,
            w: match kind {
                CommandMenuKind::More => 168,
                _ => 132,
            },
            h: rows * MENU_ROW_H + 6,
        }
    }

    pub(super) fn command_menu_items(
        &self,
        kind: CommandMenuKind,
    ) -> Vec<(&'static str, ContextAction)> {
        match kind {
            CommandMenuKind::New => alloc::vec![
                ("Folder", ContextAction::NewFolder),
                ("Text File", ContextAction::NewFile),
            ],
            CommandMenuKind::Sort => alloc::vec![
                ("Name", ContextAction::Refresh),
                ("Type", ContextAction::Refresh),
                ("Size", ContextAction::Refresh),
            ],
            CommandMenuKind::More => self.context_menu_items(self.focused),
        }
    }

    pub(super) fn hit_command_menu_row(&self, lx: i32, ly: i32) -> Option<usize> {
        let kind = self.command_menu?;
        let rect = self.command_menu_rect(kind);
        if !rect.hit(lx, ly) {
            return None;
        }
        let rel_y = ly - rect.y - 3;
        if rel_y < 0 {
            return None;
        }
        let row = (rel_y / MENU_ROW_H) as usize;
        (row < self.command_menu_items(kind).len()).then_some(row)
    }

    pub(super) fn context_menu_rect(&self, menu: &ContextMenuState) -> Rect {
        let items = self.context_menu_items(menu.target);
        Rect {
            x: menu.x,
            y: menu.y,
            w: MENU_W,
            h: items.len() as i32 * MENU_ROW_H + 6,
        }
    }

    pub(super) fn clamp_context_menu(
        &self,
        lx: i32,
        ly: i32,
        target: Option<usize>,
    ) -> ContextMenuState {
        let layout = self.layout();
        let temp = ContextMenuState {
            x: lx,
            y: ly,
            target,
        };
        let rect = self.context_menu_rect(&temp);
        ContextMenuState {
            x: lx.clamp(
                layout.main_x + 4,
                (layout.width - rect.w - 4).max(layout.main_x + 4),
            ),
            y: ly.clamp(
                COMMAND_H + PATHBAR_H + 4,
                (layout.status_y - rect.h - 4).max(COMMAND_H + PATHBAR_H + 4),
            ),
            target,
        }
    }

    pub(super) fn layout(&self) -> Layout {
        let width = self.window.width.max(0);
        let height = (self.window.height - TITLE_H).max(0);
        let sidebar_w = SIDEBAR_W.min(width / 3).max(140);
        let main_x = sidebar_w + 1;
        let main_w = (width - main_x).max(0);
        let status_y = (height - STATUS_H).max(0);
        Layout {
            width,
            height,
            sidebar_w,
            main_x,
            main_w,
            status_y,
        }
    }

    pub(super) fn breadcrumb_rect(&self) -> Rect {
        let layout = self.layout();
        let search = self.search_rect(layout);
        let fwd = self.forward_button_rect();
        Rect {
            x: fwd.x + fwd.w + BACK_BTN_GAP,
            y: COMMAND_H + 4,
            w: (search.x - (fwd.x + fwd.w + BACK_BTN_GAP) - 12).max(104),
            h: 22,
        }
    }

    pub(super) fn back_button_rect(&self) -> Rect {
        Rect {
            x: 12,
            y: COMMAND_H + 4,
            w: BACK_BTN_W,
            h: 22,
        }
    }

    pub(super) fn forward_button_rect(&self) -> Rect {
        Rect {
            x: 12 + BACK_BTN_W + 4,
            y: COMMAND_H + 4,
            w: BACK_BTN_W,
            h: 22,
        }
    }

    pub(super) fn search_rect(&self, layout: Layout) -> Rect {
        Rect {
            x: layout.width - 170,
            y: COMMAND_H + 4,
            w: 156,
            h: 22,
        }
    }

    pub(super) fn file_op_toggle_rect(&self, layout: Layout) -> Rect {
        Rect {
            x: (layout.width - 166).max(0),
            y: layout.status_y + 3,
            w: 76,
            h: 14,
        }
    }

    pub(super) fn file_op_cancel_rect(&self, layout: Layout) -> Rect {
        Rect {
            x: (layout.width - 84).max(0),
            y: layout.status_y + 3,
            w: 74,
            h: 14,
        }
    }

    pub(super) fn title_y(&self) -> i32 {
        COMMAND_H + PATHBAR_H + 14
    }

    pub(super) fn section_start_y(&self) -> i32 {
        self.title_y() + 34
    }

    pub(super) fn detail_rect(&self, layout: Layout) -> Option<Rect> {
        if !self.details_visible {
            return None;
        }
        if layout.main_w < 520 {
            return None;
        }
        let h = layout.status_y - COMMAND_H - PATHBAR_H - 20;
        Some(Rect {
            x: layout.main_x + layout.main_w - DETAIL_W - 14,
            y: COMMAND_H + PATHBAR_H + 10,
            w: DETAIL_W,
            h,
        })
    }

    pub(super) fn content_left(&self, layout: Layout) -> i32 {
        layout.main_x + 18
    }

    pub(super) fn content_right(&self, layout: Layout) -> i32 {
        if let Some(detail) = self.detail_rect(layout) {
            detail.x - DETAIL_GAP
        } else {
            layout.main_x + layout.main_w - 18
        }
    }

    pub(super) fn content_width(&self, layout: Layout) -> i32 {
        (self.content_right(layout) - self.content_left(layout)).max(120)
    }

    pub(super) fn folder_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| if entry.is_dir { Some(idx) } else { None })
            .collect()
    }

    pub(super) fn file_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| if entry.is_dir { None } else { Some(idx) })
            .collect()
    }

    pub(super) fn file_columns(&self, layout: Layout) -> FileColumns {
        let row_x = self.content_left(layout);
        let row_w = self.content_width(layout);
        let size_x = row_x + row_w - 84;
        let type_x = row_x + row_w - 180;
        let name_x = row_x + 28;
        let name_w = (type_x - name_x - 12).max(96);
        FileColumns {
            row_x,
            row_w,
            name_x,
            name_w,
            type_x,
            size_x,
        }
    }

    pub(super) fn file_rows_y(&self, _layout: Layout) -> i32 {
        self.section_start_y() + 22 + FILE_HEADER_H
    }

    pub(super) fn root_directory_names() -> Vec<String> {
        let mut names: Vec<String> = crate::vfs::vfs_list_dir("/")
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.is_dir)
            .map(|entry| entry.name)
            .collect();
        names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
        names
    }

    pub(super) fn shell_link_path_with_roots(label: &str, root_names: &[String]) -> String {
        if let Some(name) = root_names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(label))
        {
            let mut path = String::from("/");
            path.push_str(name);
            return path;
        }

        if label.eq_ignore_ascii_case("Home") {
            return String::from("/");
        }

        let mut path = String::from("/");
        path.push_str(label);
        path
    }

    pub(super) fn current_path_links(&self) -> Vec<(usize, String, String)> {
        let mut items = Vec::new();
        if self.path == "/" {
            return items;
        }

        let components: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
        if components.len() <= 1 {
            return items;
        }

        let mut path = String::new();
        for (depth, component) in components.iter().enumerate() {
            path.push('/');
            path.push_str(component);
            if depth == 0 {
                continue;
            }
            items.push((depth - 1, String::from(*component), path.clone()));
        }
        items
    }

    pub(super) fn path_matches_or_contains(&self, path: &str) -> bool {
        self.path.eq_ignore_ascii_case(path)
            || self
                .path
                .strip_prefix(path)
                .map(|suffix| suffix.starts_with('/'))
                .unwrap_or(false)
    }

    pub(super) fn breadcrumb_segments(&self) -> Vec<(String, String)> {
        let mut segments = Vec::new();
        segments.push((String::from("This PC"), String::from("/")));
        if self.path == "/" {
            return segments;
        }

        let mut built = String::new();
        for component in self.path.split('/').filter(|s| !s.is_empty()) {
            built.push('/');
            built.push_str(component);
            segments.push((String::from(component), built.clone()));
        }
        segments
    }

    pub(super) fn breadcrumb_segment_rects(&self, rect: Rect) -> Vec<(Rect, String, String)> {
        let mut out = Vec::new();
        let mut x = rect.x + 8;
        let y = rect.y + 3;
        let right = rect.x + rect.w - 8;
        let segments = self.breadcrumb_segments();
        let segment_len = segments.len();
        for (idx, (label, path)) in segments.into_iter().enumerate() {
            let seg_w = (label.len() as i32 * CW as i32 + BREAD_SEG_PAD * 2).max(44);
            if x + seg_w > right {
                break;
            }
            out.push((
                Rect {
                    x,
                    y,
                    w: seg_w,
                    h: 16,
                },
                label,
                path,
            ));
            x += seg_w;
            if idx + 1 < segment_len {
                x += BREAD_SEG_GAP + CW as i32;
            }
        }
        out
    }

    pub(super) fn hit_breadcrumb(&self, lx: i32, ly: i32) -> Option<String> {
        let crumb = self.breadcrumb_rect();
        if !crumb.hit(lx, ly) {
            return None;
        }
        for (seg, _label, path) in self.breadcrumb_segment_rects(crumb) {
            if seg.hit(lx, ly) {
                return Some(path);
            }
        }
        None
    }

    pub(super) fn hit_navigation(&self, lx: i32, ly: i32) -> Option<String> {
        let layout = self.layout();
        if lx >= layout.sidebar_w || ly < COMMAND_H + PATHBAR_H {
            return None;
        }
        let mut y = COMMAND_H + PATHBAR_H + 14;
        for item in self.sidebar_items() {
            match item.kind {
                SidebarItemKind::Section => y += 16,
                SidebarItemKind::Link => {
                    let rect = Rect {
                        x: 10,
                        y,
                        w: layout.sidebar_w - 20,
                        h: NAV_ROW_H,
                    };
                    if let Some(path) = item.path {
                        if rect.hit(lx, ly) {
                            return Some(path);
                        }
                    }
                    y += NAV_ROW_H + 4;
                }
            }
        }
        None
    }

    pub(super) fn hit_main_entry(&self, lx: i32, ly: i32) -> Option<usize> {
        let layout = self.layout();
        if lx < layout.main_x || ly < COMMAND_H + PATHBAR_H {
            return None;
        }

        if self.path == "/" {
            self.hit_root_entry(lx, ly)
        } else {
            self.hit_directory_entry(lx, ly)
        }
    }

    pub(super) fn hit_root_entry(&self, lx: i32, ly: i32) -> Option<usize> {
        let layout = self.layout();
        let folders = self.folder_indices();
        let files = self.file_indices();
        let section_y = self.section_start_y();
        if let Some(idx) = self.hit_folder_grid(layout, section_y, &folders, lx, ly) {
            return Some(idx);
        }
        let tiles_h = self.folder_section_height(layout, &folders);
        let drives_y = section_y + tiles_h + 20;
        self.hit_drive_grid(
            layout,
            drives_y,
            if files.is_empty() { &folders } else { &files },
            lx,
            ly,
        )
    }

    pub(super) fn hit_directory_entry(&self, _lx: i32, ly: i32) -> Option<usize> {
        let layout = self.layout();
        let list_y = self.file_rows_y(layout);
        if ly < list_y || ly >= layout.status_y - 10 {
            return None;
        }
        let visible = self.total_rows.max(1);
        let entry_idx = self.offset + ((ly - list_y) / LIST_ROW_H) as usize;
        if entry_idx >= self.entries.len() || entry_idx >= self.offset + visible {
            return None;
        }
        Some(entry_idx)
    }

    pub(super) fn hit_file_header_column(&self, lx: i32, ly: i32) -> Option<SortColumn> {
        if self.path == "/" {
            return None;
        }
        let layout = self.layout();
        let header_y = self.file_rows_y(layout) - FILE_HEADER_H;
        if ly < header_y || ly >= header_y + FILE_HEADER_H {
            return None;
        }
        let columns = self.file_columns(layout);
        if lx >= columns.name_x && lx < columns.type_x - 8 {
            Some(SortColumn::Name)
        } else if lx >= columns.type_x && lx < columns.size_x - 8 {
            Some(SortColumn::Type)
        } else if lx >= columns.size_x && lx < columns.row_x + columns.row_w {
            Some(SortColumn::Size)
        } else {
            None
        }
    }

    pub(super) fn hit_folder_grid(
        &self,
        layout: Layout,
        top: i32,
        indices: &[usize],
        lx: i32,
        ly: i32,
    ) -> Option<usize> {
        let content_left = self.content_left(layout);
        let content_w = self.content_width(layout);
        let tile_y = top + 22;
        let tile_w = ((content_w - 24 - TILE_GAP_X * 2) / 3).max(140);
        let cols = (content_w / (tile_w + TILE_GAP_X)).max(1) as usize;
        for (visual_idx, &entry_idx) in indices.iter().take(6).enumerate() {
            let col = (visual_idx % cols).min(2);
            let row = visual_idx / cols;
            let rect = Rect {
                x: content_left + col as i32 * (tile_w + TILE_GAP_X),
                y: tile_y + row as i32 * (TILE_H + TILE_GAP_Y),
                w: tile_w,
                h: TILE_H,
            };
            if rect.hit(lx, ly) {
                return Some(entry_idx);
            }
        }
        None
    }

    pub(super) fn hit_drive_grid(
        &self,
        layout: Layout,
        y: i32,
        indices: &[usize],
        lx: i32,
        ly: i32,
    ) -> Option<usize> {
        let content_left = self.content_left(layout);
        let content_w = self.content_width(layout);
        let card_w = ((content_w - 12) / 2).max(180);
        let cols = if content_w > 420 { 2 } else { 1 };
        let max_items = if cols == 2 { 4 } else { 3 };
        for (visual_idx, &entry_idx) in indices.iter().take(max_items).enumerate() {
            let col = (visual_idx % cols as usize) as i32;
            let row = (visual_idx / cols as usize) as i32;
            let rect = Rect {
                x: content_left + col * (card_w + 12),
                y: y + 22 + row * (DRIVE_H + DRIVE_GAP_Y),
                w: card_w,
                h: DRIVE_H,
            };
            if rect.hit(lx, ly) {
                return Some(entry_idx);
            }
        }
        None
    }

    pub(super) fn folder_section_height(&self, layout: Layout, indices: &[usize]) -> i32 {
        if indices.is_empty() {
            return 40;
        }
        let content_w = self.content_width(layout);
        let tile_w = ((content_w - 24 - TILE_GAP_X * 2) / 3).max(140);
        let cols = (content_w / (tile_w + TILE_GAP_X)).max(1) as usize;
        let display_count = indices.len().min(6);
        let rows = ((display_count + cols - 1) / cols).max(1) as i32;
        22 + rows * TILE_H + (rows - 1) * TILE_GAP_Y
    }
}
