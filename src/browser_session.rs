extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use spin::Mutex;

const COOKIE_PATH: &str = "/CONFIG/BROWSER.COOKIES";
const MAX_COOKIES: usize = 64;
const MAX_COOKIE_NAME: usize = 48;
const MAX_COOKIE_VALUE: usize = 192;
const MAX_COOKIE_DOMAIN: usize = 128;
const MAX_COOKIE_PATH: usize = 160;
const MAX_COOKIE_HEADER: usize = 4096;

#[derive(Clone)]
struct Cookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    host_only: bool,
}

struct CookieJar {
    loaded: bool,
    cookies: Vec<Cookie>,
}

static COOKIE_JAR: Mutex<CookieJar> = Mutex::new(CookieJar {
    loaded: false,
    cookies: Vec::new(),
});

pub fn cookie_header_for_request(scheme: &str, host: &str, path: &str) -> Option<String> {
    let mut jar = COOKIE_JAR.lock();
    jar.ensure_loaded();
    jar.cookie_header_for(scheme, host, path)
}

pub fn store_set_cookie_headers(scheme: &str, host: &str, path: &str, headers: &[String]) -> usize {
    if headers.is_empty() {
        return 0;
    }
    let mut jar = COOKIE_JAR.lock();
    jar.ensure_loaded();
    let mut changed = 0usize;
    for header in headers {
        if jar.store_set_cookie(scheme, host, path, header) {
            changed += 1;
        }
    }
    if changed > 0 {
        jar.save();
    }
    changed
}

pub fn summary_line() -> String {
    let mut jar = COOKIE_JAR.lock();
    jar.ensure_loaded();
    format!("{} cookie(s)", jar.cookies.len())
}

pub fn lines() -> Vec<String> {
    let mut jar = COOKIE_JAR.lock();
    jar.ensure_loaded();
    let mut out = vec![
        String::from("Browser session state"),
        String::from(""),
        format!("Cookie jar: {} cookie(s)", jar.cookies.len()),
        String::from("Storage: /CONFIG/BROWSER.COOKIES"),
        String::from(""),
    ];
    if jar.cookies.is_empty() {
        out.push(String::from("No cookies stored."));
        return out;
    }
    out.push(String::from("Stored cookies"));
    for cookie in jar.cookies.iter().take(MAX_COOKIES) {
        let scope = if cookie.host_only { "host" } else { "domain" };
        let secure = if cookie.secure { " secure" } else { "" };
        out.push(format!(
            "{}  {}  {}  {}{}",
            cookie.name, cookie.domain, cookie.path, scope, secure
        ));
    }
    out
}

pub fn cookie_debug_for_test() -> Vec<String> {
    let mut jar = CookieJar::empty_loaded();
    let a = jar.store_set_cookie(
        "https",
        "example.com",
        "/login/index.html",
        "sid=abc; Path=/; Secure; HttpOnly",
    );
    let b = jar.store_set_cookie(
        "https",
        "example.com",
        "/prefs",
        "theme=dark; Domain=example.com; Path=/",
    );
    let c = jar.store_set_cookie(
        "https",
        "example.com",
        "/prefs",
        "bad=x; Domain=evil.example; Path=/",
    );
    let secure = jar.cookie_header_for("https", "example.com", "/account");
    let plain = jar.cookie_header_for("http", "example.com", "/account");
    let subdomain = jar.cookie_header_for("https", "www.example.com", "/account");
    let deleted = jar.store_set_cookie(
        "https",
        "example.com",
        "/login/index.html",
        "sid=gone; Max-Age=0; Path=/",
    );
    let after_delete = jar.cookie_header_for("https", "example.com", "/account");
    vec![
        format!("stored_sid={}", a),
        format!("stored_theme={}", b),
        format!("rejected_domain={}", !c),
        format!(
            "secure_header={}",
            secure.unwrap_or_else(|| String::from("-"))
        ),
        format!(
            "plain_header={}",
            plain.unwrap_or_else(|| String::from("-"))
        ),
        format!(
            "subdomain_header={}",
            subdomain.unwrap_or_else(|| String::from("-"))
        ),
        format!("deleted_sid={}", deleted),
        format!(
            "after_delete={}",
            after_delete.unwrap_or_else(|| String::from("-"))
        ),
    ]
}

impl CookieJar {
    fn empty_loaded() -> Self {
        Self {
            loaded: true,
            cookies: Vec::new(),
        }
    }

    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        self.cookies.clear();
        let Some(bytes) = crate::config_store::read(COOKIE_PATH) else {
            return;
        };
        let Ok(text) = core::str::from_utf8(&bytes) else {
            crate::config_store::recover_corrupt(
                COOKIE_PATH,
                "/CONFIG/BROWSER.COOKIES.BAD",
                &bytes,
            );
            return;
        };
        for line in text.lines() {
            if let Some(cookie) = parse_cookie_line(line) {
                upsert_cookie(&mut self.cookies, cookie);
            }
            if self.cookies.len() >= MAX_COOKIES {
                break;
            }
        }
    }

    fn save(&self) {
        let mut out = String::new();
        for cookie in self.cookies.iter().take(MAX_COOKIES) {
            out.push_str("cookie|");
            out.push_str(&cookie.name);
            out.push('|');
            out.push_str(&cookie.value);
            out.push('|');
            out.push_str(&cookie.domain);
            out.push('|');
            out.push_str(&cookie.path);
            out.push('|');
            out.push_str(if cookie.secure { "secure" } else { "plain" });
            out.push('|');
            out.push_str(if cookie.host_only { "host" } else { "domain" });
            out.push('\n');
        }
        let _ = crate::config_store::safe_write(COOKIE_PATH, out.as_bytes());
    }

    fn cookie_header_for(&self, scheme: &str, host: &str, path: &str) -> Option<String> {
        let host = canonical_host(host);
        let path = request_path(path);
        if host.is_empty() {
            return None;
        }
        let mut header = String::new();
        for cookie in &self.cookies {
            if cookie.secure && scheme != "https" {
                continue;
            }
            if !cookie_domain_matches(cookie, &host) || !cookie_path_matches(&cookie.path, &path) {
                continue;
            }
            let pair_len = cookie.name.len() + cookie.value.len() + 1;
            let sep_len = if header.is_empty() { 0 } else { 2 };
            if header.len() + sep_len + pair_len > MAX_COOKIE_HEADER {
                break;
            }
            if !header.is_empty() {
                header.push_str("; ");
            }
            header.push_str(&cookie.name);
            header.push('=');
            header.push_str(&cookie.value);
        }
        if header.is_empty() {
            None
        } else {
            Some(header)
        }
    }

    fn store_set_cookie(&mut self, scheme: &str, host: &str, path: &str, header: &str) -> bool {
        let host = canonical_host(host);
        if host.is_empty() {
            return false;
        }
        let Some(parsed) = parse_set_cookie(scheme, &host, path, header) else {
            return false;
        };
        if parsed.delete {
            return remove_cookie(
                &mut self.cookies,
                &parsed.cookie.name,
                &parsed.cookie.domain,
                &parsed.cookie.path,
            );
        }
        upsert_cookie(&mut self.cookies, parsed.cookie)
    }
}

struct ParsedCookie {
    cookie: Cookie,
    delete: bool,
}

fn parse_set_cookie(scheme: &str, host: &str, path: &str, header: &str) -> Option<ParsedCookie> {
    let mut parts = header.split(';');
    let first = parts.next()?.trim();
    let (name, value) = first.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if !valid_cookie_name(name) || !valid_cookie_field(value, MAX_COOKIE_VALUE) {
        return None;
    }
    let mut cookie = Cookie {
        name: String::from(name),
        value: String::from(value),
        domain: String::from(host),
        path: default_cookie_path(path),
        secure: false,
        host_only: true,
    };
    let mut delete = false;
    for attr in parts {
        let attr = attr.trim();
        if attr.eq_ignore_ascii_case("secure") {
            cookie.secure = true;
            continue;
        }
        if attr.eq_ignore_ascii_case("httponly")
            || attr.to_ascii_lowercase().starts_with("samesite")
        {
            continue;
        }
        let Some((key, value)) = attr.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.eq_ignore_ascii_case("domain") {
            let domain = canonical_host(value.trim_start_matches('.'));
            if domain.is_empty() || domain.len() > MAX_COOKIE_DOMAIN || !domain_match(host, &domain)
            {
                return None;
            }
            cookie.domain = domain;
            cookie.host_only = false;
        } else if key.eq_ignore_ascii_case("path") {
            if value.starts_with('/') && valid_cookie_field(value, MAX_COOKIE_PATH) {
                cookie.path = String::from(value);
            }
        } else if key.eq_ignore_ascii_case("max-age") {
            if value.trim_start().starts_with('-') || value.trim() == "0" {
                delete = true;
            }
        }
    }
    if cookie.secure && scheme != "https" {
        return None;
    }
    Some(ParsedCookie { cookie, delete })
}

fn parse_cookie_line(line: &str) -> Option<Cookie> {
    let mut parts = line.split('|');
    if parts.next()? != "cookie" {
        return None;
    }
    let name = parts.next()?;
    let value = parts.next()?;
    let domain = parts.next()?;
    let path = parts.next()?;
    let secure = match parts.next()? {
        "secure" => true,
        "plain" => false,
        _ => return None,
    };
    let host_only = match parts.next()? {
        "host" => true,
        "domain" => false,
        _ => return None,
    };
    let domain = canonical_host(domain);
    if parts.next().is_some()
        || !valid_cookie_name(name)
        || !valid_cookie_field(value, MAX_COOKIE_VALUE)
        || domain.is_empty()
        || !valid_cookie_field(path, MAX_COOKIE_PATH)
        || !path.starts_with('/')
    {
        return None;
    }
    Some(Cookie {
        name: String::from(name),
        value: String::from(value),
        domain,
        path: String::from(path),
        secure,
        host_only,
    })
}

fn upsert_cookie(cookies: &mut Vec<Cookie>, cookie: Cookie) -> bool {
    if let Some(existing) = cookies.iter_mut().find(|existing| {
        existing.name == cookie.name
            && existing.domain == cookie.domain
            && existing.path == cookie.path
    }) {
        *existing = cookie;
        return true;
    }
    if cookies.len() >= MAX_COOKIES {
        cookies.remove(0);
    }
    cookies.push(cookie);
    true
}

fn remove_cookie(cookies: &mut Vec<Cookie>, name: &str, domain: &str, path: &str) -> bool {
    let Some(pos) = cookies
        .iter()
        .position(|cookie| cookie.name == name && cookie.domain == domain && cookie.path == path)
    else {
        return false;
    };
    cookies.remove(pos);
    true
}

fn cookie_domain_matches(cookie: &Cookie, host: &str) -> bool {
    if cookie.host_only {
        host == cookie.domain
    } else {
        domain_match(host, &cookie.domain)
    }
}

fn domain_match(host: &str, domain: &str) -> bool {
    host == domain
        || (host.len() > domain.len()
            && host.ends_with(domain)
            && host.as_bytes()[host.len() - domain.len() - 1] == b'.')
}

fn cookie_path_matches(cookie_path: &str, request_path: &str) -> bool {
    if cookie_path == "/" || request_path == cookie_path {
        return true;
    }
    request_path.starts_with(cookie_path)
        && (cookie_path.ends_with('/')
            || request_path
                .as_bytes()
                .get(cookie_path.len())
                .map(|b| *b == b'/')
                .unwrap_or(false))
}

fn default_cookie_path(path: &str) -> String {
    let path = request_path(path);
    if !path.starts_with('/') {
        return String::from("/");
    }
    let Some(last_slash) = path.rfind('/') else {
        return String::from("/");
    };
    if last_slash == 0 {
        String::from("/")
    } else {
        String::from(&path[..last_slash])
    }
}

fn request_path(path: &str) -> String {
    let mut path = if path.is_empty() {
        String::from("/")
    } else {
        String::from(path)
    };
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if let Some(pos) = path.find('?') {
        path.truncate(pos);
    }
    path
}

fn canonical_host(host: &str) -> String {
    let mut out = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if let Some(pos) = out.find(':') {
        out.truncate(pos);
    }
    if out.is_empty()
        || out.len() > MAX_COOKIE_DOMAIN
        || out.starts_with('.')
        || out.contains("..")
        || !out
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
    {
        String::new()
    } else {
        out
    }
}

fn valid_cookie_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_COOKIE_NAME
        && name
            .bytes()
            .all(|b| b > 0x20 && b < 0x7f && !matches!(b, b'=' | b';' | b',' | b'|' | b'\\'))
}

fn valid_cookie_field(value: &str, max: usize) -> bool {
    value.len() <= max
        && value
            .bytes()
            .all(|b| b >= 0x20 && b < 0x7f && !matches!(b, b'\r' | b'\n' | b'\t' | b';' | b'|'))
}
