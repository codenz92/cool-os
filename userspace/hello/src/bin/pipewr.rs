#![no_std]
#![no_main]

use libcool::prelude::*;

const PIPE_FD: u64 = 3;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("pipewr: spinning before write");

    for _ in 0..100_000u64 {
        core::hint::spin_loop();
    }

    if write_all(PIPE_FD, b"hello from user writer\n").is_err() {
        println!("pipewr: write failed");
        exit(1);
    }

    println!("pipewr: write ok");
    close(PIPE_FD);
    exit(0);
}
