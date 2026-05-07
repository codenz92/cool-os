#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    let path = args.get(1).unwrap_or(b"/");
    let mut out = [0u8; 4096];
    match fs::list_dir(path, &mut out) {
        Ok(n) => {
            let _ = io::write_all(io::STDOUT, &out[..n]);
            exit(0);
        }
        Err(_) => {
            io::write_stderr(b"ls: failed\n");
            exit(1);
        }
    }
}
