#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(path) = args.get(1) else {
        io::write_stderr(b"usage: writefile <path> <text...>\n");
        exit(1);
    };
    let mut data = [0u8; 512];
    let mut len = 0usize;
    for idx in 2..args.len() {
        if idx > 2 {
            if len >= data.len() {
                break;
            }
            data[len] = b' ';
            len += 1;
        }
        if let Some(arg) = args.get(idx) {
            for &byte in arg {
                if len >= data.len() {
                    break;
                }
                data[len] = byte;
                len += 1;
            }
        }
    }
    match fs::write_file(path, &data[..len]) {
        Ok(()) => exit(0),
        Err(_) => {
            io::write_stderr(b"writefile: failed\n");
            exit(1);
        }
    }
}
