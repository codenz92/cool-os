#![no_std]
#![no_main]

use libcool::{io, net, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    let host = b"example.com";

    print!("netdemo: dns example.com = ");
    let addr = match net::dns_resolve(host) {
        Ok(addr) => addr,
        Err(_) => {
            println!("failed");
            exit(1);
        }
    };
    io::write_ipv4(addr);
    println!();

    println!("netdemo: http example.com");
    match net::http_get(host) {
        Ok(bytes) => {
            println!("netdemo: http bytes {}", bytes);
            exit(0);
        }
        Err(_) => {
            println!("netdemo: http failed");
            exit(1);
        }
    }
}
