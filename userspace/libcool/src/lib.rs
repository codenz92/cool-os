#![no_std]

use core::arch::asm;

pub const SDK_VERSION: u64 = 1;
pub const ABI_VERSION: u64 = 6;
pub const U64_MAX: u64 = u64::MAX;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Failed,
    Invalid,
}

impl Error {
    #[inline]
    pub const fn from_ret(ret: u64) -> Result<u64> {
        if ret == U64_MAX {
            Err(Error::Failed)
        } else {
            Ok(ret)
        }
    }
}

pub mod sys {
    use super::*;

    pub const EXIT: u64 = 0;
    pub const WRITE: u64 = 1;
    pub const YIELD: u64 = 2;
    pub const GETPID: u64 = 3;
    pub const MMAP: u64 = 4;
    pub const OPEN: u64 = 5;
    pub const READ: u64 = 6;
    pub const CLOSE: u64 = 7;
    pub const EXEC: u64 = 8;
    pub const PIPE: u64 = 9;
    pub const DUP: u64 = 10;
    pub const SHMEM_CREATE: u64 = 11;
    pub const SHMEM_MAP: u64 = 12;
    pub const WAITPID: u64 = 13;
    pub const SPAWN: u64 = 14;
    pub const SLEEP_MS: u64 = 15;
    pub const ABI_VERSION: u64 = 16;
    pub const DNS_RESOLVE: u64 = 17;
    pub const HTTP_GET: u64 = 18;
    pub const SOCKET: u64 = 19;
    pub const CONNECT: u64 = 20;
    pub const SEND: u64 = 21;
    pub const RECV: u64 = 22;
    pub const GUI_OPEN: u64 = 23;
    pub const GUI_PRESENT: u64 = 24;
    pub const GUI_POLL_EVENT: u64 = 25;
    pub const GUI_CLOSE: u64 = 26;
    pub const FS_WRITE_FILE: u64 = 27;
    pub const FS_CREATE_DIR: u64 = 28;
    pub const FS_DELETE_TREE: u64 = 29;
    pub const FS_LIST_DIR: u64 = 30;
    pub const SCREENSHOT: u64 = 31;
    pub const SIGNAL: u64 = 32;
    pub const SETPGID: u64 = 33;
    pub const GETPGID: u64 = 34;
    pub const SIGNAL_GROUP: u64 = 35;

    #[inline]
    pub unsafe fn syscall0(nr: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            lateout("rcx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            options(nostack),
        );
        ret
    }

    #[inline]
    pub unsafe fn syscall1(nr: u64, a1: u64) -> u64 {
        let ret: u64;
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
        ret
    }

    #[inline]
    pub unsafe fn syscall2(nr: u64, a1: u64, a2: u64) -> u64 {
        let ret: u64;
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
        ret
    }

    #[inline]
    pub unsafe fn syscall3(nr: u64, a1: u64, a2: u64, a3: u64) -> u64 {
        let ret: u64;
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
        ret
    }
}

pub mod args {
    #[derive(Clone, Copy)]
    pub struct Args {
        rsp: u64,
        argc: usize,
    }

    impl Args {
        /// Build argv access from the initial userspace stack.
        ///
        /// The kernel lays the stack out as `argc, argv..., null, envp_null`.
        #[inline]
        pub unsafe fn from_stack(rsp: u64) -> Self {
            let argc = *(rsp as *const u64) as usize;
            Args { rsp, argc }
        }

        #[inline]
        pub const fn len(self) -> usize {
            self.argc
        }

        #[inline]
        pub const fn is_empty(self) -> bool {
            self.argc == 0
        }

        pub fn get(self, index: usize) -> Option<&'static [u8]> {
            if index >= self.argc {
                return None;
            }
            let ptr_slot = (self.rsp + 8 + index as u64 * 8) as *const u64;
            let ptr = unsafe { *ptr_slot } as *const u8;
            if ptr.is_null() {
                return None;
            }
            let len = unsafe { c_strlen(ptr) };
            Some(unsafe { core::slice::from_raw_parts(ptr, len) })
        }

        #[inline]
        pub fn program(self) -> Option<&'static [u8]> {
            self.get(0)
        }
    }

    unsafe fn c_strlen(mut s: *const u8) -> usize {
        let mut n = 0usize;
        while *s != 0 {
            n += 1;
            s = s.add(1);
        }
        n
    }
}

pub use args::Args;

pub mod process {
    use super::{sys, Error, Result};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Signal {
        Int,
        User1,
        Term,
        Continue,
        Stop,
    }

    impl Signal {
        pub const fn code(self) -> u64 {
            match self {
                Signal::Int => 2,
                Signal::User1 => 10,
                Signal::Term => 15,
                Signal::Continue => 18,
                Signal::Stop => 19,
            }
        }
    }

    #[inline]
    pub fn exit(code: u64) -> ! {
        unsafe {
            sys::syscall1(sys::EXIT, code);
        }
        abort()
    }

    #[inline]
    pub fn abort() -> ! {
        loop {
            core::hint::spin_loop();
        }
    }

    #[inline]
    pub fn yield_now() {
        unsafe {
            sys::syscall0(sys::YIELD);
        }
    }

    #[inline]
    pub fn getpid() -> u64 {
        unsafe { sys::syscall0(sys::GETPID) }
    }

    #[inline]
    pub fn abi_version() -> u64 {
        unsafe { sys::syscall0(sys::ABI_VERSION) }
    }

    #[inline]
    pub fn sleep_ms(ms: u64) {
        unsafe {
            sys::syscall1(sys::SLEEP_MS, ms);
        }
    }

    pub fn exec(path: &[u8]) -> Result<()> {
        let ret = unsafe { sys::syscall2(sys::EXEC, path.as_ptr() as u64, path.len() as u64) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn spawn(path: &[u8]) -> Result<u64> {
        let ret = unsafe { sys::syscall2(sys::SPAWN, path.as_ptr() as u64, path.len() as u64) };
        Error::from_ret(ret)
    }

    pub fn waitpid(pid: u64) -> Result<u64> {
        let mut status = 0u64;
        let ret = unsafe { sys::syscall2(sys::WAITPID, pid, &mut status as *mut u64 as u64) };
        Error::from_ret(ret).map(|_| status)
    }

    pub fn signal(pid: u64, signal: Signal) -> Result<()> {
        let ret = unsafe { sys::syscall2(sys::SIGNAL, pid, signal.code()) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn set_process_group(pid: u64, group: u64) -> Result<()> {
        let ret = unsafe { sys::syscall2(sys::SETPGID, pid, group) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn get_process_group(pid: u64) -> Result<u64> {
        let ret = unsafe { sys::syscall1(sys::GETPGID, pid) };
        Error::from_ret(ret)
    }

    pub fn signal_group(group: u64, signal: Signal) -> Result<u64> {
        let ret = unsafe { sys::syscall2(sys::SIGNAL_GROUP, group, signal.code()) };
        Error::from_ret(ret)
    }
}

pub mod memory {
    use super::{sys, Error, Result};

    pub const PROT_WRITE: u64 = 1;

    pub fn mmap(addr: u64, len: usize, writable: bool) -> Result<u64> {
        let flags = if writable { PROT_WRITE } else { 0 };
        let ret = unsafe { sys::syscall3(sys::MMAP, addr, len as u64, flags) };
        Error::from_ret(ret)
    }
}

pub mod io {
    use core::fmt;

    use super::{sys, Error, Result};

    pub const STDOUT: u64 = 1;
    pub const STDERR: u64 = 2;

    pub fn write(fd: u64, bytes: &[u8]) -> Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let ret =
            unsafe { sys::syscall3(sys::WRITE, fd, bytes.as_ptr() as u64, bytes.len() as u64) };
        Error::from_ret(ret).map(|n| n as usize)
    }

    pub fn write_all(fd: u64, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            let n = write(fd, bytes)?;
            if n == 0 {
                return Err(Error::Failed);
            }
            bytes = &bytes[n.min(bytes.len())..];
        }
        Ok(())
    }

    #[inline]
    pub fn write_stdout(bytes: &[u8]) {
        let _ = write_all(STDOUT, bytes);
    }

    #[inline]
    pub fn write_stderr(bytes: &[u8]) {
        let _ = write_all(STDERR, bytes);
    }

    #[inline]
    pub fn write_byte(fd: u64, byte: u8) -> Result<()> {
        write_all(fd, &[byte])
    }

    pub fn read(fd: u64, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let ret =
            unsafe { sys::syscall3(sys::READ, fd, buf.as_mut_ptr() as u64, buf.len() as u64) };
        Error::from_ret(ret).map(|n| n as usize)
    }

    pub fn open(path: &[u8]) -> Result<u64> {
        let ret = unsafe { sys::syscall2(sys::OPEN, path.as_ptr() as u64, path.len() as u64) };
        Error::from_ret(ret)
    }

    pub fn close(fd: u64) {
        unsafe {
            sys::syscall1(sys::CLOSE, fd);
        }
    }

    pub fn dup(fd: u64) -> Result<u64> {
        let ret = unsafe { sys::syscall1(sys::DUP, fd) };
        Error::from_ret(ret)
    }

    pub fn pipe() -> Result<(u64, u64)> {
        let mut fds = [0u64; 2];
        let ret = unsafe { sys::syscall1(sys::PIPE, fds.as_mut_ptr() as u64) };
        Error::from_ret(ret).map(|_| (fds[0], fds[1]))
    }

    pub struct File {
        fd: u64,
    }

    impl File {
        pub fn open(path: &[u8]) -> Result<Self> {
            open(path).map(|fd| File { fd })
        }

        #[inline]
        pub const fn fd(&self) -> u64 {
            self.fd
        }

        #[inline]
        pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
            read(self.fd, buf)
        }

        #[inline]
        pub fn write(&self, bytes: &[u8]) -> Result<usize> {
            write(self.fd, bytes)
        }

        #[inline]
        pub fn close(self) {
            let fd = self.fd;
            core::mem::forget(self);
            close(fd);
        }
    }

    impl Drop for File {
        fn drop(&mut self) {
            close(self.fd);
        }
    }

    pub struct Stdout;

    impl fmt::Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            write_all(STDOUT, s.as_bytes()).map_err(|_| fmt::Error)
        }
    }

    #[doc(hidden)]
    pub fn _print(args: fmt::Arguments<'_>) {
        let _ = fmt::write(&mut Stdout, args);
    }

    pub fn write_u64(mut n: u64) {
        if n == 0 {
            write_stdout(b"0");
            return;
        }
        let mut buf = [0u8; 20];
        let mut len = 0usize;
        while n > 0 {
            buf[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        while len > 0 {
            len -= 1;
            let _ = write_byte(STDOUT, buf[len]);
        }
    }

    pub fn write_ipv4(addr: u32) {
        write_u64(((addr >> 24) & 0xff) as u64);
        write_stdout(b".");
        write_u64(((addr >> 16) & 0xff) as u64);
        write_stdout(b".");
        write_u64(((addr >> 8) & 0xff) as u64);
        write_stdout(b".");
        write_u64((addr & 0xff) as u64);
    }
}

pub mod ipc {
    use super::{sys, Error, Result};

    pub fn shmem_create(len: usize) -> Result<u64> {
        let ret = unsafe { sys::syscall1(sys::SHMEM_CREATE, len as u64) };
        Error::from_ret(ret)
    }

    pub fn shmem_map(id: u64) -> Result<u64> {
        let ret = unsafe { sys::syscall1(sys::SHMEM_MAP, id) };
        Error::from_ret(ret)
    }
}

pub mod fs {
    use super::{io, sys, Error, Result};

    pub fn read_file(path: &[u8], out: &mut [u8]) -> Result<usize> {
        let file = io::File::open(path)?;
        let n = file.read(out)?;
        file.close();
        Ok(n)
    }

    pub fn write_file(path: &[u8], data: &[u8]) -> Result<()> {
        let desc = [
            path.as_ptr() as u64,
            path.len() as u64,
            data.as_ptr() as u64,
            data.len() as u64,
        ];
        let ret = unsafe { sys::syscall1(sys::FS_WRITE_FILE, desc.as_ptr() as u64) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn create_dir(path: &[u8]) -> Result<()> {
        let ret =
            unsafe { sys::syscall2(sys::FS_CREATE_DIR, path.as_ptr() as u64, path.len() as u64) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn delete_tree(path: &[u8]) -> Result<()> {
        let ret =
            unsafe { sys::syscall2(sys::FS_DELETE_TREE, path.as_ptr() as u64, path.len() as u64) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn list_dir(path: &[u8], out: &mut [u8]) -> Result<usize> {
        if out.is_empty() {
            return Err(Error::Invalid);
        }
        let desc = [
            path.as_ptr() as u64,
            path.len() as u64,
            out.as_mut_ptr() as u64,
            out.len() as u64,
        ];
        let ret = unsafe { sys::syscall1(sys::FS_LIST_DIR, desc.as_ptr() as u64) };
        Error::from_ret(ret).map(|n| n as usize)
    }

    pub fn screenshot(path: &[u8]) -> Result<()> {
        let ret =
            unsafe { sys::syscall3(sys::SCREENSHOT, path.as_ptr() as u64, path.len() as u64, 0) };
        Error::from_ret(ret).map(|_| ())
    }
}

pub mod net {
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
}

pub mod event {
    use super::{io, Error, Result};

    pub const INPUT_FD: u64 = 3;
    pub const EVENT_PACKET_SIZE: usize = 8;
    pub const EVENT_KIND_KEY_CHAR: u8 = 1;
    pub const EVENT_KIND_MOUSE_DOWN: u8 = 2;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Event {
        KeyChar { bytes: [u8; 4], len: usize },
        MouseDown { x: u16, y: u16 },
    }

    impl Event {
        pub fn parse(packet: &[u8; EVENT_PACKET_SIZE]) -> Result<Self> {
            match packet[0] {
                EVENT_KIND_KEY_CHAR => {
                    let len = packet[1] as usize;
                    if len == 0 || len > 4 {
                        return Err(Error::Invalid);
                    }
                    let mut bytes = [0u8; 4];
                    bytes[..len].copy_from_slice(&packet[2..2 + len]);
                    Ok(Event::KeyChar { bytes, len })
                }
                EVENT_KIND_MOUSE_DOWN => Ok(Event::MouseDown {
                    x: u16::from_le_bytes([packet[2], packet[3]]),
                    y: u16::from_le_bytes([packet[4], packet[5]]),
                }),
                _ => Err(Error::Invalid),
            }
        }
    }

    pub fn read_event(fd: u64) -> Result<Option<Event>> {
        let mut packet = [0u8; EVENT_PACKET_SIZE];
        let n = io::read(fd, &mut packet)?;
        if n == 0 {
            return Ok(None);
        }
        if n != EVENT_PACKET_SIZE {
            return Err(Error::Invalid);
        }
        Event::parse(&packet).map(Some)
    }
}

pub mod gui {
    use font8x8::UnicodeFonts;

    use super::{process, sys, Error, Result};

    pub const EVENT_PACKET_SIZE: usize = 16;
    pub const EVENT_KIND_KEY_CHAR: u8 = 1;
    pub const EVENT_KIND_MOUSE_DOWN: u8 = 2;
    pub const EVENT_KIND_CLOSE: u8 = 3;
    pub const EVENT_KIND_RESIZE: u8 = 4;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Event {
        KeyChar { bytes: [u8; 4], len: usize },
        MouseDown { button: u8, x: u16, y: u16 },
        Close,
        Resize { width: u16, height: u16 },
    }

    pub struct Window {
        handle: u64,
        width: usize,
        height: usize,
    }

    impl Window {
        pub fn open(title: &[u8], width: u16, height: u16) -> Result<Self> {
            let dims = (width as u64) | ((height as u64) << 16);
            let mut ret = u64::MAX;
            for _ in 0..128 {
                ret = unsafe {
                    sys::syscall3(
                        sys::GUI_OPEN,
                        title.as_ptr() as u64,
                        title.len() as u64,
                        dims,
                    )
                };
                if ret != u64::MAX {
                    break;
                }
                process::sleep_ms(1);
            }
            Error::from_ret(ret).map(|handle| Window {
                handle,
                width: width as usize,
                height: height as usize,
            })
        }

        #[inline]
        pub const fn handle(&self) -> u64 {
            self.handle
        }

        #[inline]
        pub const fn width(&self) -> usize {
            self.width
        }

        #[inline]
        pub const fn height(&self) -> usize {
            self.height
        }

        pub fn present(&self, pixels: &[u32]) -> Result<()> {
            if pixels.len() != self.width.saturating_mul(self.height) {
                return Err(Error::Invalid);
            }
            let bytes = unsafe {
                core::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)
            };
            let mut ret = u64::MAX;
            for _ in 0..64 {
                ret = unsafe {
                    sys::syscall3(
                        sys::GUI_PRESENT,
                        self.handle,
                        bytes.as_ptr() as u64,
                        bytes.len() as u64,
                    )
                };
                if ret != u64::MAX {
                    break;
                }
                process::sleep_ms(1);
            }
            Error::from_ret(ret).map(|_| ())
        }

        pub fn poll_event(&mut self) -> Result<Option<Event>> {
            let mut packet = [0u8; EVENT_PACKET_SIZE];
            let ret = unsafe {
                sys::syscall3(
                    sys::GUI_POLL_EVENT,
                    self.handle,
                    packet.as_mut_ptr() as u64,
                    EVENT_PACKET_SIZE as u64,
                )
            };
            if ret == u64::MAX {
                return Err(Error::Failed);
            }
            if ret == 0 {
                return Ok(None);
            }
            let event = parse_event(&packet)?;
            if let Event::Resize { width, height } = event {
                self.width = width as usize;
                self.height = height as usize;
            }
            Ok(Some(event))
        }

        pub fn close(self) -> Result<()> {
            let handle = self.handle;
            core::mem::forget(self);
            close_handle(handle)
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            let _ = close_handle(self.handle);
        }
    }

    pub struct Canvas<'a> {
        pixels: &'a mut [u32],
        width: usize,
        height: usize,
    }

    impl<'a> Canvas<'a> {
        pub fn new(pixels: &'a mut [u32], width: usize, height: usize) -> Self {
            Canvas {
                pixels,
                width,
                height,
            }
        }

        pub fn clear(&mut self, color: u32) {
            for pixel in self.pixels.iter_mut() {
                *pixel = color & 0x00ff_ffff;
            }
        }

        pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
            let x0 = x.max(0) as usize;
            let y0 = y.max(0) as usize;
            let x1 = (x + w).clamp(0, self.width as i32) as usize;
            let y1 = (y + h).clamp(0, self.height as i32) as usize;
            if x0 >= x1 || y0 >= y1 {
                return;
            }
            let color = color & 0x00ff_ffff;
            for row in y0..y1 {
                let start = row * self.width + x0;
                let end = row * self.width + x1;
                for pixel in &mut self.pixels[start..end] {
                    *pixel = color;
                }
            }
        }

        pub fn border(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
            if w <= 0 || h <= 0 {
                return;
            }
            self.rect(x, y, w, 1, color);
            self.rect(x, y + h - 1, w, 1, color);
            self.rect(x, y, 1, h, color);
            self.rect(x + w - 1, y, 1, h, color);
        }

        pub fn text(&mut self, x: i32, y: i32, text: &str, color: u32) {
            let mut cursor = x;
            for ch in text.chars() {
                self.char(cursor, y, ch, color);
                cursor += 8;
            }
        }

        pub fn char(&mut self, x: i32, y: i32, ch: char, color: u32) {
            let glyph = font8x8::BASIC_FONTS
                .get(ch)
                .or_else(|| font8x8::BASIC_FONTS.get(' '));
            let Some(glyph) = glyph else {
                return;
            };
            let color = color & 0x00ff_ffff;
            for (gy, &byte) in glyph.iter().enumerate() {
                for bit in 0..8usize {
                    if byte & (1 << bit) == 0 {
                        continue;
                    }
                    let px = x + bit as i32;
                    let py = y + gy as i32;
                    if px >= 0 && py >= 0 {
                        let px = px as usize;
                        let py = py as usize;
                        if px < self.width && py < self.height {
                            self.pixels[py * self.width + px] = color;
                        }
                    }
                }
            }
        }
    }

    fn close_handle(handle: u64) -> Result<()> {
        let mut ret = u64::MAX;
        for _ in 0..64 {
            ret = unsafe { sys::syscall1(sys::GUI_CLOSE, handle) };
            if ret != u64::MAX {
                break;
            }
            process::sleep_ms(1);
        }
        Error::from_ret(ret).map(|_| ())
    }

    fn parse_event(packet: &[u8; EVENT_PACKET_SIZE]) -> Result<Event> {
        match packet[0] {
            EVENT_KIND_KEY_CHAR => {
                let len = packet[1] as usize;
                if len == 0 || len > 4 {
                    return Err(Error::Invalid);
                }
                let mut bytes = [0u8; 4];
                bytes[..len].copy_from_slice(&packet[2..2 + len]);
                Ok(Event::KeyChar { bytes, len })
            }
            EVENT_KIND_MOUSE_DOWN => Ok(Event::MouseDown {
                button: packet[1],
                x: u16::from_le_bytes([packet[2], packet[3]]),
                y: u16::from_le_bytes([packet[4], packet[5]]),
            }),
            EVENT_KIND_CLOSE => Ok(Event::Close),
            EVENT_KIND_RESIZE => Ok(Event::Resize {
                width: u16::from_le_bytes([packet[2], packet[3]]),
                height: u16::from_le_bytes([packet[4], packet[5]]),
            }),
            _ => Err(Error::Invalid),
        }
    }
}

pub mod prelude {
    pub use crate::args::Args;
    pub use crate::event::{read_event, Event, INPUT_FD};
    pub use crate::io::{close, open, pipe, read, write, write_all, write_stdout, File};
    pub use crate::memory::mmap;
    pub use crate::process::{
        abi_version, exit, get_process_group, getpid, set_process_group, signal, signal_group,
        sleep_ms, spawn, waitpid, yield_now, Signal,
    };
    pub use crate::{entry, print, println, Error, Result, ABI_VERSION, SDK_VERSION};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::io::_print(core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::io::write_stdout(b"\n");
    }};
    ($fmt:expr) => {{
        $crate::print!(concat!($fmt, "\n"));
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!(concat!($fmt, "\n"), $($arg)*);
    }};
}

#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub extern "C" fn _start() -> ! {
            core::arch::naked_asm!(
                "mov rdi, rsp",
                "jmp {entry}",
                entry = sym __libcool_entry,
            );
        }

        extern "C" fn __libcool_entry(rsp: u64) -> ! {
            let args = unsafe { $crate::Args::from_stack(rsp) };
            $main(args)
        }

        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo) -> ! {
            $crate::process::abort()
        }
    };
}
