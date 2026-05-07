#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    for idx in 1..args.len() {
        if idx > 1 {
            io::write_stdout(b" ");
        }
        if let Some(arg) = args.get(idx) {
            io::write_stdout(arg);
        }
    }
    println!();
    exit(0);
}
