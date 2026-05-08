#![no_std]
#![no_main]

use core::mem;

use libcool::dynlink;
use libcool::prelude::*;

libcool::entry!(main);

static mut IMAGE: [u8; dynlink::MAX_IMAGE_BYTES] = [0; dynlink::MAX_IMAGE_BYTES];

fn main(_args: Args) -> ! {
    println!("lddemo: abi={}", abi_version());

    let image = unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(IMAGE).cast::<u8>(),
            dynlink::MAX_IMAGE_BYTES,
        )
    };
    let object = match dynlink::load(b"/lib/libphase75.so", image, dynlink::DEFAULT_LOAD_BASE) {
        Ok(object) => object,
        Err(_) => {
            println!("lddemo: load failed");
            exit(1);
        }
    };
    println!(
        "lddemo: loaded /lib/libphase75.so base={:#x} loads={} rela={} init={}",
        object.base(),
        object.load_count(),
        object.relocation_count(),
        object.init_count()
    );

    let add_addr = match object.symbol(b"phase75_add") {
        Ok(addr) => addr,
        Err(_) => {
            println!("lddemo: phase75_add missing");
            exit(2);
        }
    };
    let increment_addr = match object.symbol(b"phase75_increment") {
        Ok(addr) => addr,
        Err(_) => {
            println!("lddemo: phase75_increment missing");
            exit(3);
        }
    };

    let phase75_add: extern "C" fn(u64, u64) -> u64 = unsafe { mem::transmute(add_addr as usize) };
    let result = phase75_add(30, 3);
    let increment = unsafe { *(increment_addr as *const u64) };
    println!(
        "lddemo: symbol phase75_add={:#x} increment={} result={}",
        add_addr, increment, result
    );

    if increment != 9 || result != 42 {
        println!("lddemo: result mismatch");
        exit(4);
    }

    println!("lddemo: phase75 ok");
    exit(0);
}
