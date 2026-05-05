#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("read: opening /bin/motd.txt");

    let file = match File::open(b"/bin/motd.txt") {
        Ok(file) => file,
        Err(_) => {
            println!("read: syscall failed");
            exit(1);
        }
    };

    let mut buf = [0u8; 64];
    match file.read(&mut buf) {
        Ok(n) => {
            write_stdout(&buf[..n]);
            file.close();
            exit(0);
        }
        Err(_) => {
            file.close();
            println!("read: syscall failed");
            exit(1);
        }
    }
}
