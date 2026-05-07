#![no_std]
#![no_main]

use libcool::{io, prelude::*};

const BUF: usize = 4096;

libcool::entry!(main);

fn main(args: Args) -> ! {
    let Some(path) = args.get(1) else {
        io::write_stderr(b"usage: tail <file>\n");
        exit(1);
    };
    let file = match io::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            io::write_stderr(b"tail: open failed\n");
            exit(1);
        }
    };
    let mut data = [0u8; BUF];
    let mut len = 0usize;
    loop {
        let n = match file.read(&mut data[len..]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                file.close();
                exit(1);
            }
        };
        len += n;
        if len == data.len() {
            break;
        }
    }
    file.close();
    let mut start = 0usize;
    let mut lines = 0usize;
    let mut idx = len;
    while idx > 0 {
        idx -= 1;
        if data[idx] == b'\n' && idx + 1 < len {
            lines += 1;
            if lines >= 10 {
                start = idx + 1;
                break;
            }
        }
    }
    let _ = io::write_all(io::STDOUT, &data[start..len]);
    exit(0);
}
