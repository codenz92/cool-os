#![no_std]
#![no_main]

use libcool::{io, ipc, prelude::*};

libcool::entry!(main);

fn main(args: Args) -> ! {
    println!("sdkdemo: libcool sdk={} abi={}", SDK_VERSION, abi_version());
    print!("sdkdemo: argv");
    for idx in 0..args.len() {
        io::write_stdout(b" [");
        io::write_u64(idx as u64);
        io::write_stdout(b"]=");
        io::write_stdout(args.get(idx).unwrap_or(b"?"));
    }
    println!();

    let mut buf = [0u8; 64];
    match File::open(b"/bin/motd.txt").and_then(|file| {
        let n = file.read(&mut buf)?;
        file.close();
        Ok(n)
    }) {
        Ok(n) => {
            io::write_stdout(b"sdkdemo: read ");
            io::write_stdout(&buf[..n]);
        }
        Err(_) => println!("sdkdemo: read failed"),
    }

    match pipe() {
        Ok((r, w)) => {
            let _ = write_all(w, b"sdk pipe ok\n");
            let mut pipe_buf = [0u8; 32];
            match read(r, &mut pipe_buf) {
                Ok(n) => {
                    io::write_stdout(b"sdkdemo: ");
                    io::write_stdout(&pipe_buf[..n]);
                }
                Err(_) => println!("sdkdemo: pipe read failed"),
            }
            close(r);
            close(w);
        }
        Err(_) => println!("sdkdemo: pipe failed"),
    }

    match ipc::shmem_create(4096).and_then(ipc::shmem_map) {
        Ok(addr) => println!("sdkdemo: shmem {:#x}", addr),
        Err(_) => println!("sdkdemo: shmem failed"),
    }

    match mmap(0x0000_7fff_1000_0000, 4096, true) {
        Ok(addr) => unsafe {
            let ptr = addr as *mut u64;
            core::ptr::write_volatile(ptr, 0xC001_CAFE);
            if core::ptr::read_volatile(ptr) == 0xC001_CAFE {
                println!("sdkdemo: mmap ok");
            } else {
                println!("sdkdemo: mmap bad");
            }
        },
        Err(_) => println!("sdkdemo: mmap failed"),
    }

    println!("sdkdemo: done");
    exit(0);
}
