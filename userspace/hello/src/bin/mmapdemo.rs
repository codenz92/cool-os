#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

const TMP_PATH: &[u8] = b"/TMP/MMAPDEMO.TXT";
const MOTD_PATH: &[u8] = b"/bin/motd.txt";
const DSO_PATH: &[u8] = b"/lib/libphase75.so";
const TMP_TEXT: &[u8] = b"phase77 tmp file-backed mmap ok\n";
const TMP_ADDR: u64 = 0x0000_7fff_1800_0000;
const MOTD_ADDR: u64 = 0x0000_7fff_1801_0000;
const EXEC_ADDR: u64 = 0x0000_7fff_1802_0000;
const DENY_ADDR: u64 = 0x0000_7fff_1803_0000;

fn main(_args: Args) -> ! {
    println!("mmapdemo: abi={}", abi_version());
    write_tmp_file();
    map_tmp_file();
    map_motd();
    deny_write_mapping();
    map_exec_file();
    println!("mmapdemo: phase77 ok");
    exit(0);
}

fn write_tmp_file() {
    let file = match File::open_flags(TMP_PATH, O_WRONLY | O_CREAT | O_TRUNC) {
        Ok(file) => file,
        Err(_) => {
            println!("mmapdemo: tmp create failed");
            exit(1);
        }
    };
    match file.write(TMP_TEXT) {
        Ok(n) if n == TMP_TEXT.len() => {}
        _ => {
            println!("mmapdemo: tmp write failed");
            exit(2);
        }
    }
    file.close();
}

fn map_tmp_file() {
    let file = open_readonly(TMP_PATH, 3);
    let addr = match mmap_file(file.fd(), TMP_ADDR, 4096, 0, 0) {
        Ok(addr) => addr,
        Err(_) => {
            println!("mmapdemo: tmp map failed");
            exit(4);
        }
    };
    let mapped = unsafe { core::slice::from_raw_parts(addr as *const u8, TMP_TEXT.len()) };
    if mapped != TMP_TEXT {
        println!("mmapdemo: tmp verify failed");
        exit(5);
    }
    println!("mmapdemo: tmp roundtrip ok");
    file.close();
}

fn map_motd() {
    let file = open_readonly(MOTD_PATH, 6);
    let addr = match mmap_file(file.fd(), MOTD_ADDR, 4096, 0, 0) {
        Ok(addr) => addr,
        Err(_) => {
            println!("mmapdemo: motd map failed");
            exit(7);
        }
    };
    let mapped = unsafe { core::slice::from_raw_parts(addr as *const u8, 64) };
    if !starts_with(mapped, b"coolOS Phase") {
        println!("mmapdemo: motd verify failed");
        exit(8);
    }
    println!("mmapdemo: file map /bin/motd.txt");
    println!("mmapdemo: mapped text coolOS Phase");
    file.close();
}

fn deny_write_mapping() {
    let file = open_readonly(TMP_PATH, 9);
    if mmap_file(file.fd(), DENY_ADDR, 4096, 0, PROT_WRITE).is_ok() {
        println!("mmapdemo: write-map unexpectedly allowed");
        exit(10);
    }
    println!("mmapdemo: write-map denied");
    file.close();
}

fn map_exec_file() {
    let file = open_readonly(DSO_PATH, 11);
    let addr = match mmap_file(file.fd(), EXEC_ADDR, 4096, 0, PROT_EXEC) {
        Ok(addr) => addr,
        Err(_) => {
            println!("mmapdemo: exec-map failed");
            exit(12);
        }
    };
    let mapped = unsafe { core::slice::from_raw_parts(addr as *const u8, 4) };
    if mapped != b"\x7fELF" {
        println!("mmapdemo: exec-map verify failed");
        exit(13);
    }
    println!("mmapdemo: exec-map ok");
    file.close();
}

fn open_readonly(path: &[u8], code: u64) -> File {
    match File::open_flags(path, O_RDONLY) {
        Ok(file) => file,
        Err(_) => {
            println!("mmapdemo: open failed");
            exit(code);
        }
    }
}

fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && &haystack[..needle.len()] == needle
}
