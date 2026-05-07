#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(path) = args.get(1) else {
        io::write_stderr(b"usage: stat <path>\n");
        exit(1);
    };
    match fs::stat(path) {
        Ok(meta) => {
            io::write_stdout(b"kind=");
            io::write_stdout(match meta.kind {
                fs::FileKind::File => &b"file"[..],
                fs::FileKind::Directory => &b"dir"[..],
                fs::FileKind::Missing => &b"missing"[..],
                fs::FileKind::Other => &b"other"[..],
            });
            io::write_stdout(b" size=");
            libcool::io::write_u64(meta.size);
            io::write_stdout(b" uid=");
            libcool::io::write_u64(meta.uid);
            io::write_stdout(b" gid=");
            libcool::io::write_u64(meta.gid);
            io::write_stdout(b" mode=");
            write_octal(meta.mode);
            io::write_stdout(b"\n");
            exit(0);
        }
        Err(_) => {
            io::write_stderr(b"stat: failed\n");
            exit(1);
        }
    }
}

fn write_octal(mut value: u64) {
    let mut buf = [0u8; 8];
    let mut len = 0usize;
    if value == 0 {
        io::write_stdout(b"0");
        return;
    }
    while value > 0 && len < buf.len() {
        buf[len] = b'0' + (value & 7) as u8;
        value >>= 3;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        let _ = io::write_byte(io::STDOUT, buf[len]);
    }
}
