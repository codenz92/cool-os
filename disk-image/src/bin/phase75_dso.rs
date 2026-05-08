use std::path::Path;

const FILE_LEN: usize = 0x3018;
const EHDR_SIZE: usize = 64;
const PHDR_OFF: usize = 0x40;
const PHDR_SIZE: usize = 56;
const PHDR_COUNT: usize = 4;

const DYN_OFF: usize = 0x200;
const DYN_COUNT: usize = 11;
const HASH_OFF: usize = 0x300;
const SYMTAB_OFF: usize = 0x340;
const STRTAB_OFF: usize = 0x3a0;
const RELA_OFF: usize = 0x400;
const TEXT_OFF: usize = 0x1000;
const DATA_OFF: usize = 0x3000;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

const DT_NULL: i64 = 0;
const DT_HASH: i64 = 4;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_STRSZ: i64 = 10;
const DT_SYMENT: i64 = 11;
const DT_INIT_ARRAY: i64 = 25;
const DT_INIT_ARRAYSZ: i64 = 27;

const R_X86_64_RELATIVE: u32 = 8;

fn main() {
    let out_path = std::env::args()
        .nth(1)
        .expect("usage: phase75-dso <output.so>");
    let mut image = vec![0u8; FILE_LEN];

    write_elf_header(&mut image);
    write_program_headers(&mut image);
    write_dynamic(&mut image);
    write_hash(&mut image);
    write_symbols_and_strings(&mut image);
    write_relocations(&mut image);
    write_text(&mut image);
    write_data(&mut image);

    let path = Path::new(&out_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("failed to create {}: {}", parent.display(), e));
    }
    std::fs::write(path, image).unwrap_or_else(|e| panic!("failed to write {}: {}", out_path, e));
    println!("{}", out_path);
}

fn write_elf_header(image: &mut [u8]) {
    image[0..4].copy_from_slice(b"\x7fELF");
    image[4] = 2; // ELFCLASS64
    image[5] = 1; // ELFDATA2LSB
    image[6] = 1; // EV_CURRENT
    image[7] = 0; // System V ABI
    put_u16(image, 16, 3); // ET_DYN
    put_u16(image, 18, 62); // EM_X86_64
    put_u32(image, 20, 1);
    put_u64(image, 24, 0);
    put_u64(image, 32, PHDR_OFF as u64);
    put_u64(image, 40, 0);
    put_u32(image, 48, 0);
    put_u16(image, 52, EHDR_SIZE as u16);
    put_u16(image, 54, PHDR_SIZE as u16);
    put_u16(image, 56, PHDR_COUNT as u16);
    put_u16(image, 58, 0);
    put_u16(image, 60, 0);
    put_u16(image, 62, 0);
}

fn write_program_headers(image: &mut [u8]) {
    write_phdr(
        image,
        0,
        PT_LOAD,
        PF_R,
        0,
        0,
        (RELA_OFF + 2 * 24) as u64,
        0x1000,
        0x1000,
    );
    write_phdr(
        image,
        1,
        PT_LOAD,
        PF_R | PF_X,
        TEXT_OFF as u64,
        0x1000,
        0x40,
        0x1000,
        0x1000,
    );
    write_phdr(
        image,
        2,
        PT_LOAD,
        PF_R | PF_W,
        DATA_OFF as u64,
        0x3000,
        0x18,
        0x1000,
        0x1000,
    );
    write_phdr(
        image,
        3,
        PT_DYNAMIC,
        PF_R,
        DYN_OFF as u64,
        DYN_OFF as u64,
        (DYN_COUNT * 16) as u64,
        (DYN_COUNT * 16) as u64,
        8,
    );
}

fn write_dynamic(image: &mut [u8]) {
    let entries = [
        (DT_HASH, HASH_OFF as u64),
        (DT_STRTAB, STRTAB_OFF as u64),
        (DT_SYMTAB, SYMTAB_OFF as u64),
        (DT_STRSZ, 31),
        (DT_SYMENT, 24),
        (DT_RELA, RELA_OFF as u64),
        (DT_RELASZ, 2 * 24),
        (DT_RELAENT, 24),
        (DT_INIT_ARRAY, 0x3010),
        (DT_INIT_ARRAYSZ, 8),
        (DT_NULL, 0),
    ];
    for (idx, (tag, val)) in entries.iter().enumerate() {
        let off = DYN_OFF + idx * 16;
        put_i64(image, off, *tag);
        put_u64(image, off + 8, *val);
    }
}

fn write_hash(image: &mut [u8]) {
    let names = [b"phase75_add".as_slice(), b"phase75_increment".as_slice()];
    let mut buckets = [0u32; 4];
    let mut chains = [0u32; 3];
    for (idx, name) in names.iter().enumerate() {
        let sym_index = (idx + 1) as u32;
        let bucket = (elf_hash(name) % buckets.len() as u32) as usize;
        chains[sym_index as usize] = buckets[bucket];
        buckets[bucket] = sym_index;
    }

    put_u32(image, HASH_OFF, buckets.len() as u32);
    put_u32(image, HASH_OFF + 4, chains.len() as u32);
    for (idx, value) in buckets.iter().enumerate() {
        put_u32(image, HASH_OFF + 8 + idx * 4, *value);
    }
    let chain_off = HASH_OFF + 8 + buckets.len() * 4;
    for (idx, value) in chains.iter().enumerate() {
        put_u32(image, chain_off + idx * 4, *value);
    }
}

fn write_symbols_and_strings(image: &mut [u8]) {
    let strings = b"\0phase75_add\0phase75_increment\0";
    image[STRTAB_OFF..STRTAB_OFF + strings.len()].copy_from_slice(strings);

    write_sym(image, 0, 0, 0, 0, 0, 0);
    write_sym(image, 1, 1, 0x12, 1, 0x1000, 17);
    write_sym(image, 2, 13, 0x11, 2, 0x3008, 8);
}

fn write_relocations(image: &mut [u8]) {
    write_rela(image, 0, 0x3000, R_X86_64_RELATIVE, 0, 0x3008);
    write_rela(image, 1, 0x3010, R_X86_64_RELATIVE, 0, 0x1020);
}

fn write_text(image: &mut [u8]) {
    let add_code = [
        0x48, 0x8b, 0x05, 0xf9, 0x1f, 0x00, 0x00, // mov rax, [rip + got]
        0x48, 0x8b, 0x00, // mov rax, [rax]
        0x48, 0x01, 0xf8, // add rax, rdi
        0x48, 0x01, 0xf0, // add rax, rsi
        0xc3, // ret
    ];
    image[TEXT_OFF..TEXT_OFF + add_code.len()].copy_from_slice(&add_code);

    let init_code = [
        0x48, 0xc7, 0x05, 0xdd, 0x1f, 0x00, 0x00, // mov qword [rip + increment], 9
        0x09, 0x00, 0x00, 0x00, 0xc3,
    ];
    let init_off = TEXT_OFF + 0x20;
    image[init_off..init_off + init_code.len()].copy_from_slice(&init_code);
}

fn write_data(image: &mut [u8]) {
    put_u64(image, DATA_OFF, 0);
    put_u64(image, DATA_OFF + 8, 5);
    put_u64(image, DATA_OFF + 16, 0);
}

fn write_phdr(
    image: &mut [u8],
    index: usize,
    kind: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
) {
    let off = PHDR_OFF + index * PHDR_SIZE;
    put_u32(image, off, kind);
    put_u32(image, off + 4, flags);
    put_u64(image, off + 8, offset);
    put_u64(image, off + 16, vaddr);
    put_u64(image, off + 24, vaddr);
    put_u64(image, off + 32, filesz);
    put_u64(image, off + 40, memsz);
    put_u64(image, off + 48, align);
}

fn write_sym(
    image: &mut [u8],
    index: usize,
    name: u32,
    info: u8,
    shndx: u16,
    value: u64,
    size: u64,
) {
    let off = SYMTAB_OFF + index * 24;
    put_u32(image, off, name);
    image[off + 4] = info;
    image[off + 5] = 0;
    put_u16(image, off + 6, shndx);
    put_u64(image, off + 8, value);
    put_u64(image, off + 16, size);
}

fn write_rela(image: &mut [u8], index: usize, offset: u64, kind: u32, sym: u32, addend: i64) {
    let off = RELA_OFF + index * 24;
    put_u64(image, off, offset);
    put_u64(image, off + 8, ((sym as u64) << 32) | kind as u64);
    put_i64(image, off + 16, addend);
}

fn elf_hash(name: &[u8]) -> u32 {
    let mut h = 0u32;
    for &byte in name {
        h = (h << 4).wrapping_add(byte as u32);
        let g = h & 0xf000_0000;
        if g != 0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    h
}

fn put_u16(image: &mut [u8], off: usize, value: u16) {
    image[off..off + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(image: &mut [u8], off: usize, value: u32) {
    image[off..off + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(image: &mut [u8], off: usize, value: u64) {
    image[off..off + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i64(image: &mut [u8], off: usize, value: i64) {
    image[off..off + 8].copy_from_slice(&value.to_le_bytes());
}
