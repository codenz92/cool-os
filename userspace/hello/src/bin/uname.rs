#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    io::write_stdout(b"coolOS coolOS-userspace-abi/8 x86_64\n");
    exit(0);
}
