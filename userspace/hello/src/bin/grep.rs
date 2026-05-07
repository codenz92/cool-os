#![no_std]
#![no_main]

use libcool::{io, prelude::*};

const LINE_MAX: usize = 256;

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(pattern) = args.get(1) else {
        io::write_stderr(b"usage: grep <pattern> [file...]\n");
        exit(1);
    };
    let mut matched = false;
    if args.len() <= 2 {
        matched = grep_fd(pattern, io::STDIN);
    } else {
        for idx in 2..args.len() {
            if let Some(path) = args.get(idx) {
                match io::File::open(path) {
                    Ok(file) => {
                        matched |= grep_fd(pattern, file.fd());
                        file.close();
                    }
                    Err(_) => io::write_stderr(b"grep: open failed\n"),
                }
            }
        }
    }
    exit(if matched { 0 } else { 1 });
}

fn grep_fd(pattern: &[u8], fd: u64) -> bool {
    let mut buf = [0u8; 128];
    let mut line = [0u8; LINE_MAX];
    let mut len = 0usize;
    let mut matched = false;
    loop {
        let n = match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return matched,
        };
        for &byte in &buf[..n] {
            if len < line.len() {
                line[len] = byte;
                len += 1;
            }
            if byte == b'\n' {
                if contains(&line[..len], pattern) {
                    let _ = io::write_all(io::STDOUT, &line[..len]);
                    matched = true;
                }
                len = 0;
            }
        }
    }
    if len > 0 && contains(&line[..len], pattern) {
        let _ = io::write_all(io::STDOUT, &line[..len]);
        io::write_stdout(b"\n");
        matched = true;
    }
    matched
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    let mut i = 0usize;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            return true;
        }
        i += 1;
    }
    false
}
