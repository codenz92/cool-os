use core::{mem, ptr};

use super::{io, memory, thread, Error, Result};

pub const DEFAULT_LOAD_BASE: u64 = 0x0000_7fff_2000_0000;
pub const DEFAULT_OBJECT_STRIDE: u64 = 0x0000_0000_0020_0000;
pub const MAX_IMAGE_BYTES: usize = 16 * 1024;
pub const MAX_OBJECTS: usize = 4;
pub const MAX_OBJECT_NAME: usize = 48;
pub const MAX_PATH_BYTES: usize = 64;
pub const MAX_TLS_BYTES: usize = 256;

const PAGE_SIZE: u64 = 4096;
const MAX_PHDRS: usize = 16;
const MAX_LOAD_SEGMENTS: usize = 8;
const MAX_NEEDED: usize = 4;
const MAX_SYMBOLS: usize = 128;
const MAX_RELOCATIONS: usize = 128;
const MAX_INIT_ARRAY: usize = 16;

const ET_DYN: u16 = 3;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_TLS: u32 = 7;
const PF_X: u32 = 1;
const PF_W: u32 = 2;

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
const DT_FINI_ARRAY: i64 = 26;
const DT_INIT_ARRAYSZ: i64 = 27;
const DT_FINI_ARRAYSZ: i64 = 28;

const R_X86_64_NONE: u32 = 0;
const R_X86_64_64: u32 = 1;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JUMP_SLOT: u32 = 7;
const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_DTPMOD64: u32 = 16;
const R_X86_64_DTPOFF64: u32 = 17;
const R_X86_64_TPOFF64: u32 = 18;

const STT_TLS: u8 = 6;

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Dyn {
    d_tag: i64,
    d_val: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Rela {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Sym {
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
}

#[derive(Clone, Copy)]
struct LoadSegment {
    vaddr_start: u64,
    vaddr_end: u64,
    map_start: u64,
    map_len: u64,
    flags: u32,
}

impl LoadSegment {
    const fn empty() -> Self {
        Self {
            vaddr_start: 0,
            vaddr_end: 0,
            map_start: 0,
            map_len: 0,
            flags: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct DynamicInfo {
    hash: u64,
    strtab: u64,
    strsz: usize,
    symtab: u64,
    syment: usize,
    needed: [u64; MAX_NEEDED],
    needed_count: usize,
    soname: u64,
    rela: u64,
    relasz: usize,
    relaent: usize,
    init_array: u64,
    init_arraysz: usize,
    fini_array: u64,
    fini_arraysz: usize,
}

impl DynamicInfo {
    const fn empty() -> Self {
        Self {
            hash: 0,
            strtab: 0,
            strsz: 0,
            symtab: 0,
            syment: mem::size_of::<Elf64Sym>(),
            needed: [0; MAX_NEEDED],
            needed_count: 0,
            soname: 0,
            rela: 0,
            relasz: 0,
            relaent: mem::size_of::<Elf64Rela>(),
            init_array: 0,
            init_arraysz: 0,
            fini_array: 0,
            fini_arraysz: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct NeededName {
    bytes: [u8; MAX_OBJECT_NAME],
    len: usize,
}

impl NeededName {
    const fn empty() -> Self {
        Self {
            bytes: [0; MAX_OBJECT_NAME],
            len: 0,
        }
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[derive(Clone, Copy)]
struct TlsInfo {
    present: bool,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

impl TlsInfo {
    const fn empty() -> Self {
        Self {
            present: false,
            offset: 0,
            vaddr: 0,
            filesz: 0,
            memsz: 0,
            align: 1,
        }
    }
}

#[derive(Clone, Copy)]
struct ResolvedSymbol {
    value: u64,
    tls_module_id: u64,
    tls_offset: u64,
    is_tls: bool,
}

#[derive(Clone, Copy)]
pub struct LoadedObject {
    base: u64,
    bias: u64,
    path: [u8; MAX_PATH_BYTES],
    path_len: usize,
    soname: [u8; MAX_OBJECT_NAME],
    soname_len: usize,
    strtab: u64,
    strsz: usize,
    symtab: u64,
    syment: usize,
    symbol_count: usize,
    load_count: usize,
    needed_count: usize,
    relocation_count: usize,
    init_array: u64,
    init_count: usize,
    fini_count: usize,
    tls_addr: u64,
    tls_vaddr: u64,
    tls_memsz: usize,
    tls_module_id: u64,
}

impl LoadedObject {
    const fn empty() -> Self {
        Self {
            base: 0,
            bias: 0,
            path: [0; MAX_PATH_BYTES],
            path_len: 0,
            soname: [0; MAX_OBJECT_NAME],
            soname_len: 0,
            strtab: 0,
            strsz: 0,
            symtab: 0,
            syment: mem::size_of::<Elf64Sym>(),
            symbol_count: 0,
            load_count: 0,
            needed_count: 0,
            relocation_count: 0,
            init_array: 0,
            init_count: 0,
            fini_count: 0,
            tls_addr: 0,
            tls_vaddr: 0,
            tls_memsz: 0,
            tls_module_id: 0,
        }
    }

    pub const fn base(self) -> u64 {
        self.base
    }

    pub fn path(&self) -> &[u8] {
        &self.path[..self.path_len]
    }

    pub fn soname(&self) -> &[u8] {
        &self.soname[..self.soname_len]
    }

    pub const fn load_count(self) -> usize {
        self.load_count
    }

    pub const fn needed_count(self) -> usize {
        self.needed_count
    }

    pub const fn relocation_count(self) -> usize {
        self.relocation_count
    }

    pub const fn init_count(self) -> usize {
        self.init_count
    }

    pub const fn fini_count(self) -> usize {
        self.fini_count
    }

    pub const fn tls_module_id(self) -> u64 {
        self.tls_module_id
    }

    pub const fn tls_bytes(self) -> usize {
        self.tls_memsz
    }

    pub fn symbol(self, name: &[u8]) -> Result<u64> {
        if name.is_empty() || self.symbol_count == 0 {
            return Err(Error::Invalid);
        }
        let mut idx = 0usize;
        while idx < self.symbol_count {
            let sym = unsafe {
                read_runtime::<Elf64Sym>(self.symtab + idx as u64 * self.syment as u64)
            };
            if sym.st_name as usize >= self.strsz {
                idx += 1;
                continue;
            }
            let str_addr = self.strtab + sym.st_name as u64;
            if cstr_eq(str_addr, self.strsz - sym.st_name as usize, name) {
                if sym.st_shndx == 0
                    || (sym.st_value == 0 && symbol_type(sym.st_info) != STT_TLS)
                {
                    return Err(Error::Invalid);
                }
                return object_symbol_value(&self, sym).map(|resolved| resolved.value);
            }
            idx += 1;
        }
        Err(Error::Invalid)
    }

    pub unsafe fn call_init_array(self) -> Result<()> {
        let mut idx = 0usize;
        while idx < self.init_count {
            let slot = self.init_array + idx as u64 * 8;
            let func_addr = read_runtime::<u64>(slot);
            if func_addr != 0 {
                let init: extern "C" fn() = mem::transmute(func_addr as usize);
                init();
            }
            idx += 1;
        }
        Ok(())
    }
}

pub struct Workspace {
    images: [[u8; MAX_IMAGE_BYTES]; MAX_OBJECTS],
    tls: [u8; MAX_TLS_BYTES],
}

impl Workspace {
    pub const fn new() -> Self {
        Self {
            images: [[0; MAX_IMAGE_BYTES]; MAX_OBJECTS],
            tls: [0; MAX_TLS_BYTES],
        }
    }
}

pub struct LoadedSet {
    objects: [LoadedObject; MAX_OBJECTS],
    object_count: usize,
    image_count: usize,
    dependency_count: usize,
    relocation_count: usize,
    init_count: usize,
    tls_bytes: usize,
    next_tls_module: u64,
}

impl LoadedSet {
    const fn empty() -> Self {
        Self {
            objects: [LoadedObject::empty(); MAX_OBJECTS],
            object_count: 0,
            image_count: 0,
            dependency_count: 0,
            relocation_count: 0,
            init_count: 0,
            tls_bytes: 0,
            next_tls_module: 1,
        }
    }

    pub const fn object_count(&self) -> usize {
        self.object_count
    }

    pub const fn dependency_count(&self) -> usize {
        self.dependency_count
    }

    pub const fn relocation_count(&self) -> usize {
        self.relocation_count
    }

    pub const fn init_count(&self) -> usize {
        self.init_count
    }

    pub const fn tls_bytes(&self) -> usize {
        self.tls_bytes
    }

    pub fn object(&self, index: usize) -> Option<LoadedObject> {
        if index < self.object_count {
            Some(self.objects[index])
        } else {
            None
        }
    }

    pub fn symbol(&self, name: &[u8]) -> Result<u64> {
        let mut idx = self.object_count;
        while idx > 0 {
            idx -= 1;
            if let Ok(addr) = self.objects[idx].symbol(name) {
                return Ok(addr);
            }
        }
        Err(Error::Invalid)
    }

    fn find_loaded(&self, path: &[u8]) -> Option<usize> {
        let name = basename(path);
        let mut idx = 0usize;
        while idx < self.object_count {
            let object = &self.objects[idx];
            if bytes_eq(object.path(), path)
                || bytes_eq(basename(object.path()), name)
                || (!object.soname().is_empty() && bytes_eq(object.soname(), name))
            {
                return Some(idx);
            }
            idx += 1;
        }
        None
    }
}

pub fn load(path: &[u8], image: &mut [u8], load_base: u64) -> Result<LoadedObject> {
    if image.len() > MAX_IMAGE_BYTES || load_base & (PAGE_SIZE - 1) != 0 {
        return Err(Error::Invalid);
    }
    let file = io::File::open(path)?;
    let len = read_file_contents(&file, image)?;
    let object = load_image_from_file(&image[..len], load_base, Some(file.fd()))?;
    file.close();
    Ok(object)
}

pub fn load_with_deps(
    path: &[u8],
    workspace: &mut Workspace,
    load_base: u64,
) -> Result<LoadedSet> {
    if load_base & (PAGE_SIZE - 1) != 0 || DEFAULT_OBJECT_STRIDE & (PAGE_SIZE - 1) != 0 {
        return Err(Error::Invalid);
    }
    let mut set = LoadedSet::empty();
    load_recursive(path, workspace, &mut set, load_base)?;
    Ok(set)
}

pub fn load_image(image: &[u8], load_base: u64) -> Result<LoadedObject> {
    load_image_from_file(image, load_base, None)
}

fn load_image_from_file(
    image: &[u8],
    load_base: u64,
    file_fd: Option<u64>,
) -> Result<LoadedObject> {
    let header = parse_header(image)?;
    let mut loads = [LoadSegment::empty(); MAX_LOAD_SEGMENTS];
    let load_count = collect_load_segments(image, &header, load_base, &mut loads)?;
    let bias = load_bias(load_base, &loads, load_count)?;
    let mut adjust = 0usize;
    while adjust < load_count {
        loads[adjust].map_start = runtime_addr(bias, loads[adjust].vaddr_start)?;
        adjust += 1;
    }

    let mut idx = 0usize;
    while idx < load_count {
        map_load_segment(image, &loads[idx], bias, &header, file_fd)?;
        idx += 1;
    }

    let dyninfo = parse_dynamic(image, &header)?;
    let symbol_count = symbol_count(&dyninfo, bias, &loads, load_count)?;
    let relocation_count = apply_relocations(&dyninfo, bias, symbol_count, &loads, load_count)?;
    if dyninfo.init_arraysz != 0
        && (dyninfo.init_array == 0
            || !vaddr_range_loaded(
                &loads,
                load_count,
                dyninfo.init_array,
                dyninfo.init_arraysz as u64,
                false,
            ))
    {
        return Err(Error::Invalid);
    }
    protect_load_segments(&loads, load_count)?;

    let init_count = dyninfo.init_arraysz / 8;
    if init_count > MAX_INIT_ARRAY {
        return Err(Error::Invalid);
    }
    let object = LoadedObject {
        base: load_base,
        bias,
        path: [0; MAX_PATH_BYTES],
        path_len: 0,
        soname: [0; MAX_OBJECT_NAME],
        soname_len: 0,
        strtab: runtime_addr(bias, dyninfo.strtab)?,
        strsz: dyninfo.strsz,
        symtab: runtime_addr(bias, dyninfo.symtab)?,
        syment: dyninfo.syment,
        symbol_count,
        load_count,
        needed_count: dyninfo.needed_count,
        relocation_count,
        init_array: if dyninfo.init_array == 0 {
            0
        } else {
            runtime_addr(bias, dyninfo.init_array)?
        },
        init_count,
        fini_count: dyninfo.fini_arraysz / 8,
        tls_addr: 0,
        tls_vaddr: 0,
        tls_memsz: 0,
        tls_module_id: 0,
    };
    unsafe { object.call_init_array()? };
    Ok(object)
}

fn load_recursive(
    path: &[u8],
    workspace: &mut Workspace,
    set: &mut LoadedSet,
    load_base: u64,
) -> Result<usize> {
    if let Some(index) = set.find_loaded(path) {
        return Ok(index);
    }
    if set.image_count >= MAX_OBJECTS {
        return Err(Error::Invalid);
    }
    let image_index = set.image_count;
    set.image_count += 1;
    let file = io::File::open(path)?;
    let len = read_file_contents(&file, &mut workspace.images[image_index])?;

    let mut needed = [NeededName::empty(); MAX_NEEDED];
    let needed_count = {
        let image = &workspace.images[image_index][..len];
        let header = parse_header(image)?;
        let dyninfo = parse_dynamic(image, &header)?;
        collect_needed_names(image, &header, &dyninfo, &mut needed)?
    };

    let mut dep_idx = 0usize;
    while dep_idx < needed_count {
        let mut dep_path = [0u8; MAX_PATH_BYTES];
        let dep_len = build_lib_path(needed[dep_idx].as_slice(), &mut dep_path)?;
        let before = set.object_count;
        load_recursive(&dep_path[..dep_len], workspace, set, load_base)?;
        if set.object_count > before {
            set.dependency_count += 1;
        }
        dep_idx += 1;
    }

    if set.object_count >= MAX_OBJECTS {
        return Err(Error::Invalid);
    }
    let object_index = set.object_count;
    let object_base = load_base
        .checked_add(
            DEFAULT_OBJECT_STRIDE
                .checked_mul(object_index as u64)
                .ok_or(Error::Invalid)?,
        )
        .ok_or(Error::Invalid)?;
    let image = &workspace.images[image_index][..len];
    let object =
        load_image_with_set(image, object_base, path, &mut workspace.tls, set, Some(file.fd()))?;
    file.close();
    set.objects[object_index] = object;
    set.object_count += 1;
    set.relocation_count = set
        .relocation_count
        .checked_add(object.relocation_count)
        .ok_or(Error::Invalid)?;
    set.init_count = set
        .init_count
        .checked_add(object.init_count)
        .ok_or(Error::Invalid)?;
    unsafe { set.objects[object_index].call_init_array()? };
    Ok(object_index)
}

fn load_image_with_set(
    image: &[u8],
    load_base: u64,
    path: &[u8],
    tls_workspace: &mut [u8; MAX_TLS_BYTES],
    set: &mut LoadedSet,
    file_fd: Option<u64>,
) -> Result<LoadedObject> {
    let header = parse_header(image)?;
    let mut loads = [LoadSegment::empty(); MAX_LOAD_SEGMENTS];
    let load_count = collect_load_segments(image, &header, load_base, &mut loads)?;
    let bias = load_bias(load_base, &loads, load_count)?;
    let mut adjust = 0usize;
    while adjust < load_count {
        loads[adjust].map_start = runtime_addr(bias, loads[adjust].vaddr_start)?;
        adjust += 1;
    }

    let mut idx = 0usize;
    while idx < load_count {
        map_load_segment(image, &loads[idx], bias, &header, file_fd)?;
        idx += 1;
    }

    let dyninfo = parse_dynamic(image, &header)?;
    let symbol_count = symbol_count(&dyninfo, bias, &loads, load_count)?;
    let tlsinfo = parse_tls(image, &header)?;
    let (tls_addr, tls_module_id) = allocate_tls(image, &tlsinfo, tls_workspace, set)?;
    let mut path_buf = [0u8; MAX_PATH_BYTES];
    let path_len = copy_bytes(path, &mut path_buf)?;
    let mut soname = [0u8; MAX_OBJECT_NAME];
    let soname_len =
        copy_dynamic_string(image, &header, &dyninfo, dyninfo.soname, &mut soname)?;
    let init_count = dyninfo.init_arraysz / 8;
    let fini_count = dyninfo.fini_arraysz / 8;
    if init_count > MAX_INIT_ARRAY || fini_count > MAX_INIT_ARRAY {
        return Err(Error::Invalid);
    }
    if dyninfo.init_arraysz != 0
        && (dyninfo.init_array == 0
            || !vaddr_range_loaded(
                &loads,
                load_count,
                dyninfo.init_array,
                dyninfo.init_arraysz as u64,
                false,
            ))
    {
        return Err(Error::Invalid);
    }
    if dyninfo.fini_arraysz != 0
        && (dyninfo.fini_array == 0
            || !vaddr_range_loaded(
                &loads,
                load_count,
                dyninfo.fini_array,
                dyninfo.fini_arraysz as u64,
                false,
            ))
    {
        return Err(Error::Invalid);
    }

    let mut object = LoadedObject {
        base: load_base,
        bias,
        path: path_buf,
        path_len,
        soname,
        soname_len,
        strtab: runtime_addr(bias, dyninfo.strtab)?,
        strsz: dyninfo.strsz,
        symtab: runtime_addr(bias, dyninfo.symtab)?,
        syment: dyninfo.syment,
        symbol_count,
        load_count,
        needed_count: dyninfo.needed_count,
        relocation_count: 0,
        init_array: if dyninfo.init_array == 0 {
            0
        } else {
            runtime_addr(bias, dyninfo.init_array)?
        },
        init_count,
        fini_count,
        tls_addr,
        tls_vaddr: tlsinfo.vaddr,
        tls_memsz: tlsinfo.memsz as usize,
        tls_module_id,
    };
    let relocation_count =
        apply_relocations_with_set(&dyninfo, &object, &loads, load_count, set)?;
    object.relocation_count = relocation_count;
    protect_load_segments(&loads, load_count)?;
    Ok(object)
}

#[allow(dead_code)]
fn read_whole_file(path: &[u8], image: &mut [u8]) -> Result<usize> {
    let file = io::File::open(path)?;
    let len = read_file_contents(&file, image)?;
    file.close();
    Ok(len)
}

fn read_file_contents(file: &io::File, image: &mut [u8]) -> Result<usize> {
    let mut total = 0usize;
    loop {
        if total == image.len() {
            return Err(Error::Invalid);
        }
        let n = file.read(&mut image[total..])?;
        if n == 0 {
            return Ok(total);
        }
        total = total.checked_add(n).ok_or(Error::Invalid)?;
    }
}

fn parse_header(image: &[u8]) -> Result<Elf64Header> {
    let header = read_struct::<Elf64Header>(image, 0).ok_or(Error::Invalid)?;
    if &header.e_ident[0..4] != b"\x7fELF"
        || header.e_ident[4] != 2
        || header.e_ident[5] != 1
        || header.e_ident[6] != 1
    {
        return Err(Error::Invalid);
    }
    if header.e_type != ET_DYN || header.e_machine != EM_X86_64 {
        return Err(Error::Invalid);
    }
    if header.e_phentsize as usize != mem::size_of::<Elf64ProgramHeader>()
        || header.e_phnum as usize > MAX_PHDRS
    {
        return Err(Error::Invalid);
    }
    let ph_bytes = (header.e_phnum as usize)
        .checked_mul(header.e_phentsize as usize)
        .ok_or(Error::Invalid)?;
    let ph_end = (header.e_phoff as usize)
        .checked_add(ph_bytes)
        .ok_or(Error::Invalid)?;
    if ph_end > image.len() {
        return Err(Error::Invalid);
    }
    Ok(header)
}

fn collect_load_segments(
    image: &[u8],
    header: &Elf64Header,
    _load_base: u64,
    loads: &mut [LoadSegment; MAX_LOAD_SEGMENTS],
) -> Result<usize> {
    let mut count = 0usize;
    let mut i = 0u16;
    while i < header.e_phnum {
        let ph = program_header(image, header, i)?;
        if ph.p_type == PT_LOAD {
            if count >= MAX_LOAD_SEGMENTS {
                return Err(Error::Invalid);
            }
            validate_load(image, &ph)?;
            let start = align_down(ph.p_vaddr, PAGE_SIZE);
            let end = align_up(
                ph.p_vaddr.checked_add(ph.p_memsz).ok_or(Error::Invalid)?,
                PAGE_SIZE,
            )?;
            let mut existing = 0usize;
            while existing < count {
                if ranges_overlap(
                    start,
                    end,
                    loads[existing].vaddr_start,
                    loads[existing].vaddr_end,
                ) {
                    return Err(Error::Invalid);
                }
                existing += 1;
            }
            loads[count] = LoadSegment {
                vaddr_start: start,
                vaddr_end: end,
                map_start: 0,
                map_len: end - start,
                flags: ph.p_flags,
            };
            count += 1;
        }
        i += 1;
    }
    if count == 0 {
        return Err(Error::Invalid);
    }
    Ok(count)
}

fn load_bias(
    load_base: u64,
    loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
    count: usize,
) -> Result<u64> {
    let mut min = u64::MAX;
    let mut idx = 0usize;
    while idx < count {
        if loads[idx].vaddr_start < min {
            min = loads[idx].vaddr_start;
        }
        idx += 1;
    }
    load_base.checked_sub(min).ok_or(Error::Invalid)
}

fn validate_load(image: &[u8], ph: &Elf64ProgramHeader) -> Result<()> {
    if ph.p_memsz == 0 || ph.p_filesz > ph.p_memsz {
        return Err(Error::Invalid);
    }
    if ph.p_align > 1 && !ph.p_align.is_power_of_two() {
        return Err(Error::Invalid);
    }
    if ph.p_align >= PAGE_SIZE
        && (ph.p_offset & (PAGE_SIZE - 1)) != (ph.p_vaddr & (PAGE_SIZE - 1))
    {
        return Err(Error::Invalid);
    }
    let file_end = ph.p_offset.checked_add(ph.p_filesz).ok_or(Error::Invalid)?;
    if file_end > image.len() as u64 {
        return Err(Error::Invalid);
    }
    ph.p_vaddr.checked_add(ph.p_memsz).ok_or(Error::Invalid)?;
    Ok(())
}

fn map_load_segment(
    image: &[u8],
    load: &LoadSegment,
    bias: u64,
    header: &Elf64Header,
    file_fd: Option<u64>,
) -> Result<()> {
    let mut i = 0u16;
    while i < header.e_phnum {
        let ph = program_header(image, header, i)?;
        let start = align_down(ph.p_vaddr, PAGE_SIZE);
        if ph.p_type == PT_LOAD && start == load.vaddr_start {
            if let Some(fd) = file_fd {
                if can_file_back_segment(image, load, &ph)? {
                    let prot = if ph.p_flags & PF_X != 0 {
                        memory::PROT_EXEC
                    } else {
                        0
                    };
                    memory::mmap_file(
                        fd,
                        load.map_start,
                        load.map_len as usize,
                        align_down(ph.p_offset, PAGE_SIZE),
                        prot,
                    )?;
                    return Ok(());
                }
            }

            memory::mmap(load.map_start, load.map_len as usize, true)?;
            if ph.p_filesz != 0 {
                let dst = runtime_addr(bias, ph.p_vaddr)? as *mut u8;
                let src = image.as_ptr().wrapping_add(ph.p_offset as usize);
                unsafe {
                    ptr::copy_nonoverlapping(src, dst, ph.p_filesz as usize);
                }
            }
            return Ok(());
        }
        i += 1;
    }
    Err(Error::Invalid)
}

fn can_file_back_segment(
    image: &[u8],
    load: &LoadSegment,
    ph: &Elf64ProgramHeader,
) -> Result<bool> {
    if ph.p_flags & PF_W != 0 {
        return Ok(false);
    }
    let file_map_start = align_down(ph.p_offset, PAGE_SIZE);
    let Some(file_map_end) = file_map_start.checked_add(load.map_len) else {
        return Err(Error::Invalid);
    };
    let Some(file_data_end) = ph.p_offset.checked_add(ph.p_filesz) else {
        return Err(Error::Invalid);
    };
    let image_len = image.len() as u64;
    if file_data_end > image_len {
        return Err(Error::Invalid);
    }
    let mut idx = file_data_end as usize;
    let end = file_map_end.min(image_len) as usize;
    while idx < end {
        if image[idx] != 0 {
            return Ok(false);
        }
        idx += 1;
    }
    Ok(true)
}

fn parse_dynamic(image: &[u8], header: &Elf64Header) -> Result<DynamicInfo> {
    let mut info = DynamicInfo::empty();
    let mut found = false;
    let mut i = 0u16;
    while i < header.e_phnum {
        let ph = program_header(image, header, i)?;
        if ph.p_type == PT_DYNAMIC {
            found = true;
            let count = (ph.p_filesz as usize) / mem::size_of::<Elf64Dyn>();
            let mut idx = 0usize;
            while idx < count {
                let dynent = read_struct::<Elf64Dyn>(
                    image,
                    ph.p_offset as usize + idx * mem::size_of::<Elf64Dyn>(),
                )
                .ok_or(Error::Invalid)?;
                match dynent.d_tag {
                    DT_NULL => break,
                    DT_NEEDED => {
                        if info.needed_count >= MAX_NEEDED {
                            return Err(Error::Invalid);
                        }
                        info.needed[info.needed_count] = dynent.d_val;
                        info.needed_count += 1;
                    }
                    DT_HASH => info.hash = dynent.d_val,
                    DT_STRTAB => info.strtab = dynent.d_val,
                    DT_SYMTAB => info.symtab = dynent.d_val,
                    DT_STRSZ => info.strsz = dynent.d_val as usize,
                    DT_SYMENT => info.syment = dynent.d_val as usize,
                    DT_SONAME => info.soname = dynent.d_val,
                    DT_RELA => info.rela = dynent.d_val,
                    DT_RELASZ => info.relasz = dynent.d_val as usize,
                    DT_RELAENT => info.relaent = dynent.d_val as usize,
                    DT_INIT_ARRAY => info.init_array = dynent.d_val,
                    DT_INIT_ARRAYSZ => info.init_arraysz = dynent.d_val as usize,
                    DT_FINI_ARRAY => info.fini_array = dynent.d_val,
                    DT_FINI_ARRAYSZ => info.fini_arraysz = dynent.d_val as usize,
                    _ => {}
                }
                idx += 1;
            }
        }
        i += 1;
    }
    if !found
        || info.hash == 0
        || info.strtab == 0
        || info.symtab == 0
        || info.strsz == 0
        || info.syment != mem::size_of::<Elf64Sym>()
        || info.relaent != mem::size_of::<Elf64Rela>()
        || info.init_arraysz % 8 != 0
        || info.fini_arraysz % 8 != 0
    {
        return Err(Error::Invalid);
    }
    Ok(info)
}

fn collect_needed_names(
    image: &[u8],
    header: &Elf64Header,
    info: &DynamicInfo,
    out: &mut [NeededName; MAX_NEEDED],
) -> Result<usize> {
    let mut idx = 0usize;
    while idx < info.needed_count {
        let len =
            copy_dynamic_string(image, header, info, info.needed[idx], &mut out[idx].bytes)?;
        if len == 0 {
            return Err(Error::Invalid);
        }
        out[idx].len = len;
        idx += 1;
    }
    Ok(info.needed_count)
}

fn copy_dynamic_string(
    image: &[u8],
    header: &Elf64Header,
    info: &DynamicInfo,
    offset: u64,
    out: &mut [u8],
) -> Result<usize> {
    if offset == 0 {
        return Ok(0);
    }
    if offset >= info.strsz as u64 || out.is_empty() {
        return Err(Error::Invalid);
    }
    let start = vaddr_to_file_offset(
        image,
        header,
        info.strtab.checked_add(offset).ok_or(Error::Invalid)?,
        1,
    )?;
    let max = info.strsz - offset as usize;
    let mut idx = 0usize;
    while idx < max {
        let src = start.checked_add(idx).ok_or(Error::Invalid)?;
        if src >= image.len() {
            return Err(Error::Invalid);
        }
        let byte = image[src];
        if byte == 0 {
            if idx > out.len() {
                return Err(Error::Invalid);
            }
            return Ok(idx);
        }
        if idx >= out.len() {
            return Err(Error::Invalid);
        }
        out[idx] = byte;
        idx += 1;
    }
    Err(Error::Invalid)
}

fn parse_tls(image: &[u8], header: &Elf64Header) -> Result<TlsInfo> {
    let mut info = TlsInfo::empty();
    let mut i = 0u16;
    while i < header.e_phnum {
        let ph = program_header(image, header, i)?;
        if ph.p_type == PT_TLS {
            if info.present || ph.p_filesz > ph.p_memsz || ph.p_memsz as usize > MAX_TLS_BYTES {
                return Err(Error::Invalid);
            }
            if ph.p_align > 1 && !ph.p_align.is_power_of_two() {
                return Err(Error::Invalid);
            }
            let file_end = ph.p_offset.checked_add(ph.p_filesz).ok_or(Error::Invalid)?;
            if file_end > image.len() as u64 {
                return Err(Error::Invalid);
            }
            info = TlsInfo {
                present: true,
                offset: ph.p_offset,
                vaddr: ph.p_vaddr,
                filesz: ph.p_filesz,
                memsz: ph.p_memsz,
                align: if ph.p_align == 0 { 1 } else { ph.p_align },
            };
        }
        i += 1;
    }
    Ok(info)
}

fn allocate_tls(
    image: &[u8],
    info: &TlsInfo,
    tls_workspace: &mut [u8; MAX_TLS_BYTES],
    set: &mut LoadedSet,
) -> Result<(u64, u64)> {
    if !info.present || info.memsz == 0 {
        return Ok((0, 0));
    }
    let align = if info.align == 0 {
        1
    } else {
        info.align as usize
    };
    if align == 0 || align > 64 || !align.is_power_of_two() {
        return Err(Error::Invalid);
    }
    let start = align_up_usize(set.tls_bytes, align)?;
    let end = start
        .checked_add(info.memsz as usize)
        .ok_or(Error::Invalid)?;
    if end > MAX_TLS_BYTES {
        return Err(Error::Invalid);
    }
    let src_start = info.offset as usize;
    let src_end = src_start
        .checked_add(info.filesz as usize)
        .ok_or(Error::Invalid)?;
    if src_end > image.len() {
        return Err(Error::Invalid);
    }
    let mut idx = 0usize;
    while idx < info.filesz as usize {
        tls_workspace[start + idx] = image[src_start + idx];
        idx += 1;
    }
    while idx < info.memsz as usize {
        tls_workspace[start + idx] = 0;
        idx += 1;
    }
    let module_id = set.next_tls_module;
    set.next_tls_module = set.next_tls_module.checked_add(1).ok_or(Error::Invalid)?;
    set.tls_bytes = end;
    Ok((tls_workspace.as_ptr() as u64 + start as u64, module_id))
}

fn symbol_count(
    info: &DynamicInfo,
    bias: u64,
    loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
    load_count: usize,
) -> Result<usize> {
    if !vaddr_range_loaded(loads, load_count, info.hash, 8, false)
        || !vaddr_range_loaded(loads, load_count, info.strtab, info.strsz as u64, false)
        || !vaddr_range_loaded(loads, load_count, info.symtab, info.syment as u64, false)
    {
        return Err(Error::Invalid);
    }
    let hash_addr = runtime_addr(bias, info.hash)?;
    let nchain = unsafe { read_runtime::<u32>(hash_addr + 4) as usize };
    if nchain == 0 || nchain > MAX_SYMBOLS {
        return Err(Error::Invalid);
    }
    let sym_bytes = nchain.checked_mul(info.syment).ok_or(Error::Invalid)?;
    if !vaddr_range_loaded(loads, load_count, info.symtab, sym_bytes as u64, false) {
        return Err(Error::Invalid);
    }
    Ok(nchain)
}

fn apply_relocations(
    info: &DynamicInfo,
    bias: u64,
    symbol_count: usize,
    loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
    load_count: usize,
) -> Result<usize> {
    if info.relasz == 0 {
        return Ok(0);
    }
    if info.rela == 0 || info.relasz % info.relaent != 0 {
        return Err(Error::Invalid);
    }
    let count = info.relasz / info.relaent;
    if count > MAX_RELOCATIONS
        || !vaddr_range_loaded(loads, load_count, info.rela, info.relasz as u64, false)
    {
        return Err(Error::Invalid);
    }

    let mut idx = 0usize;
    while idx < count {
        let rela = unsafe {
            read_runtime::<Elf64Rela>(
                runtime_addr(bias, info.rela)? + idx as u64 * info.relaent as u64,
            )
        };
        let r_type = (rela.r_info & 0xffff_ffff) as u32;
        let r_sym = (rela.r_info >> 32) as usize;
        if r_type == R_X86_64_NONE {
            idx += 1;
            continue;
        }
        if !vaddr_range_loaded(loads, load_count, rela.r_offset, 8, true) {
            return Err(Error::Invalid);
        }
        let value = match r_type {
            R_X86_64_RELATIVE => checked_add_i64(bias, rela.r_addend)?,
            R_X86_64_64 | R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                let sym_value = symbol_value(info, bias, symbol_count, r_sym)?;
                checked_add_i64(sym_value, rela.r_addend)?
            }
            _ => return Err(Error::Invalid),
        };
        unsafe {
            ptr::write_unaligned(runtime_addr(bias, rela.r_offset)? as *mut u64, value);
        }
        idx += 1;
    }
    Ok(count)
}

fn apply_relocations_with_set(
    info: &DynamicInfo,
    object: &LoadedObject,
    loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
    load_count: usize,
    set: &LoadedSet,
) -> Result<usize> {
    if info.relasz == 0 {
        return Ok(0);
    }
    if info.rela == 0 || info.relasz % info.relaent != 0 {
        return Err(Error::Invalid);
    }
    let count = info.relasz / info.relaent;
    if count > MAX_RELOCATIONS
        || !vaddr_range_loaded(loads, load_count, info.rela, info.relasz as u64, false)
    {
        return Err(Error::Invalid);
    }

    let mut idx = 0usize;
    while idx < count {
        let rela = unsafe {
            read_runtime::<Elf64Rela>(
                runtime_addr(object.bias, info.rela)? + idx as u64 * info.relaent as u64,
            )
        };
        let r_type = (rela.r_info & 0xffff_ffff) as u32;
        let r_sym = (rela.r_info >> 32) as usize;
        if r_type == R_X86_64_NONE {
            idx += 1;
            continue;
        }
        if !vaddr_range_loaded(loads, load_count, rela.r_offset, 8, true) {
            return Err(Error::Invalid);
        }
        let value = match r_type {
            R_X86_64_RELATIVE => checked_add_i64(object.bias, rela.r_addend)?,
            R_X86_64_64 | R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                let resolved = resolve_symbol_reference(object, set, r_sym)?;
                checked_add_i64(resolved.value, rela.r_addend)?
            }
            R_X86_64_DTPMOD64 => {
                let resolved = resolve_symbol_reference(object, set, r_sym)?;
                if !resolved.is_tls || resolved.tls_module_id == 0 {
                    return Err(Error::Invalid);
                }
                checked_add_i64(resolved.tls_module_id, rela.r_addend)?
            }
            R_X86_64_DTPOFF64 => {
                let resolved = resolve_symbol_reference(object, set, r_sym)?;
                if !resolved.is_tls {
                    return Err(Error::Invalid);
                }
                checked_add_i64(resolved.tls_offset, rela.r_addend)?
            }
            R_X86_64_TPOFF64 => {
                let resolved = resolve_symbol_reference(object, set, r_sym)?;
                if !resolved.is_tls {
                    return Err(Error::Invalid);
                }
                let target = checked_add_i64(resolved.value, rela.r_addend)?;
                let tls_base = thread::tls_base();
                if tls_base == 0 || target < tls_base {
                    return Err(Error::Invalid);
                }
                target.checked_sub(tls_base).ok_or(Error::Invalid)?
            }
            _ => return Err(Error::Invalid),
        };
        unsafe {
            ptr::write_unaligned(runtime_addr(object.bias, rela.r_offset)? as *mut u64, value);
        }
        idx += 1;
    }
    Ok(count)
}

fn symbol_value(
    info: &DynamicInfo,
    bias: u64,
    symbol_count: usize,
    index: usize,
) -> Result<u64> {
    if index == 0 || index >= symbol_count {
        return Err(Error::Invalid);
    }
    let sym = unsafe {
        read_runtime::<Elf64Sym>(
            runtime_addr(bias, info.symtab)? + index as u64 * info.syment as u64,
        )
    };
    if sym.st_shndx == 0 || sym.st_value == 0 {
        return Err(Error::Invalid);
    }
    runtime_addr(bias, sym.st_value)
}

fn resolve_symbol_reference(
    object: &LoadedObject,
    set: &LoadedSet,
    index: usize,
) -> Result<ResolvedSymbol> {
    if index == 0 || index >= object.symbol_count {
        return Err(Error::Invalid);
    }
    let sym = unsafe { read_object_symbol(object, index) };
    if sym.st_name as usize >= object.strsz {
        return Err(Error::Invalid);
    }
    if sym.st_shndx != 0 {
        return object_symbol_value(object, sym);
    }
    let name_addr = object.strtab + sym.st_name as u64;
    let name_max = object.strsz - sym.st_name as usize;
    let mut idx = 0usize;
    while idx < set.object_count {
        if let Ok(resolved) =
            find_export_by_runtime_name(&set.objects[idx], name_addr, name_max)
        {
            return Ok(resolved);
        }
        idx += 1;
    }
    Err(Error::Invalid)
}

fn find_export_by_runtime_name(
    object: &LoadedObject,
    name_addr: u64,
    name_max: usize,
) -> Result<ResolvedSymbol> {
    let mut idx = 1usize;
    while idx < object.symbol_count {
        let sym = unsafe { read_object_symbol(object, idx) };
        if sym.st_name as usize >= object.strsz || sym.st_shndx == 0 {
            idx += 1;
            continue;
        }
        let str_addr = object.strtab + sym.st_name as u64;
        if cstr_runtime_eq(
            str_addr,
            object.strsz - sym.st_name as usize,
            name_addr,
            name_max,
        ) {
            return object_symbol_value(object, sym);
        }
        idx += 1;
    }
    Err(Error::Invalid)
}

fn object_symbol_value(object: &LoadedObject, sym: Elf64Sym) -> Result<ResolvedSymbol> {
    if sym.st_shndx == 0 {
        return Err(Error::Invalid);
    }
    if symbol_type(sym.st_info) == STT_TLS {
        let tls_offset = object_tls_offset(object, sym.st_value, sym.st_size)?;
        return Ok(ResolvedSymbol {
            value: object
                .tls_addr
                .checked_add(tls_offset)
                .ok_or(Error::Invalid)?,
            tls_module_id: object.tls_module_id,
            tls_offset,
            is_tls: true,
        });
    }
    Ok(ResolvedSymbol {
        value: runtime_addr(object.bias, sym.st_value)?,
        tls_module_id: 0,
        tls_offset: 0,
        is_tls: false,
    })
}

fn object_tls_offset(object: &LoadedObject, value: u64, size: u64) -> Result<u64> {
    if object.tls_addr == 0 || object.tls_memsz == 0 {
        return Err(Error::Invalid);
    }
    let offset = if value >= object.tls_vaddr {
        let end = object
            .tls_vaddr
            .checked_add(object.tls_memsz as u64)
            .ok_or(Error::Invalid)?;
        if value < end {
            value - object.tls_vaddr
        } else {
            value
        }
    } else {
        value
    };
    let need = if size == 0 { 1 } else { size };
    let end = offset.checked_add(need).ok_or(Error::Invalid)?;
    if end > object.tls_memsz as u64 {
        return Err(Error::Invalid);
    }
    Ok(offset)
}

unsafe fn read_object_symbol(object: &LoadedObject, index: usize) -> Elf64Sym {
    read_runtime::<Elf64Sym>(object.symtab + index as u64 * object.syment as u64)
}

fn symbol_type(info: u8) -> u8 {
    info & 0x0f
}

fn protect_load_segments(loads: &[LoadSegment; MAX_LOAD_SEGMENTS], count: usize) -> Result<()> {
    let mut idx = 0usize;
    while idx < count {
        let flags = loads[idx].flags;
        if flags & PF_W != 0 && flags & PF_X != 0 {
            return Err(Error::Invalid);
        }
        let prot = if flags & PF_X != 0 {
            memory::PROT_EXEC
        } else if flags & PF_W != 0 {
            memory::PROT_WRITE
        } else {
            0
        };
        memory::mprotect(loads[idx].map_start, loads[idx].map_len as usize, prot)?;
        idx += 1;
    }
    Ok(())
}

fn program_header(
    image: &[u8],
    header: &Elf64Header,
    index: u16,
) -> Result<Elf64ProgramHeader> {
    let off = header.e_phoff as usize + index as usize * header.e_phentsize as usize;
    read_struct::<Elf64ProgramHeader>(image, off).ok_or(Error::Invalid)
}

fn vaddr_range_loaded(
    loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
    count: usize,
    vaddr: u64,
    len: u64,
    writable: bool,
) -> bool {
    if len == 0 {
        return false;
    }
    let Some(end) = vaddr.checked_add(len) else {
        return false;
    };
    let mut idx = 0usize;
    while idx < count {
        let load = loads[idx];
        if vaddr >= load.vaddr_start
            && end <= load.vaddr_end
            && (!writable || load.flags & PF_W != 0)
        {
            return true;
        }
        idx += 1;
    }
    false
}

fn cstr_eq(addr: u64, max_len: usize, name: &[u8]) -> bool {
    let mut idx = 0usize;
    loop {
        if idx >= max_len {
            return false;
        }
        let byte = unsafe { ptr::read((addr + idx as u64) as *const u8) };
        if idx == name.len() {
            return byte == 0;
        }
        if byte == 0 || byte != name[idx] {
            return false;
        }
        idx += 1;
    }
}

fn cstr_runtime_eq(left: u64, left_max: usize, right: u64, right_max: usize) -> bool {
    let mut idx = 0usize;
    loop {
        if idx >= left_max || idx >= right_max {
            return false;
        }
        let left_byte = unsafe { ptr::read((left + idx as u64) as *const u8) };
        let right_byte = unsafe { ptr::read((right + idx as u64) as *const u8) };
        if left_byte != right_byte {
            return false;
        }
        if left_byte == 0 {
            return true;
        }
        idx += 1;
    }
}

fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut idx = 0usize;
    while idx < left.len() {
        if left[idx] != right[idx] {
            return false;
        }
        idx += 1;
    }
    true
}

fn basename(path: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx < path.len() {
        if path[idx] == b'/' {
            start = idx + 1;
        }
        idx += 1;
    }
    &path[start..]
}

fn build_lib_path(name: &[u8], out: &mut [u8; MAX_PATH_BYTES]) -> Result<usize> {
    if name.is_empty() || name.len() + 5 > out.len() {
        return Err(Error::Invalid);
    }
    let mut idx = 0usize;
    while idx < name.len() {
        if name[idx] == b'/' || name[idx] == 0 {
            return Err(Error::Invalid);
        }
        idx += 1;
    }
    out[0] = b'/';
    out[1] = b'l';
    out[2] = b'i';
    out[3] = b'b';
    out[4] = b'/';
    idx = 0;
    while idx < name.len() {
        out[5 + idx] = name[idx];
        idx += 1;
    }
    Ok(5 + name.len())
}

fn copy_bytes(src: &[u8], dst: &mut [u8]) -> Result<usize> {
    if src.len() > dst.len() {
        return Err(Error::Invalid);
    }
    let mut idx = 0usize;
    while idx < src.len() {
        dst[idx] = src[idx];
        idx += 1;
    }
    Ok(src.len())
}

fn vaddr_to_file_offset(
    image: &[u8],
    header: &Elf64Header,
    vaddr: u64,
    len: u64,
) -> Result<usize> {
    let end = vaddr.checked_add(len).ok_or(Error::Invalid)?;
    let mut i = 0u16;
    while i < header.e_phnum {
        let ph = program_header(image, header, i)?;
        if ph.p_type == PT_LOAD {
            let file_end_vaddr = ph.p_vaddr.checked_add(ph.p_filesz).ok_or(Error::Invalid)?;
            if vaddr >= ph.p_vaddr && end <= file_end_vaddr {
                let off = ph
                    .p_offset
                    .checked_add(vaddr - ph.p_vaddr)
                    .ok_or(Error::Invalid)?;
                if off.checked_add(len).ok_or(Error::Invalid)? <= image.len() as u64 {
                    return Ok(off as usize);
                }
            }
        }
        i += 1;
    }
    Err(Error::Invalid)
}

fn read_struct<T: Copy>(bytes: &[u8], offset: usize) -> Option<T> {
    let end = offset.checked_add(mem::size_of::<T>())?;
    if end > bytes.len() {
        return None;
    }
    Some(unsafe { ptr::read_unaligned(bytes.as_ptr().add(offset) as *const T) })
}

unsafe fn read_runtime<T: Copy>(addr: u64) -> T {
    ptr::read_unaligned(addr as *const T)
}

fn runtime_addr(bias: u64, vaddr: u64) -> Result<u64> {
    bias.checked_add(vaddr).ok_or(Error::Invalid)
}

fn checked_add_i64(base: u64, value: i64) -> Result<u64> {
    if value >= 0 {
        base.checked_add(value as u64).ok_or(Error::Invalid)
    } else {
        base.checked_sub(value.wrapping_neg() as u64)
            .ok_or(Error::Invalid)
    }
}

fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}

fn align_up(value: u64, align: u64) -> Result<u64> {
    value
        .checked_add(align - 1)
        .map(|v| v & !(align - 1))
        .ok_or(Error::Invalid)
}

fn align_up_usize(value: usize, align: usize) -> Result<usize> {
    value
        .checked_add(align - 1)
        .map(|v| v & !(align - 1))
        .ok_or(Error::Invalid)
}

fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    a_start < b_end && b_start < a_end
}
