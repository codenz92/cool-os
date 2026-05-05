extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use embedded_io::{ErrorKind, ErrorType, Read, Write};
use embedded_tls::blocking::{
    Aes128GcmSha256, Certificate, CryptoProvider, TlsConfig, TlsConnection, TlsContext, TlsError,
    TlsVerifier,
};
use embedded_tls::pki::CertVerifier;
use embedded_tls::TlsClock;
use rand_core::{CryptoRng, RngCore};
use spin::Mutex;

const TLS_RECORD_BUF: usize = 16_640;
const CERT_CHAIN_BUF: usize = 8 * 1024;
const TLS_ROOT_CACHE_MAX: usize = 16;

#[derive(Clone)]
struct RootCacheEntry {
    host: String,
    root_name: &'static str,
}

static ROOT_CACHE: Mutex<Vec<RootCacheEntry>> = Mutex::new(Vec::new());

pub struct TlsHttpExchange {
    pub resolved_addr: u32,
    pub raw_response: Vec<u8>,
    pub trust_root: &'static str,
}

enum TlsExchangeError {
    Handshake(&'static str),
    Established(&'static str),
}

pub fn https_exchange(
    host: &str,
    path: &str,
    request: &str,
    max_bytes: usize,
) -> Result<TlsHttpExchange, &'static str> {
    if !crate::entropy::has_hardware_rng() {
        return Err("TLS hardware RNG unavailable");
    }
    if CoolClock::now().is_none() {
        return Err("TLS RTC clock unavailable");
    }

    let addrs = crate::net::resolve_host_addrs(host)?;
    let mut last_err = "TLS connect failed";
    'addr_loop: for addr in addrs {
        let cached_name = cached_root_name(host);
        if let Some(root) = cached_root(host) {
            match https_exchange_with_root(host, path, request, max_bytes, addr, root) {
                Ok(exchange) => return Ok(exchange),
                Err(TlsExchangeError::Handshake(err))
                    if err == "TLS hostname validation failed" =>
                {
                    return Err(err)
                }
                Err(TlsExchangeError::Handshake(err)) => last_err = err,
                Err(TlsExchangeError::Established(err)) if err == "TLS response timeout" => {
                    last_err = err;
                    continue 'addr_loop;
                }
                Err(TlsExchangeError::Established(err)) => return Err(err),
            }
        }
        for root in crate::tls_roots::TRUST_ROOTS {
            if cached_name == Some(root.name) {
                continue;
            }
            match https_exchange_with_root(host, path, request, max_bytes, addr, root) {
                Ok(exchange) => {
                    remember_root(host, root.name);
                    return Ok(exchange);
                }
                Err(TlsExchangeError::Handshake(err))
                    if err == "TLS hostname validation failed" =>
                {
                    return Err(err)
                }
                Err(TlsExchangeError::Handshake(err)) => last_err = err,
                Err(TlsExchangeError::Established(err)) if err == "TLS response timeout" => {
                    last_err = err;
                    continue 'addr_loop;
                }
                Err(TlsExchangeError::Established(err)) => return Err(err),
            }
        }
    }
    Err(last_err)
}

pub fn status_lines() -> Vec<String> {
    let mut lines = crate::entropy::status_lines();
    lines.push(format!(
        "tls: TLS 1.3 client roots={} root_cache={} cipher=TLS_AES_128_GCM_SHA256 group=P-256",
        crate::tls_roots::TRUST_ROOTS.len(),
        ROOT_CACHE.lock().len()
    ));
    for root in crate::tls_roots::TRUST_ROOTS {
        lines.push(format!("trust root: {}", root.name));
    }
    lines
}

pub fn selftest_lines() -> Vec<String> {
    let detail = hostname_selftest_detail();
    let ok = detail.iter().all(|(_, passed)| *passed);
    vec![
        format!(
            "TLS hostname exact={} wildcard={} wildcard-negative={}",
            test_value(&detail, "exact"),
            test_value(&detail, "wildcard"),
            test_value(&detail, "wildcard-extra-label")
        ),
        format!(
            "TLS SAN-first={} CN-fallback={} IP-SAN={}",
            test_value(&detail, "san-over-cn"),
            test_value(&detail, "cn-fallback"),
            test_value(&detail, "ip-san")
        ),
        format!("TLS hostname negative {}", if ok { "ok" } else { "failed" }),
    ]
}

pub fn hostname_selftest_passes() -> bool {
    hostname_selftest_detail().iter().all(|(_, passed)| *passed)
}

pub fn http_response_selftest_passes() -> bool {
    http_response_complete(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
        && http_response_complete(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello")
        && !http_response_complete(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhell")
        && http_response_complete(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
        )
        && !http_response_complete(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel")
}

fn hostname_selftest_detail() -> Vec<(&'static str, bool)> {
    let crowded_san_names = [
        "*.google.com",
        "*.appengine.google.com",
        "*.bdn.dev",
        "*.origin-test.bdn.dev",
        "*.cloud.google.com",
        "*.crowdsource.google.com",
        "*.datacompute.google.com",
        "*.google.ca",
        "*.google.cl",
        "*.google.co.in",
        "*.google.co.jp",
        "*.google.co.uk",
        "*.google.com.ar",
        "*.google.com.au",
        "*.google.com.br",
        "*.google.com.co",
        "google.com",
    ];
    vec![
        (
            "exact",
            embedded_tls::pki::hostname_matches_for_test(
                "example.com",
                None,
                &["example.com"],
                &[],
            ),
        ),
        (
            "case-insensitive",
            embedded_tls::pki::hostname_matches_for_test(
                "WWW.BADSSL.COM",
                None,
                &["*.badssl.com"],
                &[],
            ),
        ),
        (
            "trailing-dot",
            embedded_tls::pki::hostname_matches_for_test(
                "example.com.",
                None,
                &["example.com"],
                &[],
            ),
        ),
        (
            "wildcard",
            embedded_tls::pki::hostname_matches_for_test(
                "www.badssl.com",
                None,
                &["*.badssl.com"],
                &[],
            ),
        ),
        (
            "wildcard-extra-label",
            !embedded_tls::pki::hostname_matches_for_test(
                "wrong.host.badssl.com",
                None,
                &["*.badssl.com"],
                &[],
            ),
        ),
        (
            "wildcard-tld",
            !embedded_tls::pki::hostname_matches_for_test("example.com", None, &["*.com"], &[]),
        ),
        (
            "san-over-cn",
            !embedded_tls::pki::hostname_matches_for_test(
                "legacy.example",
                Some("legacy.example"),
                &["modern.example"],
                &[],
            ),
        ),
        (
            "san-over-capacity",
            embedded_tls::pki::hostname_matches_for_test(
                "google.com",
                Some("*.google.com"),
                &crowded_san_names,
                &[],
            ),
        ),
        (
            "cn-fallback",
            embedded_tls::pki::hostname_matches_for_test(
                "legacy.example",
                Some("legacy.example"),
                &[],
                &[],
            ),
        ),
        (
            "ip-san",
            embedded_tls::pki::hostname_matches_for_test("192.0.2.1", None, &[], &[[192, 0, 2, 1]]),
        ),
        (
            "ip-no-cn-fallback",
            !embedded_tls::pki::hostname_matches_for_test("192.0.2.1", Some("192.0.2.1"), &[], &[]),
        ),
    ]
}

fn test_value(detail: &[(&'static str, bool)], name: &str) -> bool {
    detail
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, passed)| *passed)
        .unwrap_or(false)
}

fn cached_root(host: &str) -> Option<&'static crate::tls_roots::TrustRoot> {
    let root_name = cached_root_name(host)?;
    crate::tls_roots::TRUST_ROOTS
        .iter()
        .find(|root| root.name == root_name)
}

fn cached_root_name(host: &str) -> Option<&'static str> {
    ROOT_CACHE
        .lock()
        .iter()
        .find(|entry| entry.host.eq_ignore_ascii_case(host))
        .map(|entry| entry.root_name)
}

fn remember_root(host: &str, root_name: &'static str) {
    let mut cache = ROOT_CACHE.lock();
    if let Some(entry) = cache
        .iter_mut()
        .find(|entry| entry.host.eq_ignore_ascii_case(host))
    {
        entry.root_name = root_name;
        return;
    }
    cache.push(RootCacheEntry {
        host: String::from(host),
        root_name,
    });
    if cache.len() > TLS_ROOT_CACHE_MAX {
        cache.remove(0);
    }
}

fn https_exchange_with_root(
    host: &str,
    _path: &str,
    request: &str,
    max_bytes: usize,
    addr: u32,
    root: &crate::tls_roots::TrustRoot,
) -> Result<TlsHttpExchange, TlsExchangeError> {
    let stream = KernelTcpStream::connect(addr, 443).map_err(TlsExchangeError::Handshake)?;
    let mut read_record_buffer = alloc::vec![0u8; TLS_RECORD_BUF];
    let mut write_record_buffer = alloc::vec![0u8; TLS_RECORD_BUF];
    let mut tls = TlsConnection::new(
        stream,
        read_record_buffer.as_mut_slice(),
        write_record_buffer.as_mut_slice(),
    );
    let config = TlsConfig::new()
        .enable_rsa_signatures()
        .with_ca(Certificate::X509(root.der))
        .with_server_name(host);
    let provider = CoolTlsProvider::new();
    tls.open(TlsContext::new(&config, provider))
        .map_err(|err| TlsExchangeError::Handshake(tls_error_label(err)))?;
    let mut sent = 0usize;
    let bytes = request.as_bytes();
    while sent < bytes.len() {
        let n = tls
            .write(&bytes[sent..])
            .map_err(|_| TlsExchangeError::Established("TLS write failed"))?;
        if n == 0 {
            return Err(TlsExchangeError::Established("TLS write stalled"));
        }
        sent += n;
    }
    tls.flush()
        .map_err(|_| TlsExchangeError::Established("TLS flush failed"))?;

    let mut raw_response = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        match tls.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw_response.extend_from_slice(&buf[..n]);
                if raw_response.len() >= max_bytes || http_response_complete(&raw_response) {
                    break;
                }
            }
            Err(TlsError::ConnectionClosed) => break,
            Err(TlsError::IoError) => {
                return Err(TlsExchangeError::Established("TLS response timeout"))
            }
            Err(err) => return Err(TlsExchangeError::Established(tls_error_label(err))),
        }
    }
    let _ = tls.close();
    if raw_response.is_empty() {
        return Err(TlsExchangeError::Established("TLS response empty"));
    }
    Ok(TlsHttpExchange {
        resolved_addr: addr,
        raw_response,
        trust_root: root.name,
    })
}

fn http_response_complete(response: &[u8]) -> bool {
    let Some((headers_len, sep_len)) = http_header_end(response) else {
        return false;
    };
    let headers = &response[..headers_len];
    let body = &response[headers_len + sep_len..];
    if http_status_has_no_body(headers) {
        return true;
    }
    if let Some(length) = http_content_length(headers) {
        return body.len() >= length;
    }
    if http_transfer_chunked(headers) {
        return chunked_body_complete(body);
    }
    false
}

fn http_header_end(response: &[u8]) -> Option<(usize, usize)> {
    find_bytes(response, b"\r\n\r\n")
        .map(|pos| (pos, 4))
        .or_else(|| find_bytes(response, b"\n\n").map(|pos| (pos, 2)))
}

fn http_status_has_no_body(headers: &[u8]) -> bool {
    let Ok(headers) = core::str::from_utf8(headers) else {
        return false;
    };
    let Some(status) = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
    else {
        return false;
    };
    (100..200).contains(&status) || status == 204 || status == 304
}

fn http_content_length(headers: &[u8]) -> Option<usize> {
    http_header_value(headers, "content-length")?
        .trim()
        .parse::<usize>()
        .ok()
}

fn http_transfer_chunked(headers: &[u8]) -> bool {
    http_header_value(headers, "transfer-encoding")
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("chunked"))
        })
        .unwrap_or(false)
}

fn http_header_value<'a>(headers: &'a [u8], name: &str) -> Option<&'a str> {
    let Ok(headers) = core::str::from_utf8(headers) else {
        return None;
    };
    for line in headers.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case(name) {
            return Some(value);
        }
    }
    None
}

fn chunked_body_complete(body: &[u8]) -> bool {
    let mut pos = 0usize;
    loop {
        let Some(line_end) = find_crlf(body, pos) else {
            return false;
        };
        let Ok(line) = core::str::from_utf8(&body[pos..line_end]) else {
            return false;
        };
        let Some(size) = parse_hex_usize(line.split(';').next().unwrap_or("").trim()) else {
            return false;
        };
        pos = line_end + 2;
        let Some(after_chunk) = pos.checked_add(size) else {
            return false;
        };
        if body.len() < after_chunk + 2 || &body[after_chunk..after_chunk + 2] != b"\r\n" {
            return false;
        }
        pos = after_chunk + 2;
        if size == 0 {
            return true;
        }
    }
}

fn find_crlf(body: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 1 < body.len() {
        if body[pos] == b'\r' && body[pos + 1] == b'\n' {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_hex_usize(input: &str) -> Option<usize> {
    let mut value = 0usize;
    let mut saw_digit = false;
    for ch in input.bytes() {
        let digit = match ch {
            b'0'..=b'9' => (ch - b'0') as usize,
            b'a'..=b'f' => (ch - b'a' + 10) as usize,
            b'A'..=b'F' => (ch - b'A' + 10) as usize,
            _ => return None,
        };
        value = value.checked_mul(16)?.checked_add(digit)?;
        saw_digit = true;
    }
    saw_digit.then_some(value)
}

struct CoolTlsProvider {
    rng: CoolRng,
    verifier: CertVerifier<Aes128GcmSha256, CoolClock, CERT_CHAIN_BUF>,
}

impl CoolTlsProvider {
    fn new() -> Self {
        Self {
            rng: CoolRng,
            verifier: CertVerifier::new(),
        }
    }
}

impl CryptoProvider for CoolTlsProvider {
    type CipherSuite = Aes128GcmSha256;
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl embedded_tls::CryptoRngCore {
        &mut self.rng
    }

    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Aes128GcmSha256>, TlsError> {
        Ok(&mut self.verifier)
    }
}

struct CoolRng;

impl RngCore for CoolRng {
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        crate::entropy::fill_random(dest).expect("TLS hardware RNG failed");
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        crate::entropy::fill_random(dest)
    }
}

impl CryptoRng for CoolRng {}

struct CoolClock;

impl TlsClock for CoolClock {
    fn now() -> Option<u64> {
        let dt = crate::rtc::read_datetime()?;
        unix_seconds(dt.year, dt.month, dt.day, dt.hour, dt.minute)
    }
}

fn unix_seconds(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> Option<u64> {
    if year < 1970 || !(1..=12).contains(&month) || day == 0 || hour > 23 || minute > 59 {
        return None;
    }
    let mut days = 0u64;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [31u8, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += month_days[(m - 1) as usize] as u64;
        if m == 2 && is_leap(year) {
            days += 1;
        }
    }
    let max_day =
        month_days[(month - 1) as usize] + if month == 2 && is_leap(year) { 1 } else { 0 };
    if day > max_day {
        return None;
    }
    days += (day - 1) as u64;
    Some(days * 86_400 + hour as u64 * 3_600 + minute as u64 * 60)
}

fn is_leap(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[derive(Debug, Clone, Copy)]
struct KernelTcpError(ErrorKind);

impl core::fmt::Display for KernelTcpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl core::error::Error for KernelTcpError {}

impl embedded_io::Error for KernelTcpError {
    fn kind(&self) -> ErrorKind {
        self.0
    }
}

struct KernelTcpStream {
    fd: Option<u64>,
}

impl KernelTcpStream {
    fn connect(addr: u32, port: u16) -> Result<Self, &'static str> {
        let fd = crate::net::socket_open(crate::net::KERNEL_SOCKET_OWNER, 2, 1, 6)?;
        match crate::net::socket_connect(crate::net::KERNEL_SOCKET_OWNER, fd, addr, port) {
            Ok(()) => Ok(Self { fd: Some(fd) }),
            Err(err) => {
                let _ = crate::net::socket_close(crate::net::KERNEL_SOCKET_OWNER, fd);
                Err(err)
            }
        }
    }
}

impl Drop for KernelTcpStream {
    fn drop(&mut self) {
        if let Some(fd) = self.fd.take() {
            let _ = crate::net::socket_close(crate::net::KERNEL_SOCKET_OWNER, fd);
        }
    }
}

impl ErrorType for KernelTcpStream {
    type Error = KernelTcpError;
}

impl Read for KernelTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let fd = self.fd.ok_or(KernelTcpError(ErrorKind::NotConnected))?;
        crate::net::socket_recv(crate::net::KERNEL_SOCKET_OWNER, fd, buf)
            .map_err(|_| KernelTcpError(ErrorKind::Other))
    }
}

impl Write for KernelTcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let fd = self.fd.ok_or(KernelTcpError(ErrorKind::NotConnected))?;
        crate::net::socket_send(crate::net::KERNEL_SOCKET_OWNER, fd, buf)
            .map_err(|_| KernelTcpError(ErrorKind::Other))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn tls_error_label(err: TlsError) -> &'static str {
    match err {
        TlsError::InvalidCertificate => "TLS certificate validation failed",
        TlsError::InvalidCertificateRequest => "TLS hostname validation failed",
        TlsError::InvalidSignature => "TLS certificate signature invalid",
        TlsError::InvalidSignatureScheme => "TLS signature scheme unsupported",
        TlsError::InvalidCipherSuite => "TLS cipher suite unsupported",
        TlsError::InvalidKeyShare => "TLS key exchange failed",
        TlsError::HandshakeAborted(_, _) | TlsError::AbortHandshake(_, _) => {
            "TLS handshake aborted"
        }
        TlsError::Io(_) | TlsError::IoError | TlsError::ConnectionClosed => "TLS I/O failed",
        TlsError::Unimplemented => "TLS feature unsupported",
        _ => "TLS handshake failed",
    }
}
