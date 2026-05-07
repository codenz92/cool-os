#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    let mut cwd = [0u8; 160];
    match fs::getcwd(&mut cwd) {
        Ok(n) => {
            let _ = io::write_all(io::STDOUT, &cwd[..n]);
            io::write_stdout(b"\n");
            exit(0);
        }
        Err(_) => {
            io::write_stderr(b"pwd: failed\n");
            exit(1);
        }
    }
}
