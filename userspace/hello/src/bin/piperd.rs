#![no_std]
#![no_main]

use libcool::prelude::*;

const PIPE_FD: u64 = 3;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("piperd: waiting on shared pipe");

    let mut buf = [0u8; 64];
    match read(PIPE_FD, &mut buf) {
        Ok(n) => {
            write_stdout(b"piperd: got ");
            write_stdout(&buf[..n]);
            close(PIPE_FD);
            exit(0);
        }
        Err(_) => {
            println!("piperd: read failed");
            exit(1);
        }
    }
}
