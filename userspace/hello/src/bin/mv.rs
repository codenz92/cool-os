#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let (Some(src), Some(dst)) = (args.get(1), args.get(2)) else {
        io::write_stderr(b"usage: mv <src> <dst>\n");
        exit(1);
    };
    match fs::rename(src, dst) {
        Ok(()) => exit(0),
        Err(_) => {
            io::write_stderr(b"mv: failed\n");
            exit(1);
        }
    }
}
