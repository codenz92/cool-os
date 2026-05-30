fn welcome_lines() -> Vec<BrowserLine> {
    vec![
        kind_line("coolOS Browser", BrowserLineKind::Heading),
        line(""),
        kind_line("Quick links", BrowserLineKind::Muted),
        line(""),
        link_line("Example Domain", "https://example.com/"),
        link_line("History", "browser://history"),
        link_line("Bookmarks", "browser://bookmarks"),
        link_line("Downloads", "browser://downloads"),
        link_line("Session state", SESSION_INTERNAL_URL),
        link_line("Cache state", CACHE_INTERNAL_URL),
        link_line("Script diagnostics", JS_INTERNAL_URL),
        link_line("Web storage", STORAGE_INTERNAL_URL),
        link_line("Compatibility", COMPAT_INTERNAL_URL),
        link_line("Engine port", ENGINE_INTERNAL_URL),
    ]
}

fn browser_engine_lines() -> Vec<BrowserLine> {
    crate::browser_engine::browser_page_lines()
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            if idx == 0 {
                kind_line(&text, BrowserLineKind::Heading)
            } else if text.is_empty() {
                BrowserLine::new(String::new(), None, BrowserLineKind::Text)
            } else if idx < 5
                || text.starts_with("engine-port")
                || text.starts_with("goal=")
                || text.starts_with("requirements")
                || text.starts_with("readiness=")
                || text.starts_with("coolOS browser engine port manifest")
            {
                kind_line(&text, BrowserLineKind::Muted)
            } else {
                line(&text)
            }
        })
        .collect()
}

fn browser_session_lines() -> Vec<BrowserLine> {
    crate::browser_session::lines()
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            if idx == 0 {
                kind_line(&text, BrowserLineKind::Heading)
            } else if text.is_empty() {
                BrowserLine::new(String::new(), None, BrowserLineKind::Text)
            } else if text.starts_with("Cookie jar:")
                || text.starts_with("Storage:")
                || text == "No cookies stored."
            {
                kind_line(&text, BrowserLineKind::Muted)
            } else {
                line(&text)
            }
        })
        .collect()
}

fn browser_storage_lines() -> Vec<BrowserLine> {
    crate::browser_storage::lines()
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            if idx == 0 {
                kind_line(&text, BrowserLineKind::Heading)
            } else if text.is_empty() {
                BrowserLine::new(String::new(), None, BrowserLineKind::Text)
            } else if text.starts_with("localStorage:")
                || text.starts_with("Storage:")
                || text.starts_with("sessionStorage")
                || text == "No localStorage entries stored."
            {
                kind_line(&text, BrowserLineKind::Muted)
            } else {
                line(&text)
            }
        })
        .collect()
}

fn load_bookmarks() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(bytes) = crate::config_store::read(BOOKMARKS_PATH) {
        if let Ok(text) = core::str::from_utf8(&bytes) {
            for line in text.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                if key.trim() != "bookmark" {
                    continue;
                }
                let url = value.trim();
                if (url.starts_with("http://") || url.starts_with("https://"))
                    && !out.iter().any(|existing| existing == url)
                    && out.len() < MAX_BOOKMARKS
                {
                    out.push(String::from(url));
                }
            }
        }
    }
    if out.is_empty() {
        out.push(String::from("https://example.com/"));
    }
    out
}

fn save_bookmarks(bookmarks: &[String]) {
    let mut out = String::new();
    for bookmark in bookmarks.iter().take(MAX_BOOKMARKS) {
        if !(bookmark.starts_with("http://") || bookmark.starts_with("https://")) {
            continue;
        }
        out.push_str("bookmark=");
        out.push_str(bookmark);
        out.push('\n');
    }
    let _ = crate::config_store::safe_write(BOOKMARKS_PATH, out.as_bytes());
}

fn history_lines(history: &[String]) -> Vec<BrowserLine> {
    let mut out = vec![kind_line("History", BrowserLineKind::Heading), line("")];
    if history.is_empty() {
        out.push(kind_line("No pages visited yet.", BrowserLineKind::Muted));
        return out;
    }
    out.push(kind_line("Recently visited", BrowserLineKind::Muted));
    for url in history.iter().rev().take(32) {
        out.push(link_line(url, url));
    }
    out
}

fn bookmark_lines(bookmarks: &[String]) -> Vec<BrowserLine> {
    let mut out = vec![kind_line("Bookmarks", BrowserLineKind::Heading), line("")];
    if bookmarks.is_empty() {
        out.push(kind_line("No bookmarks yet.", BrowserLineKind::Muted));
        return out;
    }
    out.push(kind_line("Saved pages", BrowserLineKind::Muted));
    for url in bookmarks {
        out.push(link_line(url, url));
    }
    out
}

fn downloads_lines() -> Vec<BrowserLine> {
    let _ = crate::vfs::vfs_create_dir(DOWNLOADS_DIR);
    let mut out = vec![
        kind_line("Downloads", BrowserLineKind::Heading),
        line(""),
        link_line(
            "Open Downloads in File Manager",
            &file_url_for_path(DOWNLOADS_DIR),
        ),
        line(""),
    ];
    let Some(mut entries) = crate::vfs::vfs_list_dir(DOWNLOADS_DIR) else {
        out.push(kind_line(
            "Downloads folder unavailable.",
            BrowserLineKind::Error,
        ));
        return out;
    };
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    if entries.is_empty() {
        out.push(kind_line(
            "No downloaded files yet.",
            BrowserLineKind::Muted,
        ));
        return out;
    }
    let file_count = entries.iter().filter(|entry| !entry.is_dir).count();
    let total_bytes = entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .fold(0usize, |total, entry| {
            total.saturating_add(entry.size as usize)
        });
    out.push(kind_line(
        &format!("{} file(s), {} bytes", file_count, total_bytes),
        BrowserLineKind::Muted,
    ));
    out.push(kind_line("Files", BrowserLineKind::Muted));
    for entry in entries.into_iter().take(48) {
        let mut path = String::from(DOWNLOADS_DIR);
        path.push('/');
        path.push_str(&entry.name);
        let label = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            format!("{}  {} bytes", entry.name, entry.size)
        };
        out.push(link_line(&label, &file_url_for_path(&path)));
    }
    out
}

fn image_response_lines(
    url: &str,
    content_type: Option<&str>,
    byte_len: usize,
    preview_status: Option<&str>,
) -> Vec<BrowserLine> {
    let mut out = vec![
        kind_line("Image", BrowserLineKind::Heading),
        kind_line(content_type.unwrap_or("image/*"), BrowserLineKind::Muted),
        line(""),
    ];
    if let Some(status) = preview_status {
        out.push(kind_line(status, BrowserLineKind::Muted));
    }
    out.push(BrowserLine::new(
        format!("{} bytes received", byte_len),
        None,
        BrowserLineKind::Image,
    ));
    out.push(link_line("Image source URL", url));
    out
}

fn is_success_status(status_line: &str) -> bool {
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.as_bytes().first().copied())
        == Some(b'2')
}

fn is_image_content(content_type: Option<&str>) -> bool {
    content_type
        .map(|value| value.trim().to_ascii_lowercase().starts_with("image/"))
        .unwrap_or(false)
}

fn is_html_main_content(content_type: Option<&str>, url: &str, bytes: &[u8]) -> bool {
    if let Some(value) = content_type.and_then(|value| value.split(';').next()) {
        let value = value.trim();
        if value.eq_ignore_ascii_case("text/html")
            || value.eq_ignore_ascii_case("application/xhtml+xml")
        {
            return true;
        }
        if value.eq_ignore_ascii_case("text/plain") {
            return looks_like_html_bytes(bytes);
        }
        return false;
    }
    extension_from_path(url).eq_ignore_ascii_case("html") || looks_like_html_bytes(bytes)
}

fn source_title_for_content(content_type: Option<&str>, url: &str) -> String {
    let mime = content_type
        .and_then(|value| value.split(';').next())
        .unwrap_or("")
        .trim();
    if is_main_script_content(content_type, url) {
        String::from("JavaScript Source")
    } else if mime.eq_ignore_ascii_case("text/css") || extension_from_path(url) == "css" {
        String::from("CSS Source")
    } else if mime.eq_ignore_ascii_case("application/json") || extension_from_path(url) == "json" {
        String::from("JSON Source")
    } else if mime.starts_with("text/") {
        String::from("Text Source")
    } else {
        String::from("Resource")
    }
}

fn source_response_lines(
    url: &str,
    content_type: Option<&str>,
    body: &str,
    byte_len: usize,
    cols: usize,
) -> Vec<BrowserLine> {
    let title = source_title_for_content(content_type, url);
    let mut out = vec![
        kind_line(&title, BrowserLineKind::Heading),
        kind_line(
            content_type.unwrap_or("application/octet-stream"),
            BrowserLineKind::Muted,
        ),
        kind_line(
            &format!("{} bytes received", byte_len),
            BrowserLineKind::Muted,
        ),
        link_line("Resource URL", url),
        line(""),
        kind_line("Source preview", BrowserLineKind::Muted),
    ];
    let mut shown = 0usize;
    for raw in body.lines() {
        if shown >= 28 {
            out.push(kind_line(
                "... preview truncated ...",
                BrowserLineKind::Muted,
            ));
            break;
        }
        let mut line_text = clean_inline_text(raw);
        if line_text.len() > cols {
            line_text = truncate_text_for_source(&line_text, cols);
        }
        out.push(kind_line(&line_text, BrowserLineKind::Code));
        shown += 1;
    }
    if shown == 0 {
        out.push(kind_line("(empty resource)", BrowserLineKind::Muted));
    }
    out
}

fn truncate_text_for_source(input: &str, max_len: usize) -> String {
    let mut out = String::new();
    for c in input.chars() {
        if out.len().saturating_add(c.len_utf8()).saturating_add(3) > max_len {
            out.push_str("...");
            return out;
        }
        out.push(c);
    }
    out
}

fn google_search_compat_document(base_url: &str, body: &str) -> Option<String> {
    let Ok((_scheme, host, path)) = parse_web_url(base_url) else {
        return None;
    };
    if !is_google_host(&host) {
        return None;
    }
    let path_only = path_without_query_fragment(&path);
    if !matches!(
        path_only.as_str(),
        "/" | "/webhp" | "/search" | "/imghp" | "/advanced_search"
    ) {
        return None;
    }
    let lower = lowercase_ascii(body);
    let looks_like_google = lower.contains("<title>google")
        || lower.contains("name=\"q\"")
        || lower.contains("name='q'")
        || lower.contains("name=q")
        || lower.contains("closure library authors")
        || lower.contains("this.gbar_");
    if !looks_like_google {
        return None;
    }
    Some(build_google_search_compat_document(
        google_query_from_url(base_url).as_deref().unwrap_or(""),
        path_only == "/search",
    ))
}

fn build_google_search_compat_document(query: &str, is_results_url: bool) -> String {
    let query_value = escape_html(query);
    let mut out = String::from(
        "<!doctype html><html><head><title>Google</title><style>\
body{font-family:sans-serif;background:#fff;color:#202124;text-align:center}\
.logo{font-size:48px;margin-top:36px;margin-bottom:18px;color:#4285f4}\
form{margin:0 auto 16px auto;width:70%;padding:10px;border:1px solid #dadce0;background:#fff}\
input{margin:4px;padding:6px;border:1px solid #dadce0}\
.note{color:#5f6368;font-size:12px}\
</style></head><body><h1 class=\"logo\">Google</h1>",
    );
    out.push_str("<form action=\"https://www.google.com/search\" method=\"get\">");
    out.push_str("<input type=\"search\" name=\"q\" value=\"");
    out.push_str(&query_value);
    out.push_str("\" placeholder=\"Search Google\">");
    out.push_str("<input type=\"submit\" name=\"btnG\" value=\"Google Search\">");
    out.push_str("</form>");
    if is_results_url && !query.is_empty() {
        out.push_str("<p>Search: ");
        out.push_str(&query_value);
        out.push_str("</p>");
        out.push_str(
            "<p class=\"note\">Results pages need a modern JavaScript and layout engine; \
this shell keeps search submission usable.</p>",
        );
    } else {
        out.push_str(
            "<p class=\"note\">Compatibility mode keeps the Google search form usable while \
coolOS grows a fuller browser engine.</p>",
        );
    }
    out.push_str("<p><a href=\"browser://compat\">Compatibility diagnostics</a></p>");
    out.push_str("</body></html>");
    out
}

fn is_google_host(host: &str) -> bool {
    let host = lowercase_ascii(host);
    host.starts_with("google.") || host.contains(".google.")
}

fn path_without_query_fragment(path: &str) -> String {
    let end = path
        .find('?')
        .or_else(|| path.find('#'))
        .unwrap_or(path.len());
    String::from(&path[..end])
}

fn google_query_from_url(url: &str) -> Option<String> {
    query_param_from_url(url, "q")
}

fn query_param_from_url(url: &str, wanted: &str) -> Option<String> {
    let start = url.find('?')?.saturating_add(1);
    let end = url[start..]
        .find('#')
        .map(|rel| start + rel)
        .unwrap_or(url.len());
    for pair in url[start..end].split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if decode_query(key) == wanted {
            return Some(decode_query(value));
        }
    }
    None
}

fn escape_html(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn is_known_image_path(path: &str) -> bool {
    matches!(extension_from_path(path), "png" | "jpg" | "gif" | "webp")
}

fn looks_like_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
}

fn image_content_type_for(path: &str, bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") || extension_from_path(path) == "png" {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8") || extension_from_path(path) == "jpg" {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || extension_from_path(path) == "gif"
    {
        Some("image/gif")
    } else if (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
        || extension_from_path(path) == "webp"
    {
        Some("image/webp")
    } else {
        None
    }
}

fn is_png_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("image/png")
        })
        .unwrap_or_else(|| extension_from_path(url).eq_ignore_ascii_case("png"))
}

fn is_html_path(path: &str) -> bool {
    matches!(extension_from_path(path), "html")
}

fn looks_like_html_bytes(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(512);
    let sample = String::from_utf8_lossy(&bytes[..sample_len]);
    let lower = lowercase_ascii(&sample);
    lower.contains("<html") || lower.contains("<!doctype html") || lower.contains("<body")
}

fn load_document_stylesheets(
    base_url: &str,
    body: &str,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    for url in stylesheet_urls(base_url, body) {
        match fetch_subresource_with_cache(
            &url,
            BrowserResourceKind::Stylesheet,
            cache,
            stats,
            bypass_cache,
        ) {
            Ok(resource) => {
                if !is_stylesheet_content(resource.content_type.as_deref(), &resource.final_url) {
                    stats.stylesheets_failed = stats.stylesheets_failed.saturating_add(1);
                    continue;
                }
                let css = String::from_utf8_lossy(&resource.bytes).into_owned();
                if css.trim().is_empty() {
                    stats.stylesheets_failed = stats.stylesheets_failed.saturating_add(1);
                    continue;
                }
                stats.stylesheets_loaded = stats.stylesheets_loaded.saturating_add(1);
                out.push(css);
                if out.len() >= MAX_STYLESHEET_SUBRESOURCES {
                    break;
                }
            }
            Err(_) => {
                stats.stylesheets_failed = stats.stylesheets_failed.saturating_add(1);
            }
        }
    }
    out
}

fn load_document_scripts(
    base_url: &str,
    body: &str,
    cache: &mut BrowserSubresourceCache,
    subresource_stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> BrowserScriptBundle {
    let mut bundle = BrowserScriptBundle {
        sources: Vec::new(),
        stats: BrowserScriptStats::default(),
    };
    let lower = lowercase_ascii(body);
    let mut i = 0usize;
    while bundle.sources.len() < MAX_SCRIPT_SUBRESOURCES {
        let Some(rel) = lower[i..].find("<script") else {
            break;
        };
        let start = i + rel;
        let Some(tag_end_rel) = find_tag_end(&body[start..]) else {
            break;
        };
        let tag_end = start + tag_end_rel;
        let tag = &body[start + 1..tag_end];
        let content_start = tag_end + 1;
        let close_rel = lower[content_start..].find("</script");
        let content_end = close_rel
            .map(|rel| content_start + rel)
            .unwrap_or(content_start);
        let next_i = close_rel
            .and_then(|rel| {
                let close_start = content_start + rel;
                find_tag_end(&body[close_start..]).map(|close_end| close_start + close_end + 1)
            })
            .unwrap_or(content_end);

        if script_type_is_executable(tag) {
            if let Some(src) = attr_value(tag, "src") {
                let src = decode_entities(src.trim());
                let url = resolve_url(base_url, &src);
                if script_url_allowed(base_url, &url) {
                    match fetch_subresource_with_cache(
                        &url,
                        BrowserResourceKind::Script,
                        cache,
                        subresource_stats,
                        bypass_cache,
                    ) {
                        Ok(resource) => {
                            if is_script_content(
                                resource.content_type.as_deref(),
                                &resource.final_url,
                            ) && resource.bytes.len() <= MAX_SCRIPT_BYTES
                            {
                                bundle.stats.external_scripts =
                                    bundle.stats.external_scripts.saturating_add(1);
                                bundle
                                    .sources
                                    .push(String::from_utf8_lossy(&resource.bytes).into_owned());
                            } else {
                                bundle.stats.external_failed =
                                    bundle.stats.external_failed.saturating_add(1);
                            }
                        }
                        Err(_) => {
                            bundle.stats.external_failed =
                                bundle.stats.external_failed.saturating_add(1);
                        }
                    }
                } else {
                    bundle.stats.external_failed = bundle.stats.external_failed.saturating_add(1);
                }
            } else if content_end > content_start {
                let script = &body[content_start..content_end];
                if script.len() <= MAX_SCRIPT_BYTES {
                    bundle.stats.inline_scripts = bundle.stats.inline_scripts.saturating_add(1);
                    bundle.sources.push(String::from(script));
                } else {
                    bundle.stats.external_failed = bundle.stats.external_failed.saturating_add(1);
                }
            }
        }
        i = next_i;
    }
    bundle
}

fn stylesheet_urls(base_url: &str, body: &str) -> Vec<String> {
    let lower = lowercase_ascii(body);
    let mut out = Vec::new();
    let mut i = 0usize;
    while out.len() < MAX_STYLESHEET_SUBRESOURCES {
        let Some(rel) = lower[i..].find("<link") else {
            break;
        };
        let start = i + rel;
        let Some(end_rel) = find_tag_end(&body[start..]) else {
            break;
        };
        let tag = &body[start + 1..start + end_rel];
        let lower_tag = lowercase_ascii(tag.trim());
        if tag_name_of(&lower_tag) == "link"
            && link_rel_includes_stylesheet(tag)
            && !link_media_is_unsupported(tag)
        {
            if let Some(href) = attr_value(tag, "href") {
                let href = decode_entities(href.trim());
                if !href.is_empty() {
                    let url = resolve_url(base_url, &href);
                    if !out.iter().any(|existing| existing == &url) {
                        out.push(url);
                    }
                }
            }
        }
        i = start + end_rel + 1;
    }
    out
}

fn script_type_is_executable(tag: &str) -> bool {
    let Some(kind) = attr_value(tag, "type").or_else(|| attr_value(tag, "language")) else {
        return true;
    };
    let kind = lowercase_ascii(kind.trim());
    kind.is_empty()
        || kind == "javascript"
        || kind == "text/javascript"
        || kind == "application/javascript"
        || kind == "module"
        || kind == "text/ecmascript"
        || kind == "application/ecmascript"
}

fn script_url_allowed(base_url: &str, url: &str) -> bool {
    if url.starts_with("file://") {
        return base_url.starts_with("file://");
    }
    let Ok((scheme, host, _)) = parse_web_url(url) else {
        return false;
    };
    let Ok((base_scheme, base_host, _)) = parse_web_url(base_url) else {
        return false;
    };
    scheme == base_scheme && lowercase_ascii(&host) == lowercase_ascii(&base_host)
}

fn is_script_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            let value = value.trim();
            value.eq_ignore_ascii_case("application/javascript")
                || value.eq_ignore_ascii_case("text/javascript")
                || value.eq_ignore_ascii_case("application/ecmascript")
                || value.eq_ignore_ascii_case("text/ecmascript")
                || value.eq_ignore_ascii_case("text/plain")
        })
        .unwrap_or_else(|| extension_from_path(url).eq_ignore_ascii_case("js"))
}

fn is_main_script_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            let value = value.trim();
            value.eq_ignore_ascii_case("application/javascript")
                || value.eq_ignore_ascii_case("text/javascript")
                || value.eq_ignore_ascii_case("application/ecmascript")
                || value.eq_ignore_ascii_case("text/ecmascript")
                || value.eq_ignore_ascii_case("application/x-javascript")
        })
        .unwrap_or_else(|| {
            let ext = extension_from_path(url);
            ext.eq_ignore_ascii_case("js") || ext.eq_ignore_ascii_case("mjs")
        })
}

fn link_rel_includes_stylesheet(tag: &str) -> bool {
    attr_value(tag, "rel")
        .map(|rel| {
            lowercase_ascii(&rel)
                .split_whitespace()
                .any(|part| part == "stylesheet")
        })
        .unwrap_or(false)
}

fn link_media_is_unsupported(tag: &str) -> bool {
    let Some(media) = attr_value(tag, "media") else {
        return false;
    };
    let media = lowercase_ascii(&media);
    !(media.trim().is_empty()
        || media
            .split(',')
            .any(|part| matches!(part.trim(), "all" | "screen")))
}

fn is_stylesheet_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            let value = value.trim();
            value.eq_ignore_ascii_case("text/css") || value.eq_ignore_ascii_case("text/plain")
        })
        .unwrap_or_else(|| extension_from_path(url).eq_ignore_ascii_case("css"))
}

fn attach_html_images_with_cache(
    lines: &mut Vec<BrowserLine>,
    inline_images: &mut Vec<InlineImage>,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    cols: usize,
    bypass_cache: bool,
) -> usize {
    inline_images.clear();
    let mut idx = 0usize;
    while idx < lines.len() && inline_images.len() < MAX_HTML_INLINE_IMAGES {
        let should_try = lines
            .get(idx)
            .map(|line| line.kind == BrowserLineKind::Image && line.link.is_some())
            .unwrap_or(false);
        if !should_try {
            idx += 1;
            continue;
        }
        let Some(url) = lines[idx].link.clone() else {
            idx += 1;
            continue;
        };
        let alt = image_alt_from_line(&lines[idx].text);
        match fetch_image_for_browser(&url, cache, stats, bypass_cache) {
            Ok(BrowserFetchedImage::Png {
                image,
                source_url,
                byte_len,
                cache_hit,
            }) => {
                let slot = inline_images.len();
                let rows = inline_image_reserved_rows_for(image.width, image.height, cols);
                lines[idx].image_slot = Some(slot);
                lines[idx].text = format!(
                    "[image] {}  {}x{}  {} bytes{}",
                    alt,
                    image.width,
                    image.height,
                    byte_len,
                    if cache_hit { " cached" } else { "" }
                );
                lines[idx].link = Some(source_url);
                inline_images.push(InlineImage { image });
                for _ in 1..rows {
                    lines.insert(idx + 1, inline_image_spacer(slot, &url));
                }
                idx += rows;
            }
            Ok(BrowserFetchedImage::Placeholder {
                label,
                source_url,
                byte_len,
                cache_hit,
            }) => {
                lines[idx].text = format!(
                    "[image] {}  {}  {} bytes  preview unavailable{}",
                    alt,
                    label,
                    byte_len,
                    if cache_hit { " cached" } else { "" }
                );
                lines[idx].link = Some(source_url);
                idx += 1;
            }
            Err(err) => {
                lines[idx].text = format!("{} ({})", lines[idx].text, err);
                idx += 1;
            }
        }
    }
    inline_images.len()
}
