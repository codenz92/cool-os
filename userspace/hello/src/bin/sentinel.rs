#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(args: Args) -> ! {
    let pid = match args.get(1) {
        Some(b"1") => 1u64,
        Some(b"2") => 2u64,
        _ => 0u64,
    };
    let sentinel = 0xDEAD_0000 + pid;
    let stack_top_ptr = (0x0000_7fff_0010_0000u64 - 8) as *mut u64;

    unsafe {
        core::ptr::write_volatile(stack_top_ptr, sentinel);
    }
    let readback = unsafe { core::ptr::read_volatile(stack_top_ptr) };

    if readback == sentinel && pid == 1 {
        write_stdout(b"[ring3 pid=1] sentinel ok\n");
        exit(0);
    }
    if readback == sentinel && pid == 2 {
        write_stdout(b"[ring3 pid=2] sentinel ok\n");
        exit(0);
    }

    write_stdout(b"[ring3 pid=?] sentinel failed\n");
    exit(1);
}
