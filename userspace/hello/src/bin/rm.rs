#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(path) = args.get(1) else {
        io::write_stderr(b"usage: rm <path>\n");
        exit(1);
    };
    match fs::delete_tree(path) {
        Ok(()) => exit(0),
        Err(_) => {
            io::write_stderr(b"rm: failed\n");
            exit(1);
        }
    }
}
