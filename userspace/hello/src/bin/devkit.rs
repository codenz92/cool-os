#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    io::write_stdout(b"coolOS devkit\n");
    io::write_stdout(b"docs: /SDK/README.TXT\n");
    io::write_stdout(b"template: /SDK/APP_TEMPLATE.RS\n");
    io::write_stdout(b"package: /SDK/PACKAGE_TEMPLATE.PKG\n");
    io::write_stdout(b"build ABI: 8\n");
    exit(0);
}
