#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    for _ in 0..28 {
        io::write_stdout(b"\n");
    }
    exit(0);
}
