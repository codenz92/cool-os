#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(path) = args.get(1) else {
        io::write_stderr(b"usage: mkdir <path>\n");
        exit(1);
    };
    match fs::create_dir(path) {
        Ok(()) => exit(0),
        Err(_) => {
            io::write_stderr(b"mkdir: failed\n");
            exit(1);
        }
    }
}
