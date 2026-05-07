#![no_std]
#![no_main]

use libcool::{io, prelude::*, time};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    match time::now() {
        Some(dt) => {
            write_u16(dt.year);
            io::write_stdout(b"-");
            write_2(dt.month);
            io::write_stdout(b"-");
            write_2(dt.day);
            io::write_stdout(b" ");
            write_2(dt.hour);
            io::write_stdout(b":");
            write_2(dt.minute);
            io::write_stdout(b"\n");
            exit(0);
        }
        None => {
            io::write_stderr(b"date: rtc unavailable\n");
            exit(1);
        }
    }
}

fn write_2(value: u8) {
    let tens = value / 10;
    let ones = value % 10;
    let _ = io::write_byte(io::STDOUT, b'0' + tens);
    let _ = io::write_byte(io::STDOUT, b'0' + ones);
}

fn write_u16(value: u16) {
    libcool::io::write_u64(value as u64);
}
