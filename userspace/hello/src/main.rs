#![no_std]
#![no_main]

use libcool::{io, ipc, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    test_shmem();

    let prefix = if args.len() == 1 {
        "Hello from "
    } else {
        "Hello with bad argc from "
    };
    io::write_stdout(prefix.as_bytes());
    io::write_stdout(args.program().unwrap_or(b"/bin/hello"));
    println!();

    exit(0);
}

fn test_shmem() {
    print!("shmem_create(8192) = ");
    match ipc::shmem_create(8192) {
        Ok(id) => {
            println!("{:#x}", id);
            print!("shmem_map() = ");
            match ipc::shmem_map(id) {
                Ok(addr) => println!("{:#x}", addr),
                Err(_) => println!("FAILED"),
            }
        }
        Err(_) => println!("FAILED"),
    }
}
