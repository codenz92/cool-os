#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    print!("coolOS coolOS-userspace-abi/{} x86_64\n", abi_version());
    exit(0);
}
