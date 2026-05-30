pub fn http_get(host: &str, path: &str) -> Result<String, &'static str> {
    http_get_response(host, path).map(|response| response.request)
}

pub fn web_get_response(url: &str) -> Result<HttpResponse, &'static str> {
    let (scheme, host, path) = parse_web_url(url)?;
    let mut session = HttpRequestSession::disabled();
    http_request_response_follow(
        &scheme,
        &host,
        &path,
        HttpRequestMethod::Get,
        &[],
        None,
        0,
        &mut session,
    )
}

pub fn browser_get_response(url: &str) -> Result<HttpResponse, &'static str> {
    let (scheme, host, path) = parse_web_url(url)?;
    let mut session = HttpRequestSession::enabled();
    http_request_response_follow(
        &scheme,
        &host,
        &path,
        HttpRequestMethod::Get,
        &[],
        None,
        0,
        &mut session,
    )
}

pub fn browser_post_response(
    url: &str,
    body: &str,
    content_type: &str,
) -> Result<HttpResponse, &'static str> {
    crate::println!(
        "[http] POST {} body={} content_type={}",
        url,
        body.len(),
        content_type
    );
    let (scheme, host, path) = parse_web_url(url)?;
    let mut session = HttpRequestSession::enabled();
    http_request_response_follow(
        &scheme,
        &host,
        &path,
        HttpRequestMethod::Post,
        body.as_bytes(),
        Some(content_type),
        0,
        &mut session,
    )
}

pub fn http_get_response(host: &str, path: &str) -> Result<HttpResponse, &'static str> {
    let mut session = HttpRequestSession::disabled();
    http_request_response_follow(
        "http",
        host,
        path,
        HttpRequestMethod::Get,
        &[],
        None,
        0,
        &mut session,
    )
}

pub fn http_request_debug_for_test(
    method: &str,
    url: &str,
    body: &str,
    content_type: &str,
) -> Vec<String> {
    let method = if method.eq_ignore_ascii_case("POST") {
        HttpRequestMethod::Post
    } else {
        HttpRequestMethod::Get
    };
    let Ok((_scheme, host, path)) = parse_web_url(url) else {
        return vec![String::from("parse failed")];
    };
    let Ok(request) = build_http_request(
        method,
        &host,
        &path,
        body.as_bytes(),
        Some(content_type),
        None,
    ) else {
        return vec![String::from("request failed")];
    };
    request
        .lines()
        .filter(|line| {
            line.starts_with("GET ")
                || line.starts_with("POST ")
                || line.starts_with("Host:")
                || line.starts_with("Cookie:")
                || line.starts_with("Content-Type:")
                || line.starts_with("Content-Length:")
                || *line == body
        })
        .map(String::from)
        .collect()
}

pub fn http_cookie_request_debug_for_test(url: &str, cookie: &str) -> Vec<String> {
    let Ok((_scheme, host, path)) = parse_web_url(url) else {
        return vec![String::from("parse failed")];
    };
    let Ok(request) = build_http_request(
        HttpRequestMethod::Get,
        &host,
        &path,
        &[],
        None,
        Some(cookie),
    ) else {
        return vec![String::from("request failed")];
    };
    request
        .lines()
        .filter(|line| {
            line.starts_with("GET ") || line.starts_with("Host:") || line.starts_with("Cookie:")
        })
        .map(String::from)
        .collect()
}

fn build_http_request(
    method: HttpRequestMethod,
    host: &str,
    path: &str,
    body: &[u8],
    content_type: Option<&str>,
    cookie_header: Option<&str>,
) -> Result<String, &'static str> {
    if method == HttpRequestMethod::Get && !body.is_empty() {
        return Err("GET request body unsupported");
    }
    if body.len() > HTTP_MAX_REQUEST_BODY {
        return Err("HTTP request body too large");
    }
    if let Some(cookie) = cookie_header {
        if cookie.len() > 4096 || cookie.bytes().any(|b| matches!(b, b'\r' | b'\n')) {
            return Err("invalid Cookie header");
        }
    }
    let mut request = String::from(method.as_str());
    request.push(' ');
    request.push_str(path);
    request.push_str(" HTTP/1.1\r\nHost: ");
    request.push_str(host);
    request.push_str("\r\nUser-Agent: coolOS/19\r\nAccept: text/html,text/plain,image/*,*/*\r\nAccept-Encoding: gzip, identity\r\nConnection: close");
    if method == HttpRequestMethod::Post {
        request.push_str("\r\nContent-Type: ");
        request.push_str(content_type.unwrap_or("application/x-www-form-urlencoded"));
        request.push_str("\r\nContent-Length: ");
        push_decimal(&mut request, body.len() as u64);
    }
    if let Some(cookie) = cookie_header {
        if !cookie.trim().is_empty() {
            request.push_str("\r\nCookie: ");
            request.push_str(cookie.trim());
        }
    }
    request.push_str("\r\n\r\n");
    if !body.is_empty() {
        request.push_str(&String::from_utf8_lossy(body));
    }
    Ok(request)
}

fn http_request_response_follow(
    scheme: &str,
    host: &str,
    path: &str,
    method: HttpRequestMethod,
    request_body: &[u8],
    content_type: Option<&str>,
    redirect_count: usize,
    session: &mut HttpRequestSession,
) -> Result<HttpResponse, &'static str> {
    let settings = crate::settings_state::snapshot();
    if !settings.network_http_enabled {
        return Err("HTTP API disabled in Settings");
    }
    let host = host.trim();
    if host.is_empty() || host.len() > 253 || host.contains('/') || host.contains(' ') {
        return Err("invalid host");
    }
    let path = if path.is_empty() { "/" } else { path };

    let cookie_header = if session.enabled {
        crate::browser_session::cookie_header_for_request(scheme, host, path)
    } else {
        None
    };
    let request = build_http_request(
        method,
        host,
        path,
        request_body,
        content_type,
        cookie_header.as_deref(),
    )?;

    if scheme == "https" {
        if !has_link() {
            return Err("HTTPS requires a network adapter");
        }
        let exchange = crate::tls::https_exchange(host, path, &request, HTTP_MAX_BYTES)?;
        crate::println!(
            "[tls-ok] https {}{} via {} verified_root={}",
            host,
            path,
            ipv4_string(exchange.resolved_addr),
            exchange.trust_root
        );
        let body = normalize_http_response_bytes(&exchange.raw_response)?;
        return finish_web_response(
            scheme,
            host,
            path,
            redirect_count,
            exchange.resolved_addr,
            Some(exchange.trust_root),
            request,
            body,
            method,
            request_body,
            content_type,
            session,
        );
    }

    let addrs = resolve_host_addrs(host)?;
    let resolved_addr = *addrs.first().ok_or("DNS returned no address")?;

    if !has_link() {
        if settings.network_offline_api {
            let body = format!(
                "HTTP/1.1 200 OK (offline)\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\ncoolOS offline HTTP {} response from {} at {}\nrequest-body-bytes={}",
                method.as_str(),
                host,
                ipv4_string(resolved_addr),
                request_body.len()
            );
            let body_bytes = response_body_bytes(body.as_bytes())
                .unwrap_or_else(|| body.as_bytes())
                .to_vec();
            return Ok(HttpResponse {
                host: String::from(host),
                path: String::from(path),
                final_url: format!("{}://{}{}", scheme, host, path),
                redirect_count,
                resolved_addr,
                tls_trust_root: None,
                request,
                status_line: String::from("HTTP/1.1 200 OK (offline)"),
                content_type: Some(String::from("text/plain")),
                session_cookies_stored: session.stored_cookies,
                body,
                body_bytes,
            });
        }
        return Err("no network adapter");
    }

    let mut last_err = "HTTP connect failed";
    let mut body_bytes = Vec::new();
    let mut connected_addr = resolved_addr;
    for addr in addrs {
        let socket = socket_open(KERNEL_SOCKET_OWNER, 2, 1, 6)?;
        match socket_connect(KERNEL_SOCKET_OWNER, socket, addr, 80)
            .and_then(|()| socket_send(KERNEL_SOCKET_OWNER, socket, request.as_bytes()).map(|_| ()))
        {
            Ok(()) => {
                connected_addr = addr;
                let mut buf = [0u8; 512];
                loop {
                    let n = socket_recv(KERNEL_SOCKET_OWNER, socket, &mut buf)?;
                    if n == 0 {
                        if socket_peer_closed(KERNEL_SOCKET_OWNER, socket) {
                            break;
                        }
                        let _ = socket_close(KERNEL_SOCKET_OWNER, socket);
                        return Err("HTTP response timeout");
                    }
                    body_bytes.extend_from_slice(&buf[..n]);
                    if body_bytes.len() > HTTP_MAX_BYTES {
                        break;
                    }
                }
                let _ = socket_close(KERNEL_SOCKET_OWNER, socket);
                break;
            }
            Err(err) => {
                last_err = err;
                let _ = socket_close(KERNEL_SOCKET_OWNER, socket);
            }
        }
    }
    if body_bytes.is_empty() {
        return Err(last_err);
    }

    let body = normalize_http_response_bytes(&body_bytes)?;
    finish_web_response(
        scheme,
        host,
        path,
        redirect_count,
        connected_addr,
        None,
        request,
        body,
        method,
        request_body,
        content_type,
        session,
    )
}

fn finish_web_response(
    scheme: &str,
    host: &str,
    path: &str,
    redirect_count: usize,
    connected_addr: u32,
    tls_trust_root: Option<&'static str>,
    request: String,
    body: NormalizedHttpResponse,
    method: HttpRequestMethod,
    request_body: &[u8],
    content_type: Option<&str>,
    session: &mut HttpRequestSession,
) -> Result<HttpResponse, &'static str> {
    let status_line = String::from(
        body.text
            .split('\n')
            .next()
            .map(|line| line.trim_end_matches('\r'))
            .unwrap_or("HTTP response"),
    );
    let response_content_type = http_header_value(&body.text, "content-type");
    let set_cookie_headers = http_header_values(&body.text, "set-cookie");
    if session.enabled && !set_cookie_headers.is_empty() {
        session.stored_cookies += crate::browser_session::store_set_cookie_headers(
            scheme,
            host,
            path,
            &set_cookie_headers,
        );
    }
    let status = http_status_code(&status_line).unwrap_or(0);
    if is_redirect_status(status) {
        if redirect_count >= HTTP_MAX_REDIRECTS {
            return Err("HTTP redirect limit reached");
        }
        let Some(location) = http_header_value(&body.text, "location") else {
            return Err("HTTP redirect missing Location");
        };
        let (next_scheme, next_host, next_path) =
            resolve_web_location(scheme, host, path, &location)?;
        let (next_method, next_body, next_content_type) =
            redirect_request_parts(status, method, request_body, content_type);
        return http_request_response_follow(
            &next_scheme,
            &next_host,
            &next_path,
            next_method,
            next_body,
            next_content_type,
            redirect_count + 1,
            session,
        );
    }

    Ok(HttpResponse {
        host: String::from(host),
        path: String::from(path),
        final_url: format!("{}://{}{}", scheme, host, path),
        redirect_count,
        resolved_addr: connected_addr,
        tls_trust_root,
        request,
        status_line,
        content_type: response_content_type,
        session_cookies_stored: session.stored_cookies,
        body: body.text,
        body_bytes: body.body_bytes,
    })
}

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn redirect_request_parts<'a>(
    status: u16,
    method: HttpRequestMethod,
    body: &'a [u8],
    content_type: Option<&'a str>,
) -> (HttpRequestMethod, &'a [u8], Option<&'a str>) {
    if method == HttpRequestMethod::Get || matches!(status, 307 | 308) {
        (method, body, content_type)
    } else {
        (HttpRequestMethod::Get, &[], None)
    }
}

fn http_status_code(status_line: &str) -> Option<u16> {
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
}

fn http_header_value(response: &str, name: &str) -> Option<String> {
    let header_block = response
        .split_once("\r\n\r\n")
        .map(|(headers, _)| headers)
        .or_else(|| response.split_once("\n\n").map(|(headers, _)| headers))
        .unwrap_or(response);
    for line in header_block.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case(name) {
            return Some(String::from(value.trim()));
        }
    }
    None
}

fn http_header_values(response: &str, name: &str) -> Vec<String> {
    let header_block = response
        .split_once("\r\n\r\n")
        .map(|(headers, _)| headers)
        .or_else(|| response.split_once("\n\n").map(|(headers, _)| headers))
        .unwrap_or(response);
    let mut out = Vec::new();
    for line in header_block.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case(name) {
            out.push(String::from(value.trim()));
        }
    }
    out
}

fn normalize_http_response_bytes(response: &[u8]) -> Result<NormalizedHttpResponse, &'static str> {
    let Some((headers, body, sep)) = split_http_response_bytes(response) else {
        return Ok(NormalizedHttpResponse {
            text: String::from_utf8_lossy(response).into_owned(),
            body_bytes: response.to_vec(),
        });
    };
    let headers_text = String::from_utf8_lossy(headers);
    let mut body_bytes = body.to_vec();
    if http_header_value(&headers_text, "transfer-encoding")
        .map(|value| header_contains_token(&value, "chunked"))
        .unwrap_or(false)
    {
        body_bytes = decode_chunked_body_bytes(&body_bytes)?;
    }
    if let Some(length) = http_header_value(&headers_text, "content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
    {
        if body_bytes.len() >= length {
            body_bytes.truncate(length);
        }
    }
    let decoded_gzip = http_header_value(&headers_text, "content-encoding")
        .map(|value| header_contains_token(&value, "gzip"))
        .unwrap_or(false);
    if decoded_gzip {
        body_bytes = decode_gzip_body(&body_bytes)?;
    }

    let mut out = headers_without_body_framing(&headers_text);
    if decoded_gzip {
        out.push_str("\r\nContent-Encoding: identity");
        out.push_str("\r\nX-coolOS-Decoded-Encoding: gzip");
    }
    out.push_str("\r\nContent-Length: ");
    push_decimal(&mut out, body_bytes.len() as u64);
    out.push_str(sep);
    out.push_str(&String::from_utf8_lossy(&body_bytes));
    Ok(NormalizedHttpResponse {
        text: out,
        body_bytes,
    })
}

fn headers_without_body_framing(headers: &str) -> String {
    let mut out = String::new();
    for line in headers.lines() {
        if let Some((name, _)) = line.split_once(':') {
            let name = name.trim();
            if name.eq_ignore_ascii_case("transfer-encoding")
                || name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("content-encoding")
            {
                continue;
            }
        }
        if !out.is_empty() {
            out.push_str("\r\n");
        }
        out.push_str(line.trim_end_matches('\r'));
    }
    out
}

fn response_body_bytes(response: &[u8]) -> Option<&[u8]> {
    split_http_response_bytes(response).map(|(_, body, _)| body)
}

fn split_http_response_bytes(response: &[u8]) -> Option<(&[u8], &[u8], &'static str)> {
    if let Some(pos) = find_bytes(response, b"\r\n\r\n") {
        return Some((&response[..pos], &response[pos + 4..], "\r\n\r\n"));
    }
    find_bytes(response, b"\n\n").map(|pos| (&response[..pos], &response[pos + 2..], "\n\n"))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn header_contains_token(value: &str, token: &str) -> bool {
    value
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case(token))
}

fn decode_chunked_body_bytes(body: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        if pos >= body.len() {
            return if out.is_empty() {
                Err("bad chunk header")
            } else {
                Ok(out)
            };
        }
        let line_end = find_crlf(body, pos).ok_or("bad chunk header")?;
        let header = core::str::from_utf8(&body[pos..line_end]).map_err(|_| "bad chunk header")?;
        let size_text = header.split(';').next().unwrap_or("").trim();
        let size = parse_hex_usize(size_text).ok_or("bad chunk size")?;
        pos = line_end + crlf_len(body, line_end);
        if size == 0 {
            break;
        }
        if pos + size > body.len() {
            return Err("truncated chunk body");
        }
        out.extend_from_slice(&body[pos..pos + size]);
        pos += size;
        if pos < body.len() {
            if body.get(pos) == Some(&b'\r') && body.get(pos + 1) == Some(&b'\n') {
                pos += 2;
            } else if body.get(pos) == Some(&b'\n') {
                pos += 1;
            } else {
                return Err("bad chunk terminator");
            }
        }
        if out.len() > HTTP_MAX_BYTES {
            return Err("decoded response too large");
        }
    }
    Ok(out)
}

fn decode_gzip_body(body: &[u8]) -> Result<Vec<u8>, &'static str> {
    if body.len() < 18 || body[0] != 0x1f || body[1] != 0x8b || body[2] != 8 {
        return Err("bad gzip response");
    }
    let flags = body[3];
    let mut pos = 10usize;
    if flags & 0x04 != 0 {
        if pos + 2 > body.len() {
            return Err("bad gzip extra");
        }
        let extra_len = u16::from_le_bytes([body[pos], body[pos + 1]]) as usize;
        pos += 2 + extra_len;
    }
    if flags & 0x08 != 0 {
        pos = skip_gzip_cstring(body, pos)?;
    }
    if flags & 0x10 != 0 {
        pos = skip_gzip_cstring(body, pos)?;
    }
    if flags & 0x02 != 0 {
        pos = pos.checked_add(2).ok_or("bad gzip header")?;
    }
    if pos >= body.len().saturating_sub(8) {
        return Err("bad gzip payload");
    }
    let compressed = &body[pos..body.len() - 8];
    miniz_oxide::inflate::decompress_to_vec_with_limit(compressed, HTTP_MAX_BYTES)
        .map_err(|_| "gzip decode failed")
}

fn skip_gzip_cstring(body: &[u8], mut pos: usize) -> Result<usize, &'static str> {
    while pos < body.len() {
        if body[pos] == 0 {
            return Ok(pos + 1);
        }
        pos += 1;
    }
    Err("bad gzip header")
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            return Some(if i > start && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            });
        }
        i += 1;
    }
    None
}

fn crlf_len(bytes: &[u8], line_end: usize) -> usize {
    if bytes.get(line_end) == Some(&b'\r') && bytes.get(line_end + 1) == Some(&b'\n') {
        2
    } else {
        1
    }
}

fn parse_hex_usize(input: &str) -> Option<usize> {
    let mut value = 0usize;
    let mut saw_digit = false;
    for b in input.bytes() {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as usize,
            b'a'..=b'f' => (b - b'a' + 10) as usize,
            b'A'..=b'F' => (b - b'A' + 10) as usize,
            _ => return None,
        };
        value = value.checked_mul(16)?.checked_add(digit)?;
        saw_digit = true;
    }
    saw_digit.then_some(value)
}

fn push_decimal(out: &mut String, mut value: u64) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    loop {
        digits[len] = b'0' + (value % 10) as u8;
        len += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for idx in (0..len).rev() {
        out.push(digits[idx] as char);
    }
}

fn parse_web_url(url: &str) -> Result<(String, String, String), &'static str> {
    if let Some(rest) = url.trim().strip_prefix("http://") {
        let (host, path) = split_web_host_path("http", rest)?;
        return Ok((String::from("http"), host, path));
    }
    if let Some(rest) = url.trim().strip_prefix("https://") {
        let (host, path) = split_web_host_path("https", rest)?;
        return Ok((String::from("https"), host, path));
    }
    Err("URL must start with http:// or https://")
}

fn resolve_web_location(
    base_scheme: &str,
    base_host: &str,
    base_path: &str,
    location: &str,
) -> Result<(String, String, String), &'static str> {
    let location = location.trim();
    if let Some(rest) = location.strip_prefix("https://") {
        let (host, path) = split_web_host_path("https", rest)?;
        return Ok((String::from("https"), host, path));
    }
    if let Some(rest) = location.strip_prefix("http://") {
        let (host, path) = split_web_host_path("http", rest)?;
        return Ok((String::from("http"), host, path));
    }
    if let Some(rest) = location.strip_prefix("//") {
        let (host, path) = split_web_host_path(base_scheme, rest)?;
        return Ok((String::from(base_scheme), host, path));
    }
    if location.starts_with('/') {
        return Ok((
            String::from(base_scheme),
            String::from(base_host),
            String::from(location),
        ));
    }
    let mut dir = String::from(base_path);
    if let Some(pos) = dir.rfind('/') {
        dir.truncate(pos + 1);
    } else {
        dir = String::from("/");
    }
    dir.push_str(location);
    Ok((String::from(base_scheme), String::from(base_host), dir))
}

fn split_web_host_path(scheme: &str, rest: &str) -> Result<(String, String), &'static str> {
    let slash = rest.find('/').unwrap_or(rest.len());
    let host = rest[..slash].trim();
    if host.is_empty() || host.len() > 253 || host.contains(' ') {
        return Err("invalid redirect host");
    }
    if let Some((name, port)) = host.rsplit_once(':') {
        let expected_port = if scheme == "https" { "443" } else { "80" };
        if port != expected_port {
            return Err("web redirect port unsupported");
        }
        if name.is_empty() {
            return Err("invalid redirect host");
        }
        let path = if slash < rest.len() {
            &rest[slash..]
        } else {
            "/"
        };
        return Ok((String::from(name), String::from(path)));
    }
    let path = if slash < rest.len() {
        &rest[slash..]
    } else {
        "/"
    };
    Ok((String::from(host), String::from(path)))
}
