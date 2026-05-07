#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let fd = if let Some(path) = args.get(1) {
        match io::File::open(path) {
            Ok(file) => {
                let fd = file.fd();
                core::mem::forget(file);
                fd
            }
            Err(_) => {
                io::write_stderr(b"head: open failed\n");
                exit(1);
            }
        }
    } else {
        io::STDIN
    };
    let mut buf = [0u8; 128];
    let mut lines = 0usize;
    while lines < 10 {
        let n = match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => exit(1),
        };
        for &byte in &buf[..n] {
            let _ = io::write_byte(io::STDOUT, byte);
            if byte == b'\n' {
                lines += 1;
                if lines >= 10 {
                    break;
                }
            }
        }
    }
    if fd != io::STDIN {
        io::close(fd);
    }
    exit(0);
}
