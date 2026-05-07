#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    match fs::sync() {
        Ok(()) => {
            io::write_stdout(b"sync: ok\n");
            exit(0);
        }
        Err(_) => {
            io::write_stderr(b"sync: failed\n");
            exit(1);
        }
    }
}
