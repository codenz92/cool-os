/// Host-side tool: creates a FAT32 disk image and populates it with
/// /bin/hello.txt, userspace binaries, and the CoolFS backing image.
///
/// Usage: fs-image <output-path> [hello-elf] [exec-elf] [pipe-elf] [read-elf] [piperd-elf] [pipewr-elf] [keyecho-elf] [terminal-elf] [netdemo-elf] [wget-elf] [sdkdemo-elf] [guidemo-elf] [notes-elf] [editor-elf] [trash-elf] [screenshot-elf]
/// Output: a 64 MiB raw FAT32 disk image ready to attach as a QEMU IDE drive.
use std::io::Write;

const IMAGE_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB

fn main() {
    let mut args = std::env::args().skip(1);
    let out_path = args.next().expect("Usage: fs-image <output-path>");
    let hello_elf = args.next();
    let exec_elf = args.next();
    let pipe_elf = args.next();
    let read_elf = args.next();
    let piperd_elf = args.next();
    let pipewr_elf = args.next();
    let keyecho_elf = args.next();
    let terminal_elf = args.next();
    let netdemo_elf = args.next();
    let wget_elf = args.next();
    let sdkdemo_elf = args.next();
    let guidemo_elf = args.next();
    let notes_elf = args.next();
    let editor_elf = args.next();
    let trash_elf = args.next();
    let screenshot_elf = args.next();

    // Create or truncate the file, set it to the desired size.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&out_path)
        .unwrap_or_else(|e| panic!("cannot open {}: {}", out_path, e));

    file.set_len(IMAGE_SIZE).expect("failed to set image size");

    // Format as FAT32.
    fatfs::format_volume(
        &file,
        fatfs::FormatVolumeOptions::new()
            .fat_type(fatfs::FatType::Fat32)
            .volume_label(*b"COOLOS     "), // 11 ASCII bytes
    )
    .expect("FAT32 format failed");

    // Populate the filesystem.
    let fs = fatfs::FileSystem::new(&file, fatfs::FsOptions::new())
        .expect("failed to open FAT32 filesystem");

    let root = fs.root_dir();
    for dir in [
        "CONFIG",
        "LOGS",
        "APPS",
        "DEV",
        "TMP",
        "Documents",
        "Pictures",
        "Desktop",
        "Trash",
        "Downloads",
        "Packages",
        "COOL",
    ] {
        root.create_dir(dir)
            .unwrap_or_else(|e| panic!("failed to create /{}: {}", dir, e));
    }

    // /bin/
    root.create_dir("bin").expect("failed to create /bin");
    let bin = root.open_dir("bin").expect("failed to open /bin");

    // /bin/hello.txt
    let mut hello = bin
        .create_file("hello.txt")
        .expect("failed to create hello.txt");
    hello.truncate().unwrap();
    hello
        .write_all(b"Hello from /bin/hello.txt!\n")
        .expect("failed to write hello.txt");

    // /bin/motd.txt — message of the day, for a second file test
    let mut motd = bin
        .create_file("motd.txt")
        .expect("failed to create motd.txt");
    motd.truncate().unwrap();
    motd.write_all(b"coolOS Phase 11 - filesystem alive!\n")
        .expect("failed to write motd.txt");

    let mut coolfs = root
        .create_file("COOLFS.IMG")
        .expect("failed to create COOLFS.IMG");
    coolfs.truncate().unwrap();
    coolfs
        .write_all(&create_coolfs_image())
        .expect("failed to write COOLFS.IMG");

    if let Some(hello_path) = hello_elf {
        let hello_bytes = std::fs::read(&hello_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", hello_path, e));
        let mut hello_bin = bin.create_file("hello").expect("failed to create hello");
        hello_bin.truncate().unwrap();
        hello_bin
            .write_all(&hello_bytes)
            .expect("failed to write hello");
    }

    if let Some(exec_path) = exec_elf {
        let exec_bytes = std::fs::read(&exec_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", exec_path, e));
        let mut exec_bin = bin.create_file("exec").expect("failed to create exec");
        exec_bin.truncate().unwrap();
        exec_bin
            .write_all(&exec_bytes)
            .expect("failed to write exec");
    }

    if let Some(pipe_path) = pipe_elf {
        let pipe_bytes = std::fs::read(&pipe_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", pipe_path, e));
        let mut pipe_bin = bin.create_file("pipe").expect("failed to create pipe");
        pipe_bin.truncate().unwrap();
        pipe_bin
            .write_all(&pipe_bytes)
            .expect("failed to write pipe");
    }

    if let Some(read_path) = read_elf {
        let read_bytes = std::fs::read(&read_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", read_path, e));
        let mut read_bin = bin.create_file("read").expect("failed to create read");
        read_bin.truncate().unwrap();
        read_bin
            .write_all(&read_bytes)
            .expect("failed to write read");
    }

    if let Some(piperd_path) = piperd_elf {
        let piperd_bytes = std::fs::read(&piperd_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", piperd_path, e));
        let mut piperd_bin = bin.create_file("piperd").expect("failed to create piperd");
        piperd_bin.truncate().unwrap();
        piperd_bin
            .write_all(&piperd_bytes)
            .expect("failed to write piperd");
    }

    if let Some(pipewr_path) = pipewr_elf {
        let pipewr_bytes = std::fs::read(&pipewr_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", pipewr_path, e));
        let mut pipewr_bin = bin.create_file("pipewr").expect("failed to create pipewr");
        pipewr_bin.truncate().unwrap();
        pipewr_bin
            .write_all(&pipewr_bytes)
            .expect("failed to write pipewr");
    }

    if let Some(keyecho_path) = keyecho_elf {
        let keyecho_bytes = std::fs::read(&keyecho_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", keyecho_path, e));
        let mut keyecho_bin = bin
            .create_file("keyecho")
            .expect("failed to create keyecho");
        keyecho_bin.truncate().unwrap();
        keyecho_bin
            .write_all(&keyecho_bytes)
            .expect("failed to write keyecho");
    }

    if let Some(terminal_path) = terminal_elf {
        let terminal_bytes = std::fs::read(&terminal_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", terminal_path, e));
        let mut terminal_bin = bin
            .create_file("terminal")
            .expect("failed to create terminal");
        terminal_bin.truncate().unwrap();
        terminal_bin
            .write_all(&terminal_bytes)
            .expect("failed to write terminal");
    }

    if let Some(netdemo_path) = netdemo_elf {
        let netdemo_bytes = std::fs::read(&netdemo_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", netdemo_path, e));
        let mut netdemo_bin = bin
            .create_file("netdemo")
            .expect("failed to create netdemo");
        netdemo_bin.truncate().unwrap();
        netdemo_bin
            .write_all(&netdemo_bytes)
            .expect("failed to write netdemo");
    }

    if let Some(wget_path) = wget_elf {
        let wget_bytes = std::fs::read(&wget_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", wget_path, e));
        let mut wget_bin = bin.create_file("wget").expect("failed to create wget");
        wget_bin.truncate().unwrap();
        wget_bin
            .write_all(&wget_bytes)
            .expect("failed to write wget");
    }

    if let Some(sdkdemo_path) = sdkdemo_elf {
        let sdkdemo_bytes = std::fs::read(&sdkdemo_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", sdkdemo_path, e));
        let mut sdkdemo_bin = bin
            .create_file("sdkdemo")
            .expect("failed to create sdkdemo");
        sdkdemo_bin.truncate().unwrap();
        sdkdemo_bin
            .write_all(&sdkdemo_bytes)
            .expect("failed to write sdkdemo");
    }

    if let Some(guidemo_path) = guidemo_elf {
        let guidemo_bytes = std::fs::read(&guidemo_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", guidemo_path, e));
        let mut guidemo_bin = bin
            .create_file("guidemo")
            .expect("failed to create guidemo");
        guidemo_bin.truncate().unwrap();
        guidemo_bin
            .write_all(&guidemo_bytes)
            .expect("failed to write guidemo");
    }

    if let Some(notes_path) = notes_elf {
        let notes_bytes = std::fs::read(&notes_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", notes_path, e));
        let mut notes_bin = bin.create_file("notes").expect("failed to create notes");
        notes_bin.truncate().unwrap();
        notes_bin
            .write_all(&notes_bytes)
            .expect("failed to write notes");
    }

    if let Some(editor_path) = editor_elf {
        let editor_bytes = std::fs::read(&editor_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", editor_path, e));
        let mut editor_bin = bin.create_file("editor").expect("failed to create editor");
        editor_bin.truncate().unwrap();
        editor_bin
            .write_all(&editor_bytes)
            .expect("failed to write editor");
    }

    if let Some(trash_path) = trash_elf {
        let trash_bytes = std::fs::read(&trash_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", trash_path, e));
        let mut trash_bin = bin.create_file("trash").expect("failed to create trash");
        trash_bin.truncate().unwrap();
        trash_bin
            .write_all(&trash_bytes)
            .expect("failed to write trash");
    }

    if let Some(screenshot_path) = screenshot_elf {
        let screenshot_bytes = std::fs::read(&screenshot_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", screenshot_path, e));
        let mut screenshot_bin = bin
            .create_file("screenshot")
            .expect("failed to create screenshot");
        screenshot_bin.truncate().unwrap();
        screenshot_bin
            .write_all(&screenshot_bytes)
            .expect("failed to write screenshot");
    }

    let packages = root.open_dir("Packages").expect("failed to open /Packages");
    let mut guidemo_pkg = packages
        .create_file("guidemo.pkg")
        .expect("failed to create guidemo.pkg");
    guidemo_pkg.truncate().unwrap();
    guidemo_pkg
        .write_all(PHASE25_GUIDEMO_PACKAGE)
        .expect("failed to write guidemo.pkg");

    let documents = root.open_dir("Documents").expect("failed to open /Documents");
    let mut package_demo = documents
        .create_file("package-demo.p25")
        .expect("failed to create package-demo.p25");
    package_demo.truncate().unwrap();
    package_demo
        .write_all(b"Phase 25 package association sample.\n")
        .expect("failed to write package-demo.p25");

    println!("{}", out_path);
}

const PHASE25_GUIDEMO_PACKAGE: &[u8] = b"id=app.phase25.guidemo\nname=Packaged GUI Demo\ncommand=pkgdemo\nversion=1.0\nicon=P5\ncategory=Development\npermission=desktop\nexec=/bin/guidemo\naliases=package,demo,phase25\nassociations=P25\n";

const CF_MAGIC: [u8; 8] = *b"COOLFS1\0";
const CF_VERSION: u32 = 1;
const CF_BLOCK_SIZE: usize = 512;
const CF_TOTAL_BLOCKS: u32 = 512;
const CF_INODE_COUNT: u32 = 128;
const CF_INODE_SIZE: usize = 256;
const CF_DIRECT_BLOCKS: usize = 48;
const CF_DIR_ENTRY_SIZE: usize = 32;
const CF_INODE_TABLE_START: u32 = 1;
const CF_INODE_TABLE_BLOCKS: u32 =
    ((CF_INODE_COUNT as usize * CF_INODE_SIZE + CF_BLOCK_SIZE - 1) / CF_BLOCK_SIZE) as u32;
const CF_BITMAP_START: u32 = CF_INODE_TABLE_START + CF_INODE_TABLE_BLOCKS;
const CF_BITMAP_BLOCKS: u32 = 1;
const CF_DATA_START: u32 = CF_BITMAP_START + CF_BITMAP_BLOCKS;
const CF_KIND_DIR: u8 = 2;
const CF_KIND_FILE: u8 = 1;

fn create_coolfs_image() -> Vec<u8> {
    let mut image = vec![0u8; CF_TOTAL_BLOCKS as usize * CF_BLOCK_SIZE];
    image[0..8].copy_from_slice(&CF_MAGIC);
    cf_write_u32(&mut image, 8, CF_VERSION);
    cf_write_u32(&mut image, 12, CF_BLOCK_SIZE as u32);
    cf_write_u32(&mut image, 16, CF_TOTAL_BLOCKS);
    cf_write_u32(&mut image, 20, CF_INODE_COUNT);
    cf_write_u32(&mut image, 24, CF_INODE_SIZE as u32);
    cf_write_u32(&mut image, 28, CF_INODE_TABLE_START);
    cf_write_u32(&mut image, 32, CF_INODE_TABLE_BLOCKS);
    cf_write_u32(&mut image, 36, CF_BITMAP_START);
    cf_write_u32(&mut image, 40, CF_BITMAP_BLOCKS);
    cf_write_u32(&mut image, 44, CF_DATA_START);
    cf_write_u32(&mut image, 48, 0);

    for block in 0..CF_DATA_START {
        cf_set_block_used(&mut image, block);
    }

    let root_dir_block = CF_DATA_START;
    let readme_block = CF_DATA_START + 1;
    cf_set_block_used(&mut image, root_dir_block);
    cf_set_block_used(&mut image, readme_block);

    cf_write_inode(
        &mut image,
        0,
        CF_KIND_DIR,
        CF_DIR_ENTRY_SIZE as u32,
        &[root_dir_block],
    );
    cf_write_inode(
        &mut image,
        1,
        CF_KIND_FILE,
        COOLFS_README.len() as u32,
        &[readme_block],
    );
    cf_write_dir_entry(&mut image, root_dir_block, 0, 1, "README.TXT");

    let start = readme_block as usize * CF_BLOCK_SIZE;
    image[start..start + COOLFS_README.len()].copy_from_slice(COOLFS_README);
    image
}

const COOLFS_README: &[u8] = b"Welcome to CoolFS.\nThis file lives inside a native coolOS filesystem image mounted at /COOL.\n";

fn cf_write_inode(image: &mut [u8], inode: u32, kind: u8, size: u32, direct: &[u32]) {
    let off = CF_INODE_TABLE_START as usize * CF_BLOCK_SIZE + inode as usize * CF_INODE_SIZE;
    image[off] = kind;
    cf_write_u32(image, off + 4, size);
    for (idx, block) in direct.iter().take(CF_DIRECT_BLOCKS).enumerate() {
        cf_write_u32(image, off + 8 + idx * 4, *block);
    }
}

fn cf_write_dir_entry(image: &mut [u8], block: u32, slot: usize, inode: u32, name: &str) {
    let off = block as usize * CF_BLOCK_SIZE + slot * CF_DIR_ENTRY_SIZE;
    cf_write_u32(image, off, inode);
    image[off + 4] = name.len() as u8;
    image[off + 5..off + 5 + name.len()].copy_from_slice(name.as_bytes());
}

fn cf_set_block_used(image: &mut [u8], block: u32) {
    let byte = CF_BITMAP_START as usize * CF_BLOCK_SIZE + block as usize / 8;
    image[byte] |= 1u8 << (block % 8);
}

fn cf_write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
