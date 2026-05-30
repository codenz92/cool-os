fn response_body_text(response: &str) -> Option<&str> {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .or_else(|| response.split_once("\n\n").map(|(_, body)| body))
}

fn file_url_for_path(path: &str) -> String {
    let mut out = String::from("file://");
    out.push_str(path);
    out
}

fn download_filename(url: &str, content_type: Option<&str>, source: bool) -> String {
    let (_scheme, host, path) = parse_web_url(url).unwrap_or_else(|_| {
        (
            String::from("web"),
            String::from("download"),
            String::from("/index"),
        )
    });
    let ext = if source {
        "html"
    } else {
        extension_for_content_type(content_type).unwrap_or_else(|| extension_from_path(&path))
    };
    let mut stem = String::new();
    stem.push_str(&sanitize_filename_part(&host));
    let leaf = path
        .split('?')
        .next()
        .unwrap_or("/")
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or("index");
    stem.push('-');
    stem.push_str(&sanitize_filename_part(leaf));
    if stem.len() > 72 {
        stem.truncate(72);
    }
    if stem.ends_with('-') {
        stem.push_str("page");
    }
    stem.push('.');
    stem.push_str(ext);
    stem
}

fn extension_for_content_type(content_type: Option<&str>) -> Option<&'static str> {
    let value = content_type?.split(';').next()?.trim();
    if value.eq_ignore_ascii_case("text/html") {
        Some("html")
    } else if value.eq_ignore_ascii_case("text/css") {
        Some("css")
    } else if value.eq_ignore_ascii_case("application/javascript")
        || value.eq_ignore_ascii_case("text/javascript")
        || value.eq_ignore_ascii_case("application/ecmascript")
        || value.eq_ignore_ascii_case("text/ecmascript")
    {
        Some("js")
    } else if value.eq_ignore_ascii_case("text/plain") {
        Some("txt")
    } else if value.eq_ignore_ascii_case("image/png") {
        Some("png")
    } else if value.eq_ignore_ascii_case("image/jpeg") || value.eq_ignore_ascii_case("image/jpg") {
        Some("jpg")
    } else if value.eq_ignore_ascii_case("image/gif") {
        Some("gif")
    } else if value.eq_ignore_ascii_case("image/webp") {
        Some("webp")
    } else {
        None
    }
}

fn extension_from_path(path: &str) -> &'static str {
    let leaf = path.split('?').next().unwrap_or(path);
    let lower = lowercase_ascii(leaf);
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "html"
    } else if lower.ends_with(".css") {
        "css"
    } else if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "js"
    } else if lower.ends_with(".txt") {
        "txt"
    } else if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "jpg"
    } else if lower.ends_with(".gif") {
        "gif"
    } else if lower.ends_with(".webp") {
        "webp"
    } else {
        "bin"
    }
}

fn sanitize_filename_part(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        let b = b.to_ascii_lowercase();
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else if matches!(b, b'.' | b'-' | b'_') {
            out.push(b as char);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.starts_with('.') || out.starts_with('-') {
        out.remove(0);
    }
    while out.ends_with('.') || out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        String::from("download")
    } else {
        out
    }
}

fn line(text: &str) -> BrowserLine {
    BrowserLine::new(String::from(text), None, BrowserLineKind::Text)
}

fn kind_line(text: &str, kind: BrowserLineKind) -> BrowserLine {
    BrowserLine::new(String::from(text), None, kind)
}

fn link_line(text: &str, url: &str) -> BrowserLine {
    BrowserLine::new(
        String::from(text),
        Some(String::from(url)),
        BrowserLineKind::Link,
    )
}

fn network_error_lines(url: &str, host: &str, path: &str, err: &str) -> Vec<BrowserLine> {
    let mut lines = vec![
        kind_line("Unable to load page", BrowserLineKind::Error),
        line(url),
        kind_line(err, BrowserLineKind::Muted),
        line(""),
    ];
    if err.contains("timeout") {
        lines.push(kind_line(
            "The connection timed out before the page finished loading.",
            BrowserLineKind::Muted,
        ));
    } else if err.contains("certificate") || err.contains("hostname") {
        lines.push(kind_line(
            "The TLS certificate could not be verified for this host.",
            BrowserLineKind::Muted,
        ));
    } else if err.contains("DNS") {
        lines.push(kind_line(
            "The hostname did not resolve through the configured DNS server.",
            BrowserLineKind::Muted,
        ));
    }
    lines.push(link_line("Retry", url));
    if path != "/" {
        let mut origin = if url.starts_with("http://") {
            String::from("http://")
        } else {
            String::from("https://")
        };
        origin.push_str(host);
        origin.push('/');
        lines.push(link_line("Open site root", &origin));
    }
    lines.push(link_line(
        "Open known-good HTTPS page",
        "https://example.com/",
    ));
    lines
}

fn normalize_address_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("browser://")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        String::from(trimmed)
    } else if looks_like_url(trimmed) {
        let mut out = String::from("https://");
        out.push_str(trimmed);
        out
    } else {
        let mut out = String::from("browser://search?q=");
        push_query_encoded(&mut out, trimmed);
        out
    }
}

fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("browser://")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        String::from(trimmed)
    } else {
        let mut out = String::from("http://");
        out.push_str(trimmed);
        out
    }
}

fn looks_like_url(input: &str) -> bool {
    input.contains('.')
        || input.starts_with("localhost")
        || input.starts_with("10.")
        || input.starts_with("172.")
        || input.starts_with("192.168.")
}

fn push_query_encoded(out: &mut String, input: &str) {
    for b in input.bytes() {
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => out.push('+'),
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'_' => out.push(b as char),
            _ => {
                out.push('%');
                out.push(hex_digit((b >> 4) & 0x0f));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
}

fn decode_query(input: &str) -> String {
    let mut out = String::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push(((hi << 4) | lo) as char);
                i += 3;
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn browser_event_url(label: &str) -> String {
    let mut out = String::from("browser://event?label=");
    push_query_encoded(&mut out, label);
    out
}

fn browser_event_label(url: &str) -> Option<String> {
    let query = url.strip_prefix("browser://event?label=")?;
    Some(decode_query(query))
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'A' + value - 10) as char,
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn parse_web_url(url: &str) -> Result<(String, String, String), &'static str> {
    if let Some(rest) = url.strip_prefix("http://") {
        let (host, path) = parse_web_host_path("http", rest)?;
        return Ok((String::from("http"), host, path));
    }
    if let Some(rest) = url.strip_prefix("https://") {
        let (host, path) = parse_web_host_path("https", rest)?;
        return Ok((String::from("https"), host, path));
    }
    Err("URL must start with http:// or https://")
}

fn parse_web_host_path(scheme: &str, rest: &str) -> Result<(String, String), &'static str> {
    let slash = rest.find('/').unwrap_or(rest.len());
    let mut host = rest[..slash].trim();
    if host.is_empty() {
        return Err("missing host");
    }
    if let Some((name, port)) = host.rsplit_once(':') {
        let expected_port = if scheme == "https" { "443" } else { "80" };
        if port != expected_port {
            return Err("only default web ports are supported");
        }
        host = name;
    }
    let path = if slash < rest.len() {
        &rest[slash..]
    } else {
        "/"
    };
    Ok((String::from(host), String::from(path)))
}

fn extract_base_href(body: &str, fallback: &str) -> String {
    let head_end = {
        let lower = lowercase_ascii(body);
        lower.find("</head>").unwrap_or(body.len().min(8192))
    };
    let search = &body[..head_end];
    let lower_search = lowercase_ascii(search);
    let mut i = 0usize;
    while let Some(rel) = lower_search[i..].find("<base") {
        let abs = i + rel;
        if let Some(end) = lower_search[abs..].find('>') {
            let tag = &body[abs + 1..abs + end];
            if let Some(href) = attr_value(tag, "href") {
                let href = String::from(href.trim());
                if !href.is_empty() {
                    return href;
                }
            }
            i = abs + end + 1;
        } else {
            break;
        }
    }
    String::from(fallback)
}
