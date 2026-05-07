#![no_std]
#![no_main]

use libcool::{event, evented, io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("keyecho: ready");

    loop {
        let _ = evented::wait_fd_read(INPUT_FD, evented::TIMEOUT_FOREVER);
        match event::read_event(INPUT_FD) {
            Ok(Some(Event::KeyChar { bytes, len })) => {
                io::write_stdout(&bytes[..len]);
            }
            Ok(Some(Event::MouseDown { x, y })) => {
                print!("\nclick {},{}", x, y);
            }
            Ok(None) => break,
            Err(_) => {
                println!("keyecho: bad event");
                exit(1);
            }
        }
    }

    close(INPUT_FD);
    println!("\nkeyecho: eof");
    exit(0);
}
