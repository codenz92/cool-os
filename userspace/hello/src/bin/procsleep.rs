#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(args: Args) -> ! {
    let pid = getpid();
    let pgid = get_process_group(0).unwrap_or(0);
    println!("procsleep: pid={} pgid={} ready", pid, pgid);

    let rounds = if args.get(1) == Some(b"short") {
        8
    } else {
        240
    };
    for _ in 0..rounds {
        sleep_ms(250);
    }

    println!("procsleep: done");
    exit(0);
}
