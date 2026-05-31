impl BrowserApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, BROWSER_W, BROWSER_H, "Web Browser");
        let mut app = Self {
            window,
            address: String::from("browser://home"),
            status: String::from("Ready"),
            title: String::from("New tab"),
            lines: welcome_lines(),
            history: Vec::new(),
            bookmarks: load_bookmarks(),
            history_index: 0,
            scroll: 0,
            rows: 0,
            cols: 0,
            address_focused: true,
            address_selected: true,
            last_width: BROWSER_W,
            last_height: BROWSER_H,
            last_page: None,
            subresource_cache: BrowserSubresourceCache::default(),
            subresource_stats: BrowserSubresourceStats::default(),
            script_stats: BrowserScriptStats::default(),
            bypass_subresource_cache: false,
            image_preview: None,
            inline_images: Vec::new(),
            hit_boxes: Vec::new(),
            document: None,
            compat_state: BrowserCompatState::default(),
            pending_open: None,
        };
        app.render();
        app
    }

    pub fn open_url(x: i32, y: i32, url: &str) -> Self {
        let mut app = Self::new(x, y);
        app.navigate(url, true);
        app
    }

    pub fn trim_memory_pressure(&mut self) -> usize {
        let mut bytes = self.subresource_cache.trim_memory_pressure();
        if let Some(page) = self.last_page.take() {
            bytes = bytes
                .saturating_add(page.body.len())
                .saturating_add(page.body_bytes.len());
        }
        if let Some(image) = self.image_preview.take() {
            bytes = bytes.saturating_add(image.pixels.len().saturating_mul(4));
        }
        for inline in self.inline_images.drain(..) {
            bytes = bytes.saturating_add(inline.image.pixels.len().saturating_mul(4));
        }
        bytes
    }

    pub fn handle_key(&mut self, c: char) {
        if self.address_focused {
            match c {
                '\n' | '\r' => {
                    let url = self.address.clone();
                    self.address_selected = false;
                    self.navigate(&url, true);
                }
                '\u{8}' | '\u{7f}' => {
                    if self.address_selected {
                        self.address.clear();
                        self.address_selected = false;
                    } else {
                        self.address.pop();
                    }
                    self.render();
                }
                _ if !c.is_control() && self.address.len() < 192 => {
                    if self.address_selected {
                        self.address.clear();
                        self.address_selected = false;
                    }
                    self.address.push(c);
                    self.render();
                }
                _ => {}
            }
            return;
        }

        if self.handle_document_key(c) {
            return;
        }

        match c {
            'j' | 'J' => self.scroll_by(1),
            'k' | 'K' => self.scroll_by(-1),
            'r' => self.reload(),
            'R' => self.hard_reload(),
            'b' | 'B' => self.bookmark_current(),
            'c' | 'C' => self.navigate(CACHE_INTERNAL_URL, true),
            'd' | 'D' => self.save_current_page(false),
            'h' | 'H' => self.navigate("browser://history", true),
            'm' | 'M' => self.navigate("browser://bookmarks", true),
            'o' | 'O' => self.open_downloads_folder(),
            's' | 'S' => self.save_current_page(true),
            'g' | 'G' => {
                self.address_focused = true;
                self.address_selected = true;
                self.render();
            }
            _ => {}
        }
    }

    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        if ly >= 10 && ly < 34 {
            if lx >= 14 && lx < 44 {
                self.back();
            } else if lx >= 48 && lx < 78 {
                self.forward();
            } else if lx >= REFRESH_BUTTON_X && lx < REFRESH_BUTTON_X + REFRESH_BUTTON_W {
                self.reload();
            } else {
                let search_x = self.window.width - (SEARCH_BUTTON_W + 16);
                if lx >= search_x && lx < search_x + SEARCH_BUTTON_W {
                    let url = self.address.clone();
                    self.address_selected = false;
                    self.navigate(&url, true);
                } else if lx >= ADDRESS_X && lx < search_x - 8 {
                    self.address_focused = true;
                    self.address_selected = true;
                    self.render();
                }
            }
            return;
        }

        self.address_focused = false;
        if lx >= 0 && ly >= 0 {
            let lx = lx as usize;
            let ly = ly as usize;
            for hit in self.hit_boxes.iter().rev() {
                if lx >= hit.x
                    && lx < hit.x.saturating_add(hit.w)
                    && ly >= hit.y
                    && ly < hit.y.saturating_add(hit.h)
                {
                    if let Some(control_id) = hit.control_id {
                        self.activate_document_control(control_id);
                        return;
                    }
                    if let Some(link) = hit.link.as_ref() {
                        let resolved = resolve_url(&self.address, link);
                        if let Some(label) = browser_event_label(&resolved) {
                            self.status = format!("DOM event: {}", label);
                            self.render();
                            return;
                        }
                        if self.open_file_url(&resolved) {
                            return;
                        }
                        self.navigate(&resolved, true);
                        return;
                    }
                }
            }
        }
        self.render();
    }

    pub fn handle_scroll(&mut self, delta: i32) {
        self.scroll_by(delta.signum() * 3);
    }

    pub fn update(&mut self) {
        if self.window.width != self.last_width || self.window.height != self.last_height {
            self.last_width = self.window.width;
            self.last_height = self.window.height;
            self.render();
            return;
        }
        let expected = self.scroll as i32;
        if self.window.scroll.offset != expected {
            self.scroll = self.window.scroll.offset.max(0) as usize;
            self.render();
        }
    }

    pub fn take_open_request(&mut self) -> Option<FileManagerOpenRequest> {
        self.pending_open.take()
    }

    fn navigate(&mut self, url: &str, add_history: bool) {
        let url = normalize_address_input(url);
        self.address = url.clone();
        if self.render_internal_page(&url, add_history) {
            return;
        }
        if self.open_file_url(&url) {
            if add_history {
                self.push_history(url);
            }
            return;
        }
        self.status = String::from("Loading...");
        self.title = String::from("Loading");
        self.image_preview = None;
        self.inline_images.clear();
        self.document = None;
        self.lines = vec![BrowserLine::new(
            format!("Loading {}", url),
            None,
            BrowserLineKind::Muted,
        )];
        self.scroll = 0;
        self.render();

        match parse_web_url(&url) {
            Ok((_scheme, host, path)) => match crate::net::browser_get_response(&url) {
                Ok(response) => {
                    self.apply_web_response(response, add_history, "Loaded");
                }
                Err(err) => {
                    self.title = String::from("Load failed");
                    self.status = format!("Network error: {}", err);
                    self.last_page = None;
                    self.image_preview = None;
                    self.inline_images.clear();
                    self.document = None;
                    self.lines = network_error_lines(&url, &host, &path, err);
                }
            },
            Err(err) => {
                self.title = String::from("Unsupported URL");
                self.status = String::from(err);
                self.last_page = None;
                self.image_preview = None;
                self.inline_images.clear();
                self.document = None;
                self.lines = vec![
                    line("Enter an http:// or https:// URL."),
                    link_line("Try https://example.com/", "https://example.com/"),
                ];
            }
        }
        self.address_focused = false;
        self.address_selected = false;
        self.bypass_subresource_cache = false;
        self.render();
    }

    fn apply_web_response(
        &mut self,
        response: crate::net::HttpResponse,
        add_history: bool,
        success_label: &str,
    ) {
        self.title = extract_title(&response.body).unwrap_or_else(|| response.host.clone());
        self.address = response.final_url.clone();
        let security = match response.tls_trust_root {
            Some(root) => format!("  Secure: {}", root),
            None => String::new(),
        };
        self.status = if is_success_status(&response.status_line) {
            format!("{} {}{}", success_label, response.final_url, security)
        } else if response.redirect_count > 0 {
            format!(
                "{}  {} redirect(s) -> {}{}",
                response.status_line, response.redirect_count, response.final_url, security
            )
        } else {
            format!(
                "{}  {}{} -> {}{}",
                response.status_line,
                response.host,
                response.path,
                crate::net::ipv4_string(response.resolved_addr),
                security
            )
        };
        self.last_page = Some(CachedPage {
            url: response.final_url.clone(),
            body: response.body.clone(),
            body_bytes: response.body_bytes.clone(),
            content_type: response.content_type.clone(),
        });
        let content_type = response.content_type.as_deref();
        self.lines = if is_image_content(content_type) {
            self.inline_images.clear();
            self.document = None;
            self.compat_state = BrowserCompatState::native(&response.final_url);
            let preview_status = self.decode_image_preview(&response);
            image_response_lines(
                &response.final_url,
                content_type,
                response.body_bytes.len(),
                preview_status.as_deref(),
            )
        } else if is_html_main_content(content_type, &response.final_url, &response.body_bytes) {
            self.image_preview = None;
            self.set_html_document(&response.final_url, &response.body);
            self.append_subresource_status();
            self.append_script_status();
            self.append_compat_status();
            self.lines.clone()
        } else {
            self.image_preview = None;
            self.inline_images.clear();
            self.document = None;
            self.subresource_stats = BrowserSubresourceStats::default();
            self.script_stats = BrowserScriptStats::default();
            self.compat_state = BrowserCompatState::source(&response.final_url, content_type);
            self.title = source_title_for_content(content_type, &response.final_url);
            self.append_compat_status();
            source_response_lines(
                &response.final_url,
                content_type,
                &response.body,
                response.body_bytes.len(),
                self.cols.max(48),
            )
        };
        if self.lines.is_empty() {
            self.lines.push(BrowserLine::new(
                String::from("(empty response)"),
                None,
                BrowserLineKind::Muted,
            ));
        }
        if response.session_cookies_stored > 0 {
            self.status
                .push_str(&format!("  cookies={}", response.session_cookies_stored));
        }
        if add_history {
            self.push_history(response.final_url);
        }
    }

    fn render_internal_page(&mut self, url: &str, add_history: bool) -> bool {
        if !url.starts_with("browser://") {
            return false;
        }
        self.scroll = 0;
        self.address_focused = false;
        self.address_selected = false;
        self.bypass_subresource_cache = false;
        self.last_page = None;
        self.image_preview = None;
        self.inline_images.clear();
        self.document = None;
        if add_history {
            self.push_history(String::from(url));
        }
        match url {
            "browser://home" => {
                self.title = String::from("Home");
                self.status = String::from("Ready");
                self.lines = welcome_lines();
            }
            "browser://history" => {
                self.title = String::from("History");
                self.status = format!("{} item(s)", self.history.len());
                self.lines = history_lines(&self.history);
            }
            "browser://bookmarks" => {
                self.title = String::from("Bookmarks");
                self.status = format!("{} bookmark(s)", self.bookmarks.len());
                self.lines = bookmark_lines(&self.bookmarks);
            }
            "browser://downloads" => {
                self.title = String::from("Downloads");
                self.status = String::from(DOWNLOADS_DIR);
                self.lines = downloads_lines();
            }
            SESSION_INTERNAL_URL => {
                self.title = String::from("Session");
                self.status = crate::browser_session::summary_line();
                self.lines = browser_session_lines();
            }
            CACHE_INTERNAL_URL => {
                self.title = String::from("Cache");
                self.status = format!(
                    "{} cached subresource(s)",
                    self.subresource_cache.entries().len()
                );
                self.lines = self.browser_cache_lines();
            }
            COMPAT_INTERNAL_URL => {
                self.title = String::from("Compatibility");
                self.status = format!("mode={}", self.compat_state.mode);
                self.lines = self.browser_compat_lines();
            }
            ENGINE_INTERNAL_URL => {
                self.title = String::from("Engine Port");
                self.status = format!(
                    "target={} active={}",
                    crate::browser_engine::TARGET_ENGINE,
                    crate::browser_engine::active_engine_name()
                );
                self.lines = browser_engine_lines();
            }
            JS_INTERNAL_URL => {
                self.title = String::from("Scripts");
                self.status = script_stats_debug_line(self.script_stats);
                self.lines = self.browser_script_lines();
            }
            STORAGE_INTERNAL_URL => {
                self.title = String::from("Storage");
                self.status = crate::browser_storage::summary_line();
                self.lines = browser_storage_lines();
            }
            _ if url.starts_with("browser://search?q=") => {
                let query = decode_query(&url["browser://search?q=".len()..]);
                self.title = String::from("Search");
                self.status = format!("Local search: {}", query);
                self.lines = self.search_lines(&query);
            }
            _ => {
                self.title = String::from("Browser");
                self.status = String::from("Unknown internal page");
                self.lines = vec![
                    line("Page not found"),
                    line(""),
                    link_line("Home", "browser://home"),
                ];
            }
        }
        self.render();
        true
    }

    fn push_history(&mut self, url: String) {
        if self
            .history
            .get(self.history_index)
            .map(|current| current == &url)
            .unwrap_or(false)
        {
            return;
        }
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(url);
        self.history_index = self.history.len().saturating_sub(1);
    }

    fn back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            if let Some(url) = self.history.get(self.history_index).cloned() {
                self.navigate(&url, false);
            }
        }
    }

    fn forward(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            if let Some(url) = self.history.get(self.history_index).cloned() {
                self.navigate(&url, false);
            }
        }
    }

    fn reload(&mut self) {
        let url = self.address.clone();
        self.navigate(&url, false);
    }

    fn hard_reload(&mut self) {
        self.bypass_subresource_cache = true;
        let url = self.address.clone();
        self.navigate(&url, false);
    }

    fn bookmark_current(&mut self) {
        let url = self.address.clone();
        if url.starts_with("browser://") {
            self.status = String::from("Internal pages are not bookmarked");
            self.render();
            return;
        }
        if !self.bookmarks.iter().any(|bookmark| bookmark == &url) {
            self.bookmarks.push(url.clone());
            if self.bookmarks.len() > MAX_BOOKMARKS {
                self.bookmarks.remove(0);
            }
            save_bookmarks(&self.bookmarks);
            self.status = format!("Bookmarked {}", url);
        } else {
            self.status = format!("Already bookmarked {}", url);
        }
        self.render();
    }

    fn save_current_page(&mut self, source: bool) {
        let Some(page) = self.last_page.clone() else {
            self.status = String::from("Nothing loaded to save");
            self.render();
            return;
        };
        let _ = crate::vfs::vfs_create_dir(DOWNLOADS_DIR);
        let filename = download_filename(&page.url, page.content_type.as_deref(), source);
        let mut path = String::from(DOWNLOADS_DIR);
        path.push('/');
        path.push_str(&filename);
        let data = if source {
            response_body_text(&page.body)
                .unwrap_or(page.body.as_str())
                .as_bytes()
                .to_vec()
        } else {
            page.body_bytes
        };
        match crate::vfs::vfs_safe_write_file(&path, &data) {
            Ok(()) => {
                self.status = format!("Saved {}", path);
                self.lines = vec![
                    kind_line("Saved download", BrowserLineKind::Heading),
                    line(""),
                    link_line(&path, &file_url_for_path(&path)),
                    kind_line(&format!("{} bytes", data.len()), BrowserLineKind::Muted),
                    link_line("Open Downloads", "browser://downloads"),
                ];
                self.document = None;
            }
            Err(err) => {
                self.status = format!("Save failed: {}", err.as_str());
            }
        }
        self.image_preview = None;
        self.inline_images.clear();
        self.document = None;
        self.render();
    }

    fn open_downloads_folder(&mut self) {
        let _ = crate::vfs::vfs_create_dir(DOWNLOADS_DIR);
        self.pending_open = Some(FileManagerOpenRequest::Dir(String::from(DOWNLOADS_DIR)));
        self.status = String::from("Opening Downloads in File Manager");
        self.render();
    }

    fn open_file_url(&mut self, url: &str) -> bool {
        let Some(path) = url.strip_prefix("file://") else {
            return false;
        };
        let path = if path.is_empty() { "/" } else { path };
        if crate::vfs::vfs_list_dir(path).is_some() {
            self.pending_open = Some(FileManagerOpenRequest::Dir(String::from(path)));
            self.status = format!("Opening {}", path);
        } else if let Some(bytes) = crate::vfs::vfs_read_file(path) {
            if is_known_image_path(path) || looks_like_image_bytes(&bytes) {
                self.show_local_image(path, bytes);
                return true;
            }
            if is_html_path(path) || looks_like_html_bytes(&bytes) {
                self.show_local_html(path, bytes);
                return true;
            }
            self.pending_open = Some(FileManagerOpenRequest::File(String::from(path)));
            self.status = format!("Opening {}", path);
        } else {
            self.title = String::from("File not found");
            self.status = format!("Missing {}", path);
            self.lines = vec![
                kind_line("File not found", BrowserLineKind::Error),
                kind_line(path, BrowserLineKind::Muted),
                link_line("Downloads", "browser://downloads"),
            ];
            self.image_preview = None;
            self.inline_images.clear();
            self.document = None;
        }
        self.address_focused = false;
        self.address_selected = false;
        self.bypass_subresource_cache = false;
        self.render();
        true
    }

    fn show_local_image(&mut self, path: &str, bytes: Vec<u8>) {
        let url = file_url_for_path(path);
        self.address = url.clone();
        self.title = String::from("Image");
        self.status = format!("Local image {}", path);
        self.inline_images.clear();
        self.document = None;
        let content_type = image_content_type_for(path, &bytes).unwrap_or("image/*");
        self.last_page = Some(CachedPage {
            url: url.clone(),
            body: String::new(),
            body_bytes: bytes.clone(),
            content_type: Some(String::from(content_type)),
        });
        let preview_status = self.decode_image_preview_bytes(&bytes, Some(content_type), path);
        self.lines = image_response_lines(
            &url,
            Some(content_type),
            bytes.len(),
            preview_status.as_deref(),
        );
        self.address_focused = false;
        self.address_selected = false;
        self.bypass_subresource_cache = false;
        self.render();
    }

    fn show_local_html(&mut self, path: &str, bytes: Vec<u8>) {
        let url = file_url_for_path(path);
        let body = String::from_utf8_lossy(&bytes).into_owned();
        self.address = url.clone();
        self.title = extract_title(&body).unwrap_or_else(|| String::from("Local HTML"));
        self.status = format!("Local HTML {}", path);
        self.image_preview = None;
        self.inline_images.clear();
        self.last_page = Some(CachedPage {
            url: url.clone(),
            body: body.clone(),
            body_bytes: bytes,
            content_type: Some(String::from("text/html")),
        });
        self.set_html_document(&url, &body);
        self.append_subresource_status();
        self.append_script_status();
        self.append_compat_status();
        self.lines = if self.lines.is_empty() {
            vec![kind_line("(empty document)", BrowserLineKind::Muted)]
        } else {
            self.lines.clone()
        };
        self.address_focused = false;
        self.address_selected = false;
        self.bypass_subresource_cache = false;
        self.render();
    }

    fn search_lines(&self, query: &str) -> Vec<BrowserLine> {
        let mut out = vec![line("Search"), line("")];
        let query_lower = lowercase_ascii(query);
        let mut matches = 0usize;
        for url in self
            .history
            .iter()
            .chain(self.bookmarks.iter())
            .filter(|url| lowercase_ascii(url).contains(&query_lower))
        {
            out.push(link_line(url, url));
            matches += 1;
        }
        if matches == 0 {
            out.push(line("No local matches."));
            if looks_like_url(query) {
                out.push(line(""));
                out.push(link_line(
                    "Open as web URL",
                    &normalize_address_input(query),
                ));
            }
        }
        out
    }

    fn browser_cache_lines(&self) -> Vec<BrowserLine> {
        let mut out = vec![
            kind_line("Browser Cache", BrowserLineKind::Heading),
            kind_line(
                "In-memory CSS, image, and script subresources for this Browser window.",
                BrowserLineKind::Muted,
            ),
            line(""),
        ];
        let stats = self.subresource_stats;
        out.push(kind_line(
            &format!(
                "last load: css={}/{} images={} placeholders={} failed={} cache={}/{}",
                stats.stylesheets_loaded,
                stats
                    .stylesheets_loaded
                    .saturating_add(stats.stylesheets_failed),
                stats.images_loaded,
                stats.image_placeholders,
                stats.images_failed,
                stats.cache_hits,
                stats.cache_hits.saturating_add(stats.cache_misses)
            ),
            BrowserLineKind::Muted,
        ));
        if self.subresource_cache.entries().is_empty() {
            out.push(kind_line(
                "No cached subresources yet.",
                BrowserLineKind::Muted,
            ));
            return out;
        }
        let now = crate::interrupts::ticks();
        for entry in self.subresource_cache.entries().iter().rev().take(32) {
            let age = now
                .saturating_sub(entry.created_tick)
                .saturating_div(crate::interrupts::TIMER_HZ as u64);
            let used = now
                .saturating_sub(entry.last_used_tick)
                .saturating_div(crate::interrupts::TIMER_HZ as u64);
            out.push(link_line(
                &format!(
                    "{}  {} bytes  hits={}  age={}s  used={}s  {}",
                    entry.kind.label(),
                    entry.bytes.len(),
                    entry.hits,
                    age,
                    used,
                    entry
                        .content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream")
                ),
                &entry.final_url,
            ));
            if entry.requested_url != entry.final_url {
                out.push(kind_line(
                    &format!("requested {}", entry.requested_url),
                    BrowserLineKind::Muted,
                ));
            }
        }
        out
    }

    fn browser_compat_lines(&self) -> Vec<BrowserLine> {
        let compat = &self.compat_state;
        let mut out = vec![
            kind_line("Browser Compatibility", BrowserLineKind::Heading),
            kind_line(
                "Current handling path for the last loaded main resource.",
                BrowserLineKind::Muted,
            ),
            line(""),
            line(&format!("Mode: {}", compat.mode)),
            line(&format!("URL: {}", compat.url)),
            line(&format!("Reason: {}", compat.reason)),
        ];
        if !compat.notes.is_empty() {
            out.push(line(""));
            out.push(kind_line("Notes", BrowserLineKind::Muted));
            for note in compat.notes.iter() {
                out.push(line(&format!("- {}", note)));
            }
        }
        out.push(line(""));
        out.push(link_line("Home", "browser://home"));
        out.push(link_line("Script diagnostics", JS_INTERNAL_URL));
        out.push(link_line("Cache state", CACHE_INTERNAL_URL));
        out.push(link_line("Engine port", ENGINE_INTERNAL_URL));
        out
    }

    fn browser_script_lines(&self) -> Vec<BrowserLine> {
        let stats = self.script_stats;
        let mut out = vec![
            kind_line("Browser Scripts", BrowserLineKind::Heading),
            kind_line(
                "Bounded document scripts, event handlers, timers, and DOM mutations.",
                BrowserLineKind::Muted,
            ),
            line(""),
            kind_line(&script_stats_debug_line(stats), BrowserLineKind::Muted),
            line(""),
            line(&format!(
                "scripts: inline={} external={}/{}",
                stats.inline_scripts,
                stats.external_scripts,
                stats.external_scripts.saturating_add(stats.external_failed)
            )),
            line(&format!(
                "runtime: handlers={} timers={} statements={}",
                stats.handlers, stats.timers, stats.statements
            )),
            line(&format!(
                "dom: mutations={} errors={}",
                stats.mutations, stats.errors
            )),
            line(&format!(
                "web APIs: storage={}/{} cookies={}/{} fetch={} nav={}",
                stats.storage_reads,
                stats.storage_writes,
                stats.cookie_reads,
                stats.cookie_writes,
                stats.fetches,
                stats.navigation_requests
            )),
        ];
        if stats.errors > 0 {
            out.push(kind_line(
                "Unsupported script statements were skipped by the bounded runtime.",
                BrowserLineKind::Muted,
            ));
        }
        out
    }

    fn append_subresource_status(&mut self) {
        let stats = self.subresource_stats;
        if !stats.has_activity() {
            return;
        }
        let css_total = stats
            .stylesheets_loaded
            .saturating_add(stats.stylesheets_failed);
        if css_total > 0 {
            self.status
                .push_str(&format!("  css={}/{}", stats.stylesheets_loaded, css_total));
        }
        let image_total = stats
            .images_loaded
            .saturating_add(stats.image_placeholders)
            .saturating_add(stats.images_failed);
        if image_total > 0 {
            self.status.push_str(&format!(
                "  images={} placeholders={} failed={}",
                stats.images_loaded, stats.image_placeholders, stats.images_failed
            ));
        }
        let cache_total = stats.cache_hits.saturating_add(stats.cache_misses);
        if cache_total > 0 {
            self.status
                .push_str(&format!("  cache={}/{}", stats.cache_hits, cache_total));
        }
    }

    fn append_script_status(&mut self) {
        let stats = self.script_stats;
        if !stats.has_activity() {
            return;
        }
        let total = stats
            .inline_scripts
            .saturating_add(stats.external_scripts)
            .saturating_add(stats.external_failed);
        self.status.push_str(&format!(
            "  js={}/{} handlers={} timers={} mut={} api={} err={}",
            stats.inline_scripts.saturating_add(stats.external_scripts),
            total,
            stats.handlers,
            stats.timers,
            stats.mutations,
            stats
                .storage_reads
                .saturating_add(stats.storage_writes)
                .saturating_add(stats.cookie_reads)
                .saturating_add(stats.cookie_writes)
                .saturating_add(stats.fetches)
                .saturating_add(stats.navigation_requests),
            stats.errors
        ));
    }

    fn append_compat_status(&mut self) {
        if self.compat_state.mode != "native" {
            self.status
                .push_str(&format!("  compat={}", self.compat_state.mode));
        }
    }

    fn set_html_document(&mut self, base_url: &str, body: &str) -> usize {
        self.subresource_stats = BrowserSubresourceStats::default();
        self.script_stats = BrowserScriptStats::default();
        let body_text = response_body_text(body).unwrap_or(body);
        let effective_base = extract_base_href(body_text, base_url);
        let compat_body = google_search_compat_document(&effective_base, body_text);
        let render_body = compat_body.as_deref().unwrap_or(body_text);
        self.compat_state = if compat_body.is_some() {
            BrowserCompatState::google_search(&effective_base)
        } else {
            BrowserCompatState::native(&effective_base)
        };
        let external_css = load_document_stylesheets(
            &effective_base,
            render_body,
            &mut self.subresource_cache,
            &mut self.subresource_stats,
            self.bypass_subresource_cache,
        );
        let scripts = load_document_scripts(
            &effective_base,
            render_body,
            &mut self.subresource_cache,
            &mut self.subresource_stats,
            self.bypass_subresource_cache,
        );
        let document = BrowserDocumentState::from_html_with_external_css_and_scripts(
            &effective_base,
            render_body,
            external_css,
            scripts.sources,
            scripts.stats,
        );
        self.script_stats = document.script_stats;
        self.document = Some(document);
        let images = self.reflow_document();
        self.bypass_subresource_cache = false;
        images
    }

    fn reflow_document(&mut self) -> usize {
        let Some(document) = self.document.as_ref() else {
            return 0;
        };
        let mut lines = render_document_interactive(
            &document.base_url,
            &document.source,
            self.cols.max(48),
            document,
        );
        let images = attach_html_images_with_cache(
            &mut lines,
            &mut self.inline_images,
            &mut self.subresource_cache,
            &mut self.subresource_stats,
            self.cols.max(48),
            self.bypass_subresource_cache,
        );
        self.lines = if lines.is_empty() {
            vec![kind_line("(empty document)", BrowserLineKind::Muted)]
        } else {
            lines
        };
        images
    }

    fn focused_control_id(&self) -> Option<usize> {
        self.document
            .as_ref()
            .and_then(|document| document.focused_control)
    }

    fn handle_document_key(&mut self, c: char) -> bool {
        if c == '\t' {
            if let Some(document) = self.document.as_mut() {
                if document.focus_next_control() {
                    self.status = String::from("Control focused");
                    self.render();
                    return true;
                }
            }
        }
        let Some(document) = self.document.as_mut() else {
            return false;
        };
        if document.focused_control.is_none() {
            return false;
        }
        if c == '\u{1b}' {
            document.focused_control = None;
            self.status = String::from("Control focus cleared");
            self.render();
            return true;
        }
        if c == '\n' || c == '\r' || c == ' ' {
            let Some(id) = document.focused_control else {
                return false;
            };
            let kind = document.controls.get(id).map(|control| control.kind);
            if matches!(kind, Some(BrowserFormControlKind::Text)) && (c == '\n' || c == '\r') {
                if let Some(submit_id) = document.default_submit_for(id) {
                    self.activate_document_control(submit_id);
                    return true;
                }
            }
            if matches!(
                kind,
                Some(
                    BrowserFormControlKind::Submit
                        | BrowserFormControlKind::Button
                        | BrowserFormControlKind::Reset
                        | BrowserFormControlKind::Image
                )
            ) {
                self.activate_document_control(id);
                return true;
            }
        }
        if document.edit_focused_control(c) {
            self.status = String::from("Control edited");
            self.reflow_document();
            self.render();
            return true;
        }
        false
    }

    fn activate_document_control(&mut self, control_id: usize) {
        let activation = {
            let Some(document) = self.document.as_mut() else {
                return;
            };
            document.activate_control(control_id)
        };
        if let Some(document) = self.document.as_ref() {
            self.script_stats = document.script_stats;
        }
        let pending_navigation = self
            .document
            .as_mut()
            .and_then(|document| document.pending_navigation.take());
        if let Some(url) = pending_navigation {
            self.status = format!("Script navigating {}", url);
            self.navigate(&url, true);
            return;
        }
        match activation {
            BrowserControlActivation::Ignored => {
                self.status = String::from("Control unavailable");
                self.render();
            }
            BrowserControlActivation::Focused => {
                self.status = String::from("Control focused");
                self.render();
            }
            BrowserControlActivation::Changed => {
                self.status = if self.script_stats.mutations > 0 {
                    format!(
                        "Control changed  js mut={} err={}",
                        self.script_stats.mutations, self.script_stats.errors
                    )
                } else {
                    String::from("Control changed")
                };
                self.reflow_document();
                self.render();
            }
            BrowserControlActivation::Navigate(url) => {
                self.status = format!("Submitting {}", url);
                self.navigate(&url, true);
            }
            BrowserControlActivation::Post { url, body } => {
                self.submit_post_form(&url, &body);
            }
            BrowserControlActivation::DomEvent(label) => {
                self.status = format!("DOM event: {}", label);
                self.render();
            }
        }
    }

    fn submit_post_form(&mut self, url: &str, body: &str) {
        self.address = String::from(url);
        self.status = format!("Submitting POST {} bytes...", body.len());
        self.title = String::from("Submitting");
        self.image_preview = None;
        self.inline_images.clear();
        self.document = None;
        self.lines = vec![kind_line("Submitting form...", BrowserLineKind::Muted)];
        self.scroll = 0;
        self.render();

        match parse_web_url(url) {
            Ok((_scheme, host, path)) => {
                match crate::net::browser_post_response(
                    url,
                    body,
                    "application/x-www-form-urlencoded",
                ) {
                    Ok(response) => {
                        self.apply_web_response(response, true, "Submitted POST");
                    }
                    Err(err) => {
                        self.title = String::from("POST failed");
                        self.status = format!("Network error: {}", err);
                        self.last_page = None;
                        self.image_preview = None;
                        self.inline_images.clear();
                        self.document = None;
                        self.lines = network_error_lines(url, &host, &path, err);
                    }
                }
            }
            Err(err) => {
                self.title = String::from("Unsupported POST target");
                self.status = String::from(err);
                self.last_page = None;
                self.image_preview = None;
                self.inline_images.clear();
                self.document = None;
                self.lines = vec![
                    kind_line("POST form target is not a web URL", BrowserLineKind::Error),
                    kind_line(
                        "Use an http:// or https:// form action.",
                        BrowserLineKind::Muted,
                    ),
                    line(""),
                    kind_line("Target", BrowserLineKind::Muted),
                    line(url),
                    kind_line("Body", BrowserLineKind::Muted),
                    kind_line(body, BrowserLineKind::Code),
                ];
            }
        }
        self.address_focused = false;
        self.address_selected = false;
        self.render();
    }

    fn decode_image_preview(&mut self, response: &crate::net::HttpResponse) -> Option<String> {
        self.decode_image_preview_bytes(
            &response.body_bytes,
            response.content_type.as_deref(),
            &response.final_url,
        )
    }

    fn decode_image_preview_bytes(
        &mut self,
        bytes: &[u8],
        content_type: Option<&str>,
        url: &str,
    ) -> Option<String> {
        self.image_preview = None;
        if !is_png_content(content_type, url) {
            let meta = image_metadata_label(bytes, content_type, url)
                .unwrap_or_else(|| String::from("image dimensions unknown"));
            return Some(format!("Preview unavailable: {} (PNG decoder only)", meta));
        }
        match crate::png::decode_rgb8(bytes, MAX_INLINE_PNG_PIXELS) {
            Ok(image) => {
                let status = format!("PNG preview {}x{}", image.width, image.height);
                self.image_preview = Some(image);
                Some(status)
            }
            Err(err) => Some(format!("PNG preview unavailable: {}", err)),
        }
    }

    fn scroll_by(&mut self, delta: i32) {
        let viewport_h = self.rows.saturating_mul(LINE_H) as i32;
        let max = self
            .window
            .scroll
            .content_h
            .saturating_sub(viewport_h)
            .max(0);
        let next =
            (self.scroll as i32 + delta.saturating_mul(LINE_H as i32)).clamp(0, max) as usize;
        if next != self.scroll {
            self.scroll = next;
            self.render();
        }
    }

    fn render(&mut self) {
        let width = self.window.width.max(0) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        if width == 0 || content_h == 0 {
            return;
        }
        let stride = width;
        theme::fill_app_background(&mut self.window.buf, stride, content_h);
        theme::draw_glass_panel(
            &mut self.window.buf,
            stride,
            content_h,
            0,
            0,
            width,
            TOOLBAR_H,
            BUTTON_HOT,
        );
        self.fill_rect(
            stride,
            0,
            content_h.saturating_sub(STATUS_H),
            width,
            STATUS_H,
            BAR,
        );
        self.fill_rect(
            stride,
            0,
            content_h.saturating_sub(STATUS_H),
            width,
            1,
            BORDER,
        );

        self.draw_button(stride, 14, 10, 30, 24, "<", self.history_index > 0);
        self.draw_button(
            stride,
            48,
            10,
            30,
            24,
            ">",
            self.history_index + 1 < self.history.len(),
        );
        self.draw_button(
            stride,
            REFRESH_BUTTON_X as usize,
            10,
            REFRESH_BUTTON_W as usize,
            24,
            "Refresh",
            true,
        );

        let search_w = SEARCH_BUTTON_W as usize;
        let search_x = width.saturating_sub(search_w + 16);
        let addr_x = ADDRESS_X as usize;
        let addr_w = search_x.saturating_sub(addr_x + 8);
        let address_bg = if self.address_focused && self.address_selected {
            ADDRESS_SELECTED
        } else {
            ADDRESS_BG
        };
        theme::draw_control(
            &mut self.window.buf,
            stride,
            content_h,
            addr_x,
            10,
            addr_w,
            24,
            self.address_focused,
        );
        self.fill_rect(stride, addr_x + 1, 11, addr_w.saturating_sub(2), 21, address_bg);
        let mut address = self.address.clone();
        truncate_chars(&mut address, addr_w.saturating_sub(14) / CHAR_W);
        let address_text = if self.address_focused && self.address_selected {
            WHITE
        } else {
            CHROME_TEXT
        };
        self.put_str(stride, addr_x + 8, 17, &address, address_text);
        self.draw_button(stride, search_x, 10, search_w, 24, "Search", true);

        let mut title = self.title.clone();
        truncate_chars(&mut title, width.saturating_sub(PAD_X * 2) / CHAR_W);
        self.put_str(stride, PAD_X, 40, &title, CHROME_TEXT);

        let doc_y = TOOLBAR_H + 10;
        let doc_h = content_h.saturating_sub(TOOLBAR_H + STATUS_H + 18);
        self.fill_rect(
            stride,
            10,
            TOOLBAR_H + 6,
            width.saturating_sub(20),
            doc_h + 8,
            PAGE,
        );
        self.draw_rect(
            stride,
            10,
            TOOLBAR_H + 6,
            width.saturating_sub(20),
            doc_h + 8,
            theme::BORDER_SOFT,
        );

        let mut lines_y = doc_y;
        let mut lines_h = doc_h;
        if let Some(image) = self.image_preview.clone() {
            let preview_h = self.draw_image_preview(
                stride,
                PAD_X,
                doc_y,
                width.saturating_sub(PAD_X * 2),
                doc_h.saturating_sub(42).min(260),
                &image,
            );
            if preview_h > 0 {
                lines_y = lines_y.saturating_add(preview_h + 16);
                lines_h = doc_h.saturating_sub(preview_h + 16);
            }
        }

        let doc_w = width.saturating_sub(PAD_X * 2 + 28).max(1);
        self.rows = lines_h / LINE_H;
        self.cols = doc_w / CHAR_W;
        let layout = layout_browser_lines(&self.lines, &self.inline_images, doc_w);
        let max_scroll = layout.content_h.saturating_sub(lines_h);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        self.window.scroll.content_h = layout.content_h as i32;
        self.window.scroll.offset = self.scroll as i32;
        self.window.scroll.clamp(lines_h as i32);
        self.scroll = self.window.scroll.offset.max(0) as usize;
        self.hit_boxes.clear();

        let viewport_bottom = self.scroll.saturating_add(lines_h);
        for item in layout.items.into_iter() {
            if item.box_y.saturating_add(item.box_h) <= self.scroll || item.box_y >= viewport_bottom
            {
                continue;
            }
            if item.box_y < self.scroll || item.box_y.saturating_add(item.box_h) > viewport_bottom {
                continue;
            }
            let y = lines_y + item.y.saturating_sub(self.scroll);
            let x = PAD_X + item.x;
            let box_x = PAD_X + item.box_x;
            let box_y = lines_y + item.box_y.saturating_sub(self.scroll);
            if y >= doc_y.saturating_add(doc_h) {
                continue;
            }
            self.draw_box_decoration(stride, box_x, box_y, item.box_w, item.box_h, item.style);

            if let Some(slot) = item.image_slot {
                if let Some(image) = self
                    .inline_images
                    .get(slot)
                    .map(|inline| inline.image.clone())
                {
                    draw_image_preview_pixels(
                        &mut self.window.buf,
                        width,
                        content_h,
                        stride,
                        x,
                        y,
                        item.w,
                        item.h,
                        &image,
                        false,
                    );
                }
            } else if matches!(item.control, BrowserControl::None) {
                let color = item
                    .style
                    .text_color
                    .unwrap_or_else(|| color_for_line(item.kind, item.link.is_some()));
                let mut text = item.text.clone();
                truncate_chars(&mut text, item.w / CHAR_W);
                self.put_str(stride, x, y, &text, color);
                if item.kind == BrowserLineKind::Heading {
                    self.put_str(stride, x + 1, y, &text, color);
                }
                if item.link.is_some() {
                    self.fill_rect(stride, x, y + 10, text.len() * CHAR_W, 1, LINK);
                }
            } else {
                self.draw_control(
                    stride,
                    x,
                    y,
                    item.w,
                    &item.control,
                    item.link.is_some() || item.control_id.is_some(),
                    item.control_id == self.focused_control_id(),
                );
            }

            if item.link.is_some() || item.control_id.is_some() {
                self.hit_boxes.push(BrowserHitBox {
                    x: box_x,
                    y: box_y,
                    w: item.box_w.max(item.w),
                    h: item.box_h.max(item.h).max(LINE_H),
                    link: item.link,
                    control_id: item.control_id,
                });
            }
        }

        let mut status = self.status.clone();
        truncate_chars(&mut status, width.saturating_sub(PAD_X * 2) / CHAR_W);
        self.put_str(
            stride,
            PAD_X,
            content_h.saturating_sub(STATUS_H).saturating_add(5),
            &status,
            CHROME_MUTED,
        );
        self.window.mark_dirty_all();
    }

    fn draw_button(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        label: &str,
        enabled: bool,
    ) {
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        if enabled {
            theme::draw_control(&mut self.window.buf, stride, content_h, x, y, w, h, false);
        } else {
            self.fill_rect(stride, x, y, w, h, BUTTON_DIM);
            self.draw_rect(stride, x, y, w, h, BORDER);
        }
        let label_x = x + 6;
        let label_y = y + 8;
        self.put_str(
            stride,
            label_x,
            label_y,
            label,
            if enabled { CHROME_TEXT } else { CHROME_MUTED },
        );
    }

    fn draw_control(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        control: &BrowserControl,
        active: bool,
        focused: bool,
    ) {
        let border = if focused {
            BUTTON_HOT
        } else if active {
            BUTTON_HOT
        } else {
            0x00_96_A8_B4
        };
        match control {
            BrowserControl::TextInput { label, value, .. } => {
                self.fill_rect(stride, x, y, w, CONTROL_H, WHITE);
                self.draw_rect(stride, x, y, w, CONTROL_H, border);
                let shown = if value.is_empty() { label } else { value };
                if !shown.is_empty() {
                    let mut text = shown.clone();
                    truncate_chars(&mut text, w.saturating_sub(14) / CHAR_W);
                    if focused && text.len() < w.saturating_sub(14) / CHAR_W {
                        text.push('_');
                    }
                    self.put_str(
                        stride,
                        x + 7,
                        y + 8,
                        &text,
                        if value.is_empty() { MUTED } else { TEXT },
                    );
                }
            }
            BrowserControl::Button { label } => {
                self.fill_rect(stride, x, y, w, CONTROL_H, 0x00_E8_E8_E8);
                self.draw_rect(
                    stride,
                    x,
                    y,
                    w,
                    CONTROL_H,
                    if focused || active {
                        BUTTON_HOT
                    } else {
                        0x00_88_88_88
                    },
                );
                let mut text = label.clone();
                truncate_chars(&mut text, w.saturating_sub(12) / CHAR_W);
                let text_w = text.len().saturating_mul(CHAR_W);
                let tx = x + w.saturating_sub(text_w) / 2;
                self.put_str(stride, tx, y + 8, &text, TEXT);
            }
            BrowserControl::Checkbox { label, checked } => {
                self.fill_rect(stride, x, y + 5, 12, 12, WHITE);
                self.draw_rect(stride, x, y + 5, 12, 12, border);
                if *checked {
                    self.put_str(stride, x + 2, y + 6, "x", TEXT);
                }
                let mut text = label.clone();
                truncate_chars(&mut text, w.saturating_sub(18) / CHAR_W);
                self.put_str(stride, x + 18, y + 6, &text, TEXT);
            }
            BrowserControl::Radio { label, checked } => {
                self.fill_rect(stride, x, y + 5, 12, 12, WHITE);
                self.draw_rect(stride, x, y + 5, 12, 12, border);
                if *checked {
                    self.fill_rect(stride, x + 4, y + 9, 4, 4, TEXT);
                }
                let mut text = label.clone();
                truncate_chars(&mut text, w.saturating_sub(18) / CHAR_W);
                self.put_str(stride, x + 18, y + 6, &text, TEXT);
            }
            BrowserControl::Select {
                label,
                value,
                options,
            } => {
                self.fill_rect(stride, x, y, w, CONTROL_H, WHITE);
                self.draw_rect(stride, x, y, w, CONTROL_H, border);
                let mut text = if !value.is_empty() {
                    format!("{}: {}", label, value)
                } else if *options > 0 {
                    format!("{} ({} options)", label, options)
                } else {
                    label.clone()
                };
                truncate_chars(&mut text, w.saturating_sub(28) / CHAR_W);
                self.put_str(stride, x + 7, y + 8, &text, TEXT);
                self.put_str(stride, x + w.saturating_sub(18), y + 8, "v", MUTED);
            }
            BrowserControl::TextArea { label, value, rows } => {
                let h = CONTROL_H.saturating_add(rows.saturating_sub(1).min(5) * 10);
                self.fill_rect(stride, x, y, w, h, WHITE);
                self.draw_rect(stride, x, y, w, h, border);
                let mut text = if value.is_empty() {
                    label.clone()
                } else {
                    value.clone()
                };
                truncate_chars(&mut text, w.saturating_sub(14) / CHAR_W);
                self.put_str(
                    stride,
                    x + 7,
                    y + 8,
                    &text,
                    if value.is_empty() { MUTED } else { TEXT },
                );
            }
            BrowserControl::None => {}
        }
    }

    fn fill_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let width = self.window.width.max(0) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
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

    fn draw_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(stride, x, y, w, 1, color);
        self.fill_rect(stride, x, y + h - 1, w, 1, color);
        self.fill_rect(stride, x, y, 1, h, color);
        self.fill_rect(stride, x + w - 1, y, 1, h, color);
    }

    fn draw_box_decoration(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        style: BrowserLineStyle,
    ) {
        if w == 0 || h == 0 || !style.box_style.has_decoration(style.background) {
            return;
        }
        if let Some(bg) = style.background {
            self.fill_rect(stride, x, y, w, h, bg);
        }
        let border_w = style.box_style.border_width.min(8);
        let color = style.box_style.border_color.unwrap_or(BORDER);
        for inset in 0..border_w {
            if w <= inset.saturating_mul(2) || h <= inset.saturating_mul(2) {
                break;
            }
            self.draw_rect(
                stride,
                x + inset,
                y + inset,
                w.saturating_sub(inset * 2),
                h.saturating_sub(inset * 2),
                color,
            );
        }
    }

    fn draw_image_preview(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        max_w: usize,
        max_h: usize,
        image: &crate::png::PngImage,
    ) -> usize {
        draw_image_preview_pixels(
            &mut self.window.buf,
            self.window.width.max(0) as usize,
            (self.window.height - TITLE_H).max(0) as usize,
            stride,
            x,
            y,
            max_w,
            max_h,
            image,
            true,
        )
    }

    fn put_str(&mut self, stride: usize, px: usize, py: usize, s: &str, color: u32) {
        crate::font::draw_str(
            &mut self.window.buf,
            stride,
            px,
            py,
            s,
            color,
            None,
            crate::font::UI_FONT,
        );
    }
}
