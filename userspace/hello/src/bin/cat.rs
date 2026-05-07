#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    if args.len() < 2 {
        io::write_stderr(b"usage: cat <path>...\n");
        exit(1);
    }
    let mut status = 0u64;
    for idx in 1..args.len() {
        if let Some(path) = args.get(idx) {
            if !cat_one(path) {
                status = 1;
            }
        }
    }
    exit(status);
}

fn cat_one(path: &[u8]) -> bool {
    match io::File::open(path) {
        Ok(file) => {
            let mut buf = [0u8; 512];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = io::write_all(io::STDOUT, &buf[..n]);
                    }
                    Err(_) => {
                        io::write_stderr(b"cat: read failed\n");
                        file.close();
                        return false;
                    }
                }
            }
            file.close();
            true
        }
        Err(_) => {
            io::write_stderr(b"cat: open failed\n");
            false
        }
    }
}
