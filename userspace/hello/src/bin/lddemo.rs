#![no_std]
#![no_main]

use core::mem;

use libcool::dynlink;
use libcool::prelude::*;

libcool::entry!(main);

static mut IMAGE: [u8; dynlink::MAX_IMAGE_BYTES] = [0; dynlink::MAX_IMAGE_BYTES];
static mut WORKSPACE: dynlink::Workspace = dynlink::Workspace::new();

fn main(_args: Args) -> ! {
    println!("lddemo: abi={}", abi_version());
    phase75_single_object();
    phase76_dependency_graph();
    exit(0);
}

fn phase75_single_object() {
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
}

fn phase76_dependency_graph() {
    let set = match dynlink::load_with_deps(
        b"/lib/libphase76main.so",
        unsafe { &mut *core::ptr::addr_of_mut!(WORKSPACE) },
        dynlink::DEFAULT_LOAD_BASE + 0x0100_0000,
    ) {
        Ok(set) => set,
        Err(_) => {
            println!("lddemo: phase76 load failed");
            exit(5);
        }
    };
    println!(
        "lddemo: phase76 objects={} deps={} rela={} init={} tls={}",
        set.object_count(),
        set.dependency_count(),
        set.relocation_count(),
        set.init_count(),
        set.tls_bytes()
    );

    let dep = match set.object(0) {
        Some(object) => object,
        None => {
            println!("lddemo: phase76 dep missing");
            exit(6);
        }
    };
    println!(
        "lddemo: phase76 dep module={} tls={}",
        dep.tls_module_id(),
        dep.tls_bytes()
    );

    let run_addr = match set.symbol(b"phase76_run") {
        Ok(addr) => addr,
        Err(_) => {
            println!("lddemo: phase76_run missing");
            exit(7);
        }
    };
    let tls_addr = match set.symbol(b"phase76_tls_counter") {
        Ok(addr) => addr,
        Err(_) => {
            println!("lddemo: phase76_tls_counter missing");
            exit(8);
        }
    };
    let phase76_run: extern "C" fn() -> u64 = unsafe { mem::transmute(run_addr as usize) };
    let result = phase76_run();
    let tls = unsafe { *(tls_addr as *const u64) };
    println!(
        "lddemo: phase76 result={} tls={} run={:#x}",
        result, tls, run_addr
    );

    if result != 72 || tls != 23 || dep.tls_module_id() != 1 || dep.tls_bytes() != 16 {
        println!("lddemo: phase76 mismatch");
        exit(9);
    }
    println!("lddemo: phase76 ok");
}
