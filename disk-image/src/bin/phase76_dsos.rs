use std::path::Path;

const FILE_LEN: usize = 0x3040;
const EHDR_SIZE: usize = 64;
const PHDR_OFF: usize = 0x40;
const PHDR_SIZE: usize = 56;

const DYN_OFF: usize = 0x200;
const HASH_OFF: usize = 0x320;
const SYMTAB_OFF: usize = 0x360;
const STRTAB_OFF: usize = 0x400;
const RELA_OFF: usize = 0x480;
const TLS_OFF: usize = 0x520;
const TEXT_OFF: usize = 0x1000;
const DATA_OFF: usize = 0x3000;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_TLS: u32 = 7;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_HASH: i64 = 4;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_STRSZ: i64 = 10;
const DT_SYMENT: i64 = 11;
const DT_SONAME: i64 = 14;
const DT_INIT_ARRAY: i64 = 25;
const DT_INIT_ARRAYSZ: i64 = 27;

const R_X86_64_64: u32 = 1;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JUMP_SLOT: u32 = 7;
const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_DTPMOD64: u32 = 16;
const R_X86_64_DTPOFF64: u32 = 17;

fn main() {
    let mut args = std::env::args().skip(1);
    let dep_path = args.next().expect("usage: phase76-dsos <dep.so> <main.so>");
    let main_path = args.next().expect("usage: phase76-dsos <dep.so> <main.so>");
    if args.next().is_some() {
        panic!("usage: phase76-dsos <dep.so> <main.so>");
    }

    write_file(&dep_path, build_dep());
    write_file(&main_path, build_main());
    println!("{} {}", dep_path, main_path);
}

fn build_dep() -> Vec<u8> {
    let mut image = vec![0u8; FILE_LEN];
    let strings = Strings::new(&[
        "phase76_dep_add",
        "phase76_dep_value",
        "phase76_tls_counter",
        "libphase76dep.so",
    ]);

    write_elf_header(&mut image, 5);
    write_common_load_headers(&mut image, 5);
    write_phdr(&mut image, 4, PT_TLS, PF_R, TLS_OFF as u64, 0, 8, 16, 8);
    write_dynamic(
        &mut image,
        &[
            (DT_SONAME, strings.offset("libphase76dep.so") as u64),
            (DT_HASH, HASH_OFF as u64),
            (DT_STRTAB, STRTAB_OFF as u64),
            (DT_SYMTAB, SYMTAB_OFF as u64),
            (DT_STRSZ, strings.bytes.len() as u64),
            (DT_SYMENT, 24),
            (DT_RELA, RELA_OFF as u64),
            (DT_RELASZ, 2 * 24),
            (DT_RELAENT, 24),
            (DT_INIT_ARRAY, 0x3010),
            (DT_INIT_ARRAYSZ, 8),
            (DT_NULL, 0),
        ],
    );
    write_hash(
        &mut image,
        &[
            (strings.name("phase76_dep_add"), 1),
            (strings.name("phase76_dep_value"), 2),
            (strings.name("phase76_tls_counter"), 3),
        ],
        4,
    );
    write_strings(&mut image, &strings);
    write_sym(&mut image, 0, 0, 0, 0, 0, 0);
    write_sym(
        &mut image,
        1,
        strings.offset("phase76_dep_add") as u32,
        0x12,
        1,
        0x1000,
        17,
    );
    write_sym(
        &mut image,
        2,
        strings.offset("phase76_dep_value") as u32,
        0x11,
        2,
        0x3008,
        8,
    );
    write_sym(
        &mut image,
        3,
        strings.offset("phase76_tls_counter") as u32,
        0x16,
        3,
        0,
        8,
    );
    write_rela(&mut image, 0, 0x3000, R_X86_64_RELATIVE, 0, 0x3008);
    write_rela(&mut image, 1, 0x3010, R_X86_64_RELATIVE, 0, 0x1030);
    write_dep_text(&mut image);
    put_u64(&mut image, DATA_OFF, 0);
    put_u64(&mut image, DATA_OFF + 8, 11);
    put_u64(&mut image, DATA_OFF + 16, 0);
    put_u64(&mut image, TLS_OFF, 23);
    image
}

fn build_main() -> Vec<u8> {
    let mut image = vec![0u8; FILE_LEN];
    let strings = Strings::new(&[
        "phase76_run",
        "phase76_dep_add",
        "phase76_dep_value",
        "phase76_tls_counter",
        "libphase76dep.so",
        "libphase76main.so",
    ]);

    write_elf_header(&mut image, 4);
    write_common_load_headers(&mut image, 4);
    write_dynamic(
        &mut image,
        &[
            (DT_NEEDED, strings.offset("libphase76dep.so") as u64),
            (DT_SONAME, strings.offset("libphase76main.so") as u64),
            (DT_HASH, HASH_OFF as u64),
            (DT_STRTAB, STRTAB_OFF as u64),
            (DT_SYMTAB, SYMTAB_OFF as u64),
            (DT_STRSZ, strings.bytes.len() as u64),
            (DT_SYMENT, 24),
            (DT_RELA, RELA_OFF as u64),
            (DT_RELASZ, 6 * 24),
            (DT_RELAENT, 24),
            (DT_INIT_ARRAY, 0x3028),
            (DT_INIT_ARRAYSZ, 8),
            (DT_NULL, 0),
        ],
    );
    write_hash(&mut image, &[(strings.name("phase76_run"), 1)], 5);
    write_strings(&mut image, &strings);
    write_sym(&mut image, 0, 0, 0, 0, 0, 0);
    write_sym(
        &mut image,
        1,
        strings.offset("phase76_run") as u32,
        0x12,
        1,
        0x1000,
        96,
    );
    write_sym(
        &mut image,
        2,
        strings.offset("phase76_dep_add") as u32,
        0x12,
        0,
        0,
        0,
    );
    write_sym(
        &mut image,
        3,
        strings.offset("phase76_dep_value") as u32,
        0x11,
        0,
        0,
        0,
    );
    write_sym(
        &mut image,
        4,
        strings.offset("phase76_tls_counter") as u32,
        0x16,
        0,
        0,
        8,
    );
    write_rela(&mut image, 0, 0x3000, R_X86_64_GLOB_DAT, 3, 0);
    write_rela(&mut image, 1, 0x3008, R_X86_64_64, 4, 0);
    write_rela(&mut image, 2, 0x3010, R_X86_64_DTPMOD64, 4, 0);
    write_rela(&mut image, 3, 0x3018, R_X86_64_DTPOFF64, 4, 0);
    write_rela(&mut image, 4, 0x3020, R_X86_64_JUMP_SLOT, 2, 0);
    write_rela(&mut image, 5, 0x3028, R_X86_64_RELATIVE, 0, 0x1060);
    write_main_text(&mut image);
    put_u64(&mut image, DATA_OFF + 0x30, 0);
    image
}

fn write_file(path: &str, image: Vec<u8>) {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("failed to create {}: {}", parent.display(), e));
    }
    std::fs::write(path, image)
        .unwrap_or_else(|e| panic!("failed to write {}: {}", path.display(), e));
}

fn write_elf_header(image: &mut [u8], phdr_count: u16) {
    image[0..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[7] = 0;
    put_u16(image, 16, 3);
    put_u16(image, 18, 62);
    put_u32(image, 20, 1);
    put_u64(image, 24, 0);
    put_u64(image, 32, PHDR_OFF as u64);
    put_u64(image, 40, 0);
    put_u32(image, 48, 0);
    put_u16(image, 52, EHDR_SIZE as u16);
    put_u16(image, 54, PHDR_SIZE as u16);
    put_u16(image, 56, phdr_count);
    put_u16(image, 58, 0);
    put_u16(image, 60, 0);
    put_u16(image, 62, 0);
}

fn write_common_load_headers(image: &mut [u8], phdr_count: u16) {
    let _ = phdr_count;
    write_phdr(image, 0, PT_LOAD, PF_R, 0, 0, 0x600, 0x1000, 0x1000);
    write_phdr(
        image,
        1,
        PT_LOAD,
        PF_R | PF_X,
        TEXT_OFF as u64,
        0x1000,
        0x80,
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
        0x40,
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
        14 * 16,
        14 * 16,
        8,
    );
}

fn write_dynamic(image: &mut [u8], entries: &[(i64, u64)]) {
    for (idx, (tag, val)) in entries.iter().enumerate() {
        let off = DYN_OFF + idx * 16;
        put_i64(image, off, *tag);
        put_u64(image, off + 8, *val);
    }
}

fn write_hash(image: &mut [u8], symbols: &[(&[u8], u32)], nchain: u32) {
    let nbucket = 4u32;
    let mut buckets = [0u32; 4];
    let mut chains = [0u32; 8];
    for (name, sym_index) in symbols {
        let bucket = (elf_hash(name) % nbucket) as usize;
        chains[*sym_index as usize] = buckets[bucket];
        buckets[bucket] = *sym_index;
    }

    put_u32(image, HASH_OFF, nbucket);
    put_u32(image, HASH_OFF + 4, nchain);
    for (idx, value) in buckets.iter().enumerate() {
        put_u32(image, HASH_OFF + 8 + idx * 4, *value);
    }
    let chain_off = HASH_OFF + 8 + buckets.len() * 4;
    for idx in 0..nchain as usize {
        put_u32(image, chain_off + idx * 4, chains[idx]);
    }
}

fn write_strings(image: &mut [u8], strings: &Strings) {
    image[STRTAB_OFF..STRTAB_OFF + strings.bytes.len()].copy_from_slice(&strings.bytes);
}

fn write_dep_text(image: &mut [u8]) {
    let add_code = [
        0x48, 0x8b, 0x05, 0xf9, 0x1f, 0x00, 0x00, // mov rax, [rip + got]
        0x48, 0x8b, 0x00, // mov rax, [rax]
        0x48, 0x01, 0xf8, // add rax, rdi
        0x48, 0x01, 0xf0, // add rax, rsi
        0xc3, // ret
    ];
    image[TEXT_OFF..TEXT_OFF + add_code.len()].copy_from_slice(&add_code);
    let mut init = Vec::new();
    push_mov_qword_imm32(&mut init, 0x1030, 0x3008, 17);
    init.push(0xc3);
    image[TEXT_OFF + 0x30..TEXT_OFF + 0x30 + init.len()].copy_from_slice(&init);
}

fn write_main_text(image: &mut [u8]) {
    let mut code = Vec::new();
    push_mov_r8_rip(&mut code, 0x1000, 0x3020);
    code.extend_from_slice(&[0xbf, 0x04, 0x00, 0x00, 0x00]);
    code.extend_from_slice(&[0xbe, 0x05, 0x00, 0x00, 0x00]);
    code.extend_from_slice(&[0x41, 0xff, 0xd0]);
    code.extend_from_slice(&[0x49, 0x89, 0xc1]);
    push_mov_rax_rip(&mut code, 0x1000, 0x3000);
    code.extend_from_slice(&[0x48, 0x8b, 0x00]);
    code.extend_from_slice(&[0x49, 0x01, 0xc1]);
    push_mov_rax_rip(&mut code, 0x1000, 0x3008);
    code.extend_from_slice(&[0x48, 0x8b, 0x00]);
    code.extend_from_slice(&[0x49, 0x01, 0xc1]);
    push_mov_rax_rip(&mut code, 0x1000, 0x3018);
    code.extend_from_slice(&[0x49, 0x01, 0xc1]);
    push_mov_rax_rip(&mut code, 0x1000, 0x3010);
    code.extend_from_slice(&[0x49, 0x01, 0xc1]);
    push_mov_rax_rip(&mut code, 0x1000, 0x3030);
    code.extend_from_slice(&[0x49, 0x01, 0xc1]);
    code.extend_from_slice(&[0x4c, 0x89, 0xc8, 0xc3]);
    image[TEXT_OFF..TEXT_OFF + code.len()].copy_from_slice(&code);

    let mut init = Vec::new();
    push_mov_qword_imm32(&mut init, 0x1060, 0x3030, 5);
    init.push(0xc3);
    image[TEXT_OFF + 0x60..TEXT_OFF + 0x60 + init.len()].copy_from_slice(&init);
}

fn push_mov_rax_rip(code: &mut Vec<u8>, base_vaddr: u64, target_vaddr: u64) {
    let instr_vaddr = base_vaddr + code.len() as u64;
    let next = instr_vaddr + 7;
    let disp = (target_vaddr as i64 - next as i64) as i32;
    code.extend_from_slice(&[0x48, 0x8b, 0x05]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn push_mov_r8_rip(code: &mut Vec<u8>, base_vaddr: u64, target_vaddr: u64) {
    let instr_vaddr = base_vaddr + code.len() as u64;
    let next = instr_vaddr + 7;
    let disp = (target_vaddr as i64 - next as i64) as i32;
    code.extend_from_slice(&[0x4c, 0x8b, 0x05]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn push_mov_qword_imm32(code: &mut Vec<u8>, instr_vaddr: u64, target_vaddr: u64, value: i32) {
    let next = instr_vaddr + 11;
    let disp = (target_vaddr as i64 - next as i64) as i32;
    code.extend_from_slice(&[0x48, 0xc7, 0x05]);
    code.extend_from_slice(&disp.to_le_bytes());
    code.extend_from_slice(&value.to_le_bytes());
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

struct Strings {
    bytes: Vec<u8>,
    entries: Vec<(String, usize)>,
}

impl Strings {
    fn new(names: &[&str]) -> Self {
        let mut bytes = vec![0u8];
        let mut entries = Vec::new();
        for name in names {
            let off = bytes.len();
            bytes.extend_from_slice(name.as_bytes());
            bytes.push(0);
            entries.push((name.to_string(), off));
        }
        Self { bytes, entries }
    }

    fn offset(&self, name: &str) -> usize {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, off)| *off)
            .unwrap_or_else(|| panic!("missing string {}", name))
    }

    fn name<'a>(&self, name: &'a str) -> &'a [u8] {
        name.as_bytes()
    }
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
