/// Host-side tool: creates a FAT32 disk image and populates it with
/// /bin/hello.txt (and any other files needed by Phase 11+).
///
/// Usage: fs-image <output-path>
/// Output: a 64 MiB raw FAT32 disk image ready to attach as a QEMU IDE drive.

use std::io::Write;

const IMAGE_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB

fn main() {
    let mut args = std::env::args().skip(1);
    let out_path = args.next().expect("Usage: fs-image <output-path>");

    // Create or truncate the file, set it to the desired size.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&out_path)
        .unwrap_or_else(|e| panic!("cannot open {}: {}", out_path, e));

    file.set_len(IMAGE_SIZE)
        .expect("failed to set image size");

    // Format as FAT32.
    fatfs::format_volume(
        &file,
        fatfs::FormatVolumeOptions::new()
            .fat_type(fatfs::FatType::Fat32)
            .volume_label(*b"COOLOS     ")  // 11 ASCII bytes
    )
    .expect("FAT32 format failed");

    // Populate the filesystem.
    let fs = fatfs::FileSystem::new(&file, fatfs::FsOptions::new())
        .expect("failed to open FAT32 filesystem");

    let root = fs.root_dir();

    // /bin/
    root.create_dir("bin").expect("failed to create /bin");
    let bin = root.open_dir("bin").expect("failed to open /bin");

    // /bin/hello.txt
    let mut hello = bin.create_file("hello.txt").expect("failed to create hello.txt");
    hello.truncate().unwrap();
    hello
        .write_all(b"Hello from /bin/hello.txt!\n")
        .expect("failed to write hello.txt");

    // /bin/motd.txt — message of the day, for a second file test
    let mut motd = bin.create_file("motd.txt").expect("failed to create motd.txt");
    motd.truncate().unwrap();
    motd.write_all(b"coolOS Phase 11 - filesystem alive!\n")
        .expect("failed to write motd.txt");

    println!("{}", out_path);
}
