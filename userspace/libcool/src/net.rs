use super::{sys, Error, Result};

pub const AF_INET: u64 = 2;
pub const SOCK_STREAM: u64 = 1;
pub const IPPROTO_TCP: u64 = 6;

pub fn dns_resolve(host: &[u8]) -> Result<u32> {
    let ret =
        unsafe { sys::syscall2(sys::DNS_RESOLVE, host.as_ptr() as u64, host.len() as u64) };
    Error::from_ret(ret).map(|addr| addr as u32)
}

pub fn http_get(host: &[u8]) -> Result<usize> {
    let ret = unsafe { sys::syscall2(sys::HTTP_GET, host.as_ptr() as u64, host.len() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn socket(domain: u64, socket_type: u64, protocol: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall3(sys::SOCKET, domain, socket_type, protocol) };
    Error::from_ret(ret)
}

pub fn tcp_socket() -> Result<u64> {
    socket(AF_INET, SOCK_STREAM, IPPROTO_TCP)
}

pub fn connect(socket: u64, ipv4_be: u32, port: u16) -> Result<()> {
    let ret = unsafe { sys::syscall3(sys::CONNECT, socket, ipv4_be as u64, port as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn send(socket: u64, bytes: &[u8]) -> Result<usize> {
    if bytes.is_empty() {
        return Ok(0);
    }
    let ret =
        unsafe { sys::syscall3(sys::SEND, socket, bytes.as_ptr() as u64, bytes.len() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn send_all(socket: u64, mut bytes: &[u8]) -> Result<()> {
    while !bytes.is_empty() {
        let n = send(socket, bytes)?;
        if n == 0 {
            return Err(Error::Failed);
        }
        bytes = &bytes[n.min(bytes.len())..];
    }
    Ok(())
}

pub fn recv(socket: u64, buf: &mut [u8]) -> Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }
    let ret =
        unsafe { sys::syscall3(sys::RECV, socket, buf.as_mut_ptr() as u64, buf.len() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn parse_ipv4(s: &[u8]) -> Option<u32> {
    let mut out = 0u32;
    let mut part = 0u32;
    let mut parts = 0usize;
    let mut saw_digit = false;
    for &b in s {
        if b == b'.' {
            if !saw_digit || part > 255 {
                return None;
            }
            out = (out << 8) | part;
            part = 0;
            saw_digit = false;
            parts += 1;
        } else if b.is_ascii_digit() {
            part = part * 10 + (b - b'0') as u32;
            saw_digit = true;
        } else {
            return None;
        }
    }
    if !saw_digit || part > 255 || parts != 3 {
        return None;
    }
    Some((out << 8) | part)
}
