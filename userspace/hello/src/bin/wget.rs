#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

const SYS_EXIT: u64 = 0;
const SYS_WRITE: u64 = 1;
const SYS_DNS: u64 = 17;
const SYS_SOCKET: u64 = 19;
const SYS_CONNECT: u64 = 20;
const SYS_SEND: u64 = 21;
const SYS_RECV: u64 = 22;

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "mov rdi, rsp",
        "jmp {entry}",
        entry = sym rust_start,
    );
}

extern "C" fn rust_start(rsp: u64) -> ! {
    let argc = unsafe { *(rsp as *const u64) };
    let url = if argc >= 2 {
        let argv1 = unsafe { *((rsp + 16) as *const u64) as *const u8 };
        unsafe { core::slice::from_raw_parts(argv1, c_strlen(argv1)) }
    } else {
        b"http://93.184.216.34/"
    };

    match run(url) {
        Ok(()) => exit(0),
        Err(msg) => {
            write_str(b"wget: ");
            write_str(msg);
            write_str(b"\n");
            exit(1);
        }
    }
}

fn run(url: &[u8]) -> Result<(), &'static [u8]> {
    let (host, path) = parse_http_url(url).ok_or(&b"usage: wget http://host/path"[..])?;
    let ip = parse_ipv4(host).unwrap_or_else(|| syscall2(SYS_DNS, host.as_ptr() as u64, host.len() as u64) as u32);
    if ip == u32::MAX {
        return Err(b"dns failed");
    }

    let sock = syscall3(SYS_SOCKET, 2, 1, 6);
    if sock == u64::MAX {
        return Err(b"socket failed");
    }
    if syscall3(SYS_CONNECT, sock, ip as u64, 80) == u64::MAX {
        return Err(b"connect failed");
    }

    let mut request = [0u8; 512];
    let mut len = 0usize;
    append(&mut request, &mut len, b"GET ")?;
    append(&mut request, &mut len, path)?;
    append(&mut request, &mut len, b" HTTP/1.0\r\nHost: ")?;
    append(&mut request, &mut len, host)?;
    append(&mut request, &mut len, b"\r\nConnection: close\r\n\r\n")?;

    let sent = syscall3(SYS_SEND, sock, request.as_ptr() as u64, len as u64);
    if sent == u64::MAX {
        return Err(b"send failed");
    }

    let mut buf = [0u8; 512];
    loop {
        let n = syscall3(SYS_RECV, sock, buf.as_mut_ptr() as u64, buf.len() as u64);
        if n == u64::MAX {
            return Err(b"recv failed");
        }
        if n == 0 {
            break;
        }
        write_str(&buf[..n as usize]);
    }
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
    let path = if slash < rest.len() { &rest[slash..] } else { b"/" };
    Some((host, path))
}

fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && &haystack[..needle.len()] == needle
}

fn parse_ipv4(s: &[u8]) -> Option<u32> {
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

fn append(out: &mut [u8], len: &mut usize, bytes: &[u8]) -> Result<(), &'static [u8]> {
    if *len + bytes.len() > out.len() {
        return Err(b"request too large");
    }
    out[*len..*len + bytes.len()].copy_from_slice(bytes);
    *len += bytes.len();
    Ok(())
}

fn syscall1(nr: u64, a1: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            inlateout("rdi") a1 => _,
            lateout("rcx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    ret
}

fn syscall2(nr: u64, a1: u64, a2: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            inlateout("rdi") a1 => _,
            inlateout("rsi") a2 => _,
            lateout("rcx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    ret
}

fn syscall3(nr: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            inlateout("rdi") a1 => _,
            inlateout("rsi") a2 => _,
            inlateout("rdx") a3 => _,
            lateout("rcx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    ret
}

fn write_str(s: &[u8]) {
    let _ = syscall3(SYS_WRITE, 1, s.as_ptr() as u64, s.len() as u64);
}

fn exit(code: u64) -> ! {
    let _ = syscall1(SYS_EXIT, code);
    loop {
        core::hint::spin_loop();
    }
}

fn c_strlen(mut s: *const u8) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s != 0 {
            n += 1;
            s = s.add(1);
        }
    }
    n
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
