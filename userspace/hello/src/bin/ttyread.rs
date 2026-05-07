#![no_std]
#![no_main]

use libcool::{io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("ttyread: ready");

    let mut buf = [0u8; 128];
    match io::read(io::STDIN, &mut buf) {
        Ok(0) => {
            println!("ttyread: eof");
            exit(0);
        }
        Ok(n) => {
            io::write_stdout(b"ttyread: got ");
            let _ = io::write_all(io::STDOUT, &buf[..n]);
            if buf[n - 1] != b'\n' {
                println!();
            }
            exit(0);
        }
        Err(_) => {
            println!("ttyread: read failed");
            exit(1);
        }
    }
}
