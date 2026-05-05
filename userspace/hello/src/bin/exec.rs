#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("exec: replacing self with /bin/hello");
    if libcool::process::exec(b"/bin/hello").is_err() {
        println!("exec: sys_exec failed");
        exit(1);
    }
    libcool::process::abort()
}
