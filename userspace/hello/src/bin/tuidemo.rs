#![no_std]
#![no_main]

use libcool::{evented, io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("tuidemo: abi={}", abi_version());
    let size = tty::size().unwrap_or(tty::Size { cols: 80, rows: 25 });
    let previous_mode = match tty::enter_raw_mode() {
        Ok(mode) => mode,
        Err(_) => {
            println!("tuidemo: raw mode failed");
            exit(1);
        }
    };

    io::write_stdout(b"\x1b[2J\x1b[H");
    io::write_stdout(b"\x1b[96mtuidemo: raw ready\x1b[0m ");
    println!("cols={} rows={}", size.cols, size.rows);
    io::write_stdout(b"\x1b[2;1H\x1b[32mpress q to exit without Enter\x1b[0m");
    io::write_stdout(b"\x1b[3;1Hkeys: ");

    let mut exit_key = 0u8;
    let mut buf = [0u8; 16];
    loop {
        if !evented::wait_fd_read(io::STDIN, evented::TIMEOUT_FOREVER).unwrap_or(false) {
            continue;
        }
        let n = io::read(io::STDIN, &mut buf).unwrap_or(0);
        if n == 0 {
            exit_key = b'e';
            break;
        }
        for &byte in &buf[..n] {
            if byte == b'q' || byte == 0x03 {
                exit_key = byte;
                break;
            }
            echo_key(byte);
        }
        if exit_key != 0 {
            break;
        }
    }

    let _ = tty::restore_mode(previous_mode);
    io::write_stdout(b"\x1b[5;1H\x1b[0m");
    if exit_key == 0x03 {
        println!("tuidemo: raw exit key=ctrl-c");
    } else if exit_key == b'e' {
        println!("tuidemo: raw exit key=eof");
    } else {
        println!("tuidemo: raw exit key={}", exit_key as char);
    }
    println!("tuidemo: done");
    exit(0);
}

fn echo_key(byte: u8) {
    if byte == b'\x1b' {
        io::write_stdout(b"<esc>");
    } else if byte < 0x20 {
        io::write_stdout(b"^");
        let marker = [byte + b'@'];
        io::write_stdout(&marker);
    } else if byte == 0x7f {
        io::write_stdout(b"<del>");
    } else {
        let out = [byte];
        io::write_stdout(&out);
    }
    io::write_stdout(b" ");
}
