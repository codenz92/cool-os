#![no_std]
#![no_main]

use libcool::{io, net, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let url = args.get(1).unwrap_or(b"http://93.184.216.34/");
    match run(url) {
        Ok(()) => exit(0),
        Err(msg) => {
            io::write_stdout(b"wget: ");
            io::write_stdout(msg);
            println!();
            exit(1);
        }
    }
}

fn run(url: &[u8]) -> core::result::Result<(), &'static [u8]> {
    let (host, path) = parse_http_url(url).ok_or(&b"usage: wget http://host/path"[..])?;
    let ip = match net::parse_ipv4(host) {
        Some(ip) => ip,
        None => net::dns_resolve(host).map_err(|_| &b"dns failed"[..])?,
    };

    let sock = net::tcp_socket().map_err(|_| &b"socket failed"[..])?;
    if net::connect(sock, ip, 80).is_err() {
        close(sock);
        return Err(b"connect failed");
    }

    let mut request = [0u8; 512];
    let mut len = 0usize;
    append(&mut request, &mut len, b"GET ")?;
    append(&mut request, &mut len, path)?;
    append(&mut request, &mut len, b" HTTP/1.1\r\nHost: ")?;
    append(&mut request, &mut len, host)?;
    append(
        &mut request,
        &mut len,
        b"\r\nUser-Agent: coolOS-wget/20\r\nAccept: */*\r\nConnection: close\r\n\r\n",
    )?;

    net::send_all(sock, &request[..len]).map_err(|_| &b"send failed"[..])?;

    let mut buf = [0u8; 512];
    loop {
        let n = net::recv(sock, &mut buf).map_err(|_| &b"recv failed"[..])?;
        if n == 0 {
            break;
        }
        io::write_stdout(&buf[..n]);
    }
    close(sock);
    Ok(())
}

fn parse_http_url(url: &[u8]) -> Option<(&[u8], &[u8])> {
    let rest = if starts_with(url, b"http://") {
        &url[7..]
    } else {
        url
    };
    if rest.is_empty() {
        return None;
    }
    let slash = rest.iter().position(|&b| b == b'/').unwrap_or(rest.len());
    let host = &rest[..slash];
    if host.is_empty() {
        return None;
    }
    let path = if slash < rest.len() {
        &rest[slash..]
    } else {
        b"/"
    };
    Some((host, path))
}

fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && &haystack[..needle.len()] == needle
}

fn append(
    out: &mut [u8],
    len: &mut usize,
    bytes: &[u8],
) -> core::result::Result<(), &'static [u8]> {
    if *len + bytes.len() > out.len() {
        return Err(b"request too large");
    }
    out[*len..*len + bytes.len()].copy_from_slice(bytes);
    *len += bytes.len();
    Ok(())
}
