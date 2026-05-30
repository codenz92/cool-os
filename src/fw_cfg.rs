extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU8, Ordering};
use x86_64::instructions::port::Port;

const FW_CFG_SELECTOR: u16 = 0x510;
const FW_CFG_DATA: u16 = 0x511;
const FW_CFG_SIGNATURE: u16 = 0x0000;
const FW_CFG_FILE_DIR: u16 = 0x0019;
const SMOKE_COMMAND_FILE: &[u8] = b"opt/coolos/smoke";
const SMOKE_MODE_FILE: &[u8] = b"opt/coolos/smoke-mode";
const MAX_SMOKE_COMMAND: usize = 256;
static SMOKE_MODE_CACHE: AtomicU8 = AtomicU8::new(0);

pub fn smoke_command() -> Option<String> {
    if !has_qemu_signature() {
        return None;
    }
    let bytes = read_named_file(SMOKE_COMMAND_FILE)?;
    let mut command = String::from_utf8(bytes).ok()?;
    while command.ends_with('\0') || command.ends_with('\n') || command.ends_with('\r') {
        command.pop();
    }
    if command.trim().is_empty() {
        None
    } else {
        Some(command)
    }
}

pub fn smoke_commands() -> Vec<String> {
    let Some(command) = smoke_command() else {
        return Vec::new();
    };
    let mut commands = Vec::new();
    for chunk in command.split(";;") {
        for line in chunk.lines() {
            let line = line.trim();
            if !line.is_empty() {
                commands.push(String::from(line));
            }
        }
    }
    commands
}

pub fn smoke_mode() -> bool {
    match SMOKE_MODE_CACHE.load(Ordering::Relaxed) {
        1 => return false,
        2 => return true,
        _ => {}
    }
    let active = has_qemu_signature()
        && read_named_file(SMOKE_MODE_FILE)
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .map(|value| {
                let value = value.trim_matches(char::from(0)).trim();
                value == "1" || value.eq_ignore_ascii_case("true")
            })
            .unwrap_or(false);
    SMOKE_MODE_CACHE.store(if active { 2 } else { 1 }, Ordering::Relaxed);
    active
}

fn read_named_file(name: &[u8]) -> Option<Vec<u8>> {
    select(FW_CFG_FILE_DIR);
    let count = read_be_u32().min(64);
    for _ in 0..count {
        let size = read_be_u32() as usize;
        let selector = read_be_u16();
        let _reserved = read_be_u16();
        let mut entry_name = [0u8; 56];
        for byte in entry_name.iter_mut() {
            *byte = read_u8();
        }
        let entry_len = entry_name
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(entry_name.len());
        if &entry_name[..entry_len] == name {
            return Some(read_file(selector, size.min(MAX_SMOKE_COMMAND)));
        }
    }
    None
}

fn read_file(selector: u16, size: usize) -> Vec<u8> {
    select(selector);
    let mut out = Vec::new();
    for _ in 0..size {
        out.push(read_u8());
    }
    out
}

fn has_qemu_signature() -> bool {
    select(FW_CFG_SIGNATURE);
    let signature = [read_u8(), read_u8(), read_u8(), read_u8()];
    signature == *b"QEMU"
}

fn select(selector: u16) {
    unsafe {
        Port::<u16>::new(FW_CFG_SELECTOR).write(selector);
    }
}

fn read_u8() -> u8 {
    unsafe { Port::<u8>::new(FW_CFG_DATA).read() }
}

fn read_be_u16() -> u16 {
    let hi = read_u8() as u16;
    let lo = read_u8() as u16;
    (hi << 8) | lo
}

fn read_be_u32() -> u32 {
    let b0 = read_u8() as u32;
    let b1 = read_u8() as u32;
    let b2 = read_u8() as u32;
    let b3 = read_u8() as u32;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}
