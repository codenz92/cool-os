#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let (Some(src), Some(dst)) = (args.get(1), args.get(2)) else {
        io::write_stderr(b"usage: cp <src> <dst>\n");
        exit(1);
    };
    let src_file = match io::File::open(src) {
        Ok(file) => file,
        Err(_) => {
            io::write_stderr(b"cp: open src failed\n");
            exit(1);
        }
    };
    let dst_file = match io::File::create(dst) {
        Ok(file) => file,
        Err(_) => {
            src_file.close();
            io::write_stderr(b"cp: open dst failed\n");
            exit(1);
        }
    };
    let mut buf = [0u8; 512];
    loop {
        match src_file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if dst_file.write(&buf[..n]).unwrap_or(0) != n {
                    io::write_stderr(b"cp: write failed\n");
                    src_file.close();
                    dst_file.close();
                    exit(1);
                }
            }
            Err(_) => {
                io::write_stderr(b"cp: read failed\n");
                src_file.close();
                dst_file.close();
                exit(1);
            }
        }
    }
    src_file.close();
    dst_file.close();
    exit(0);
}
