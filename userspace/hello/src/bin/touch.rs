#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(path) = args.get(1) else {
        io::write_stderr(b"usage: touch <path>\n");
        exit(1);
    };
    match fs::write_file(path, b"") {
        Ok(()) => exit(0),
        Err(_) => {
            io::write_stderr(b"touch: failed\n");
            exit(1);
        }
    }
}
