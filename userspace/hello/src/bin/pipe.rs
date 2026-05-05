#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("pipe: creating anonymous pipe");
    let (read_fd, write_fd) = match pipe() {
        Ok(fds) => fds,
        Err(_) => {
            println!("pipe: syscall failed");
            exit(1);
        }
    };

    if write_all(write_fd, b"hello through pipe\n").is_err() {
        println!("pipe: syscall failed");
        exit(1);
    }

    let mut buf = [0u8; 32];
    match read(read_fd, &mut buf) {
        Ok(n) => {
            write_stdout(&buf[..n]);
            close(read_fd);
            close(write_fd);
            exit(0);
        }
        Err(_) => {
            println!("pipe: syscall failed");
            exit(1);
        }
    }
}
