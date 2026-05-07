/// Host-side tool: creates a native CoolFS OS disk image.
/// CoolFS starts at LBA 0 and is the root filesystem. A FAT32 compatibility
/// region is still formatted at 8 MiB for the optional `/FAT` import mount.
///
/// Usage: fs-image <output-path> [hello-elf] [exec-elf] [pipe-elf] [read-elf] [piperd-elf] [pipewr-elf] [keyecho-elf] [terminal-elf] [ttyread-elf] [netdemo-elf] [wget-elf] [sdkdemo-elf] [guidemo-elf] [notes-elf] [editor-elf] [trash-elf] [screenshot-elf] [procdemo-elf] [procsleep-elf] [sentinel-elf] [badptr-elf] [badwrite-elf] [badmmap-elf] [badexec-elf] [baduserread-elf] [extra-bin-elf...]
/// Output: a 64 MiB raw OS disk image ready to attach as a QEMU IDE drive.
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const IMAGE_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB
const LEGACY_FAT_OFFSET: u64 = 8 * 1024 * 1024; // 8 MiB
const LEGACY_FAT_SIZE: u64 = IMAGE_SIZE - LEGACY_FAT_OFFSET;

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
    let ttyread_elf = args.next();
    let netdemo_elf = args.next();
    let wget_elf = args.next();
    let sdkdemo_elf = args.next();
    let guidemo_elf = args.next();
    let notes_elf = args.next();
    let editor_elf = args.next();
    let trash_elf = args.next();
    let screenshot_elf = args.next();
    let procdemo_elf = args.next();
    let procsleep_elf = args.next();
    let sentinel_elf = args.next();
    let badptr_elf = args.next();
    let badwrite_elf = args.next();
    let badmmap_elf = args.next();
    let badexec_elf = args.next();
    let baduserread_elf = args.next();
    let extra_bin_elfs: Vec<String> = args.collect();

    // Create or truncate the file, set it to the desired size.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&out_path)
        .unwrap_or_else(|e| panic!("cannot open {}: {}", out_path, e));

    file.set_len(IMAGE_SIZE).expect("failed to set image size");

    let mut legacy_fat = RegionFile::new(
        file.try_clone().expect("failed to clone output file"),
        LEGACY_FAT_OFFSET,
        LEGACY_FAT_SIZE,
    );

    // Format the optional FAT32 import area.
    fatfs::format_volume(
        &mut legacy_fat,
        fatfs::FormatVolumeOptions::new()
            .fat_type(fatfs::FatType::Fat32)
            .volume_label(*b"COOLOS     "), // 11 ASCII bytes
    )
    .expect("FAT32 format failed");
    legacy_fat
        .seek(SeekFrom::Start(0))
        .expect("failed to rewind legacy FAT region");

    // Populate the compatibility filesystem and the native CoolFS root in parallel.
    let fs = fatfs::FileSystem::new(&mut legacy_fat, fatfs::FsOptions::new())
        .expect("failed to open FAT32 filesystem");

    let root = fs.root_dir();
    let mut coolfs = CoolFsBuilder::new();
    for dir in [
        "CONFIG",
        "LOGS",
        "APPS",
        "DEV",
        "TMP",
        "Users",
        "Documents",
        "Pictures",
        "Desktop",
        "Trash",
        "Downloads",
        "Packages",
        "FONTS",
        "RECOVERY",
        "SDK",
    ] {
        root.create_dir(dir)
            .unwrap_or_else(|e| panic!("failed to create /{}: {}", dir, e));
        coolfs.create_dir(&format!("/{}", dir));
    }
    coolfs.create_dir("/Users/root");
    coolfs.create_dir("/Users/guest");

    // /bin/
    root.create_dir("bin").expect("failed to create /bin");
    coolfs.create_dir("/bin");
    let bin = root.open_dir("bin").expect("failed to open /bin");
    populate_builtin_app_manifests(&mut coolfs);

    // /bin/hello.txt
    let mut hello = bin
        .create_file("hello.txt")
        .expect("failed to create hello.txt");
    hello.truncate().unwrap();
    let hello_txt = b"Hello from /bin/hello.txt!\n";
    hello
        .write_all(hello_txt)
        .expect("failed to write hello.txt");
    coolfs.create_file("/bin/hello.txt", hello_txt);

    // /bin/motd.txt — message of the day, for a second file test
    let mut motd = bin
        .create_file("motd.txt")
        .expect("failed to create motd.txt");
    motd.truncate().unwrap();
    let motd_txt = b"coolOS Phase 29 - sessions and services online!\n";
    motd.write_all(motd_txt).expect("failed to write motd.txt");
    coolfs.create_file("/bin/motd.txt", motd_txt);

    let users_db = default_users_db();
    coolfs.create_file("/CONFIG/USERS.DB", users_db.as_bytes());
    let recovery_readme = b"coolOS recovery\n\nBoot target: BIOS VBE framebuffer, IDE disk index 1, CoolFS root at /.\nRun `recovery` for status and `recovery repair` to recreate standard system directories and write /RECOVERY/LAST-REPAIR.TXT.\n";
    coolfs.create_file("/RECOVERY/README.TXT", recovery_readme);
    let recovery_boot_cfg = b"boot=normal\nroot=/\nrootfs=coolfs\nvideo=bios-vbe\nstorage=ide1\n";
    coolfs.create_file("/RECOVERY/BOOT.CFG", recovery_boot_cfg);

    let sdk = root.open_dir("SDK").expect("failed to open /SDK");
    {
        let mut write_sdk_file = |name: &str, coolfs_path: &str, bytes: &[u8]| {
            let mut sdk_file = sdk
                .create_file(name)
                .unwrap_or_else(|e| panic!("failed to create {}: {}", coolfs_path, e));
            sdk_file.truncate().unwrap();
            sdk_file
                .write_all(bytes)
                .unwrap_or_else(|e| panic!("failed to write {}: {}", coolfs_path, e));
            coolfs.create_file(coolfs_path, bytes);
        };
        write_sdk_file("README.TXT", "/SDK/README.TXT", SDK_README);
        write_sdk_file("APP_TEMPLATE.RS", "/SDK/APP_TEMPLATE.RS", SDK_APP_TEMPLATE);
        write_sdk_file(
            "PACKAGE_TEMPLATE.PKG",
            "/SDK/PACKAGE_TEMPLATE.PKG",
            SDK_PACKAGE_TEMPLATE,
        );
    }

    if let Some(hello_path) = hello_elf {
        let hello_bytes = std::fs::read(&hello_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", hello_path, e));
        let mut hello_bin = bin.create_file("hello").expect("failed to create hello");
        hello_bin.truncate().unwrap();
        hello_bin
            .write_all(&hello_bytes)
            .expect("failed to write hello");
        coolfs.create_file("/bin/hello", &hello_bytes);
    }

    if let Some(exec_path) = exec_elf {
        let exec_bytes = std::fs::read(&exec_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", exec_path, e));
        let mut exec_bin = bin.create_file("exec").expect("failed to create exec");
        exec_bin.truncate().unwrap();
        exec_bin
            .write_all(&exec_bytes)
            .expect("failed to write exec");
        coolfs.create_file("/bin/exec", &exec_bytes);
    }

    if let Some(pipe_path) = pipe_elf {
        let pipe_bytes = std::fs::read(&pipe_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", pipe_path, e));
        let mut pipe_bin = bin.create_file("pipe").expect("failed to create pipe");
        pipe_bin.truncate().unwrap();
        pipe_bin
            .write_all(&pipe_bytes)
            .expect("failed to write pipe");
        coolfs.create_file("/bin/pipe", &pipe_bytes);
    }

    if let Some(read_path) = read_elf {
        let read_bytes = std::fs::read(&read_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", read_path, e));
        let mut read_bin = bin.create_file("read").expect("failed to create read");
        read_bin.truncate().unwrap();
        read_bin
            .write_all(&read_bytes)
            .expect("failed to write read");
        coolfs.create_file("/bin/read", &read_bytes);
    }

    if let Some(piperd_path) = piperd_elf {
        let piperd_bytes = std::fs::read(&piperd_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", piperd_path, e));
        let mut piperd_bin = bin.create_file("piperd").expect("failed to create piperd");
        piperd_bin.truncate().unwrap();
        piperd_bin
            .write_all(&piperd_bytes)
            .expect("failed to write piperd");
        coolfs.create_file("/bin/piperd", &piperd_bytes);
    }

    if let Some(pipewr_path) = pipewr_elf {
        let pipewr_bytes = std::fs::read(&pipewr_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", pipewr_path, e));
        let mut pipewr_bin = bin.create_file("pipewr").expect("failed to create pipewr");
        pipewr_bin.truncate().unwrap();
        pipewr_bin
            .write_all(&pipewr_bytes)
            .expect("failed to write pipewr");
        coolfs.create_file("/bin/pipewr", &pipewr_bytes);
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
        coolfs.create_file("/bin/keyecho", &keyecho_bytes);
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
        coolfs.create_file("/bin/terminal", &terminal_bytes);
    }

    if let Some(ttyread_path) = ttyread_elf {
        let ttyread_bytes = std::fs::read(&ttyread_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", ttyread_path, e));
        let mut ttyread_bin = bin
            .create_file("ttyread")
            .expect("failed to create ttyread");
        ttyread_bin.truncate().unwrap();
        ttyread_bin
            .write_all(&ttyread_bytes)
            .expect("failed to write ttyread");
        coolfs.create_file("/bin/ttyread", &ttyread_bytes);
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
        coolfs.create_file("/bin/netdemo", &netdemo_bytes);
    }

    if let Some(wget_path) = wget_elf {
        let wget_bytes = std::fs::read(&wget_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", wget_path, e));
        let mut wget_bin = bin.create_file("wget").expect("failed to create wget");
        wget_bin.truncate().unwrap();
        wget_bin
            .write_all(&wget_bytes)
            .expect("failed to write wget");
        coolfs.create_file("/bin/wget", &wget_bytes);
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
        coolfs.create_file("/bin/sdkdemo", &sdkdemo_bytes);
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
        coolfs.create_file("/bin/guidemo", &guidemo_bytes);
    }

    if let Some(notes_path) = notes_elf {
        let notes_bytes = std::fs::read(&notes_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", notes_path, e));
        let mut notes_bin = bin.create_file("notes").expect("failed to create notes");
        notes_bin.truncate().unwrap();
        notes_bin
            .write_all(&notes_bytes)
            .expect("failed to write notes");
        coolfs.create_file("/bin/notes", &notes_bytes);
    }

    if let Some(editor_path) = editor_elf {
        let editor_bytes = std::fs::read(&editor_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", editor_path, e));
        let mut editor_bin = bin.create_file("editor").expect("failed to create editor");
        editor_bin.truncate().unwrap();
        editor_bin
            .write_all(&editor_bytes)
            .expect("failed to write editor");
        coolfs.create_file("/bin/editor", &editor_bytes);
    }

    if let Some(trash_path) = trash_elf {
        let trash_bytes = std::fs::read(&trash_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", trash_path, e));
        let mut trash_bin = bin.create_file("trash").expect("failed to create trash");
        trash_bin.truncate().unwrap();
        trash_bin
            .write_all(&trash_bytes)
            .expect("failed to write trash");
        coolfs.create_file("/bin/trash", &trash_bytes);
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
        coolfs.create_file("/bin/screenshot", &screenshot_bytes);
    }

    if let Some(procdemo_path) = procdemo_elf {
        let procdemo_bytes = std::fs::read(&procdemo_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", procdemo_path, e));
        let mut procdemo_bin = bin
            .create_file("procdemo")
            .expect("failed to create procdemo");
        procdemo_bin.truncate().unwrap();
        procdemo_bin
            .write_all(&procdemo_bytes)
            .expect("failed to write procdemo");
        coolfs.create_file("/bin/procdemo", &procdemo_bytes);
    }

    if let Some(procsleep_path) = procsleep_elf {
        let procsleep_bytes = std::fs::read(&procsleep_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", procsleep_path, e));
        let mut procsleep_bin = bin
            .create_file("procsleep")
            .expect("failed to create procsleep");
        procsleep_bin.truncate().unwrap();
        procsleep_bin
            .write_all(&procsleep_bytes)
            .expect("failed to write procsleep");
        coolfs.create_file("/bin/procsleep", &procsleep_bytes);
    }

    if let Some(sentinel_path) = sentinel_elf {
        let sentinel_bytes = std::fs::read(&sentinel_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", sentinel_path, e));
        let mut sentinel_bin = bin
            .create_file("sentinel")
            .expect("failed to create sentinel");
        sentinel_bin.truncate().unwrap();
        sentinel_bin
            .write_all(&sentinel_bytes)
            .expect("failed to write sentinel");
        coolfs.create_file("/bin/sentinel", &sentinel_bytes);
    }

    if let Some(badptr_path) = badptr_elf {
        let badptr_bytes = std::fs::read(&badptr_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", badptr_path, e));
        let mut badptr_bin = bin.create_file("badptr").expect("failed to create badptr");
        badptr_bin.truncate().unwrap();
        badptr_bin
            .write_all(&badptr_bytes)
            .expect("failed to write badptr");
        coolfs.create_file("/bin/badptr", &badptr_bytes);
    }

    if let Some(badwrite_path) = badwrite_elf {
        let badwrite_bytes = std::fs::read(&badwrite_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", badwrite_path, e));
        let mut badwrite_bin = bin
            .create_file("badwrite")
            .expect("failed to create badwrite");
        badwrite_bin.truncate().unwrap();
        badwrite_bin
            .write_all(&badwrite_bytes)
            .expect("failed to write badwrite");
        coolfs.create_file("/bin/badwrite", &badwrite_bytes);
    }

    if let Some(badmmap_path) = badmmap_elf {
        let badmmap_bytes = std::fs::read(&badmmap_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", badmmap_path, e));
        let mut badmmap_bin = bin
            .create_file("badmmap")
            .expect("failed to create badmmap");
        badmmap_bin.truncate().unwrap();
        badmmap_bin
            .write_all(&badmmap_bytes)
            .expect("failed to write badmmap");
        coolfs.create_file("/bin/badmmap", &badmmap_bytes);
    }

    if let Some(badexec_path) = badexec_elf {
        let badexec_bytes = std::fs::read(&badexec_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", badexec_path, e));
        let mut badexec_bin = bin
            .create_file("badexec")
            .expect("failed to create badexec");
        badexec_bin.truncate().unwrap();
        badexec_bin
            .write_all(&badexec_bytes)
            .expect("failed to write badexec");
        coolfs.create_file("/bin/badexec", &badexec_bytes);
    }

    if let Some(baduserread_path) = baduserread_elf {
        let baduserread_bytes = std::fs::read(&baduserread_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", baduserread_path, e));
        let mut baduserread_bin = bin
            .create_file("baduserread")
            .expect("failed to create baduserread");
        baduserread_bin.truncate().unwrap();
        baduserread_bin
            .write_all(&baduserread_bytes)
            .expect("failed to write baduserread");
        coolfs.create_file("/bin/baduserread", &baduserread_bytes);
    }

    for extra_path in extra_bin_elfs {
        let name = Path::new(&extra_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("invalid extra binary path {}", extra_path));
        let bytes = std::fs::read(&extra_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", extra_path, e));
        let mut fat_bin = bin
            .create_file(name)
            .unwrap_or_else(|e| panic!("failed to create {}: {}", name, e));
        fat_bin.truncate().unwrap();
        fat_bin
            .write_all(&bytes)
            .unwrap_or_else(|e| panic!("failed to write {}: {}", name, e));
        coolfs.create_file(&format!("/bin/{}", name), &bytes);
    }

    let packages = root.open_dir("Packages").expect("failed to open /Packages");
    let mut guidemo_pkg = packages
        .create_file("guidemo.pkg")
        .expect("failed to create guidemo.pkg");
    guidemo_pkg.truncate().unwrap();
    guidemo_pkg
        .write_all(PHASE25_GUIDEMO_PACKAGE)
        .expect("failed to write guidemo.pkg");
    coolfs.create_file("/Packages/guidemo.pkg", PHASE25_GUIDEMO_PACKAGE);

    let documents = root
        .open_dir("Documents")
        .expect("failed to open /Documents");
    let package_demo_bytes = b"Phase 25 package association sample.\n";
    let mut package_demo = documents
        .create_file("package-demo.p25")
        .expect("failed to create package-demo.p25");
    package_demo.truncate().unwrap();
    package_demo
        .write_all(package_demo_bytes)
        .expect("failed to write package-demo.p25");
    coolfs.create_file("/Documents/package-demo.p25", package_demo_bytes);

    let coolfs_image = coolfs.into_image();
    file.seek(SeekFrom::Start(0))
        .expect("failed to seek to CoolFS start");
    file.write_all(&coolfs_image)
        .expect("failed to write native CoolFS image");
    file.sync_all().expect("failed to sync disk image");

    println!("{}", out_path);
}

struct RegionFile {
    file: std::fs::File,
    base: u64,
    len: u64,
    pos: u64,
}

impl RegionFile {
    fn new(file: std::fs::File, base: u64, len: u64) -> Self {
        Self {
            file,
            base,
            len,
            pos: 0,
        }
    }
}

impl Read for RegionFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let max_len = (self.len - self.pos).min(buf.len() as u64) as usize;
        self.file.seek(SeekFrom::Start(self.base + self.pos))?;
        let read = self.file.read(&mut buf[..max_len])?;
        self.pos += read as u64;
        Ok(read)
    }
}

impl Write for RegionFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.pos >= self.len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "region write past end",
            ));
        }
        let max_len = (self.len - self.pos).min(buf.len() as u64) as usize;
        self.file.seek(SeekFrom::Start(self.base + self.pos))?;
        let written = self.file.write(&buf[..max_len])?;
        self.pos += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Seek for RegionFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::End(offset) => self.len as i128 + offset as i128,
            SeekFrom::Current(offset) => self.pos as i128 + offset as i128,
        };
        if next < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "negative region seek",
            ));
        }
        self.pos = next as u64;
        Ok(self.pos)
    }
}

const PHASE25_GUIDEMO_PACKAGE: &[u8] = b"id=app.phase25.guidemo\nname=Packaged GUI Demo\ncommand=pkgdemo\nversion=1.0\nicon=P5\ncategory=Development\npermission=desktop\nexec=/bin/guidemo\naliases=package,demo,phase25\nassociations=P25\ndepends=\nmin_os_version=1\n";

const SDK_README: &[u8] = b"coolOS SDK\n\nABI version: 10\nUserspace apps are no_std Rust ELF64 binaries linked with userspace/libcool.\nUseful APIs: process::spawn_args, process::spawn_fds_args, evented::poll, tty::{size,set_mode,enter_raw_mode}, fs::{stat,rename,chdir,getcwd,sync}, io::{open,create,pipe}, gui::Window.\nPackage manifests live under /Packages, must be signed with pkg sign or pkg sign-as, and install into /APPS/<command>/APP.CFG with an OWNER.TXT trust record.\n";

const SDK_APP_TEMPLATE: &[u8] = b"#![no_std]\n#![no_main]\n\nuse libcool::{io, prelude::*};\n\nlibcool::entry!(main);\n\nfn main(args: Args) -> ! {\n    io::write_stdout(b\"hello from a coolOS app\\n\");\n    if let Some(name) = args.get(1) {\n        io::write_stdout(b\"arg: \");\n        io::write_stdout(name);\n        io::write_stdout(b\"\\n\");\n    }\n    exit(0);\n}\n";

const SDK_PACKAGE_TEMPLATE: &[u8] = b"id=app.example\nname=Example App\ncommand=example\nversion=1.0\nicon=EX\ncategory=Development\npermission=filesystem\nexec=/bin/example\naliases=example,demo\nassociations=TXT\ndepends=\nmin_os_version=1\n";

struct BuiltinAppManifest {
    id: &'static str,
    name: &'static str,
    glyph: &'static str,
    command: &'static str,
    category: &'static str,
    permission: &'static str,
    aliases: &'static [&'static str],
    associations: &'static [&'static str],
}

const BUILTIN_APP_MANIFESTS: &[BuiltinAppManifest] = &[
    BuiltinAppManifest {
        id: "app.terminal",
        name: "Terminal",
        glyph: "T>",
        command: "terminal",
        category: "System",
        permission: "shell",
        aliases: &["shell", "console", "cmd", "command"],
        associations: &["CMD"],
    },
    BuiltinAppManifest {
        id: "app.sysmon",
        name: "System Monitor",
        glyph: "M#",
        command: "sysmon",
        category: "System",
        permission: "diagnostics",
        aliases: &["monitor", "tasks", "processes", "performance"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.diagnostics",
        name: "Diagnostics",
        glyph: "D!",
        command: "diagnostics",
        category: "System",
        permission: "diagnostics",
        aliases: &["diag", "health", "logs", "profiler", "debug"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.files",
        name: "File Manager",
        glyph: "FM",
        command: "files",
        category: "Files",
        permission: "filesystem",
        aliases: &["files", "folders", "explorer", "documents"],
        associations: &["DIR"],
    },
    BuiltinAppManifest {
        id: "app.viewer",
        name: "Text Viewer",
        glyph: "Tx",
        command: "viewer",
        category: "Files",
        permission: "read-files",
        aliases: &["text", "notes", "readme", "viewer"],
        associations: &["TXT", "MD", "LOG", "CFG", "RS"],
    },
    BuiltinAppManifest {
        id: "app.editor",
        name: "Text Editor",
        glyph: "ED",
        command: "editor",
        category: "Files",
        permission: "filesystem",
        aliases: &["edit", "write", "notepad", "document"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.notes",
        name: "Notes",
        glyph: "NT",
        command: "notes",
        category: "Tools",
        permission: "filesystem",
        aliases: &["note", "memo", "scratchpad", "journal"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.trash",
        name: "Trash Bin",
        glyph: "TR",
        command: "trash",
        category: "System",
        permission: "filesystem",
        aliases: &["trash", "bin", "deleted", "recycle"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.screenshot",
        name: "Screenshot",
        glyph: "SS",
        command: "screenshot",
        category: "Tools",
        permission: "desktop",
        aliases: &["screen", "capture", "snapshot", "shot"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.browser",
        name: "Web Browser",
        glyph: "WB",
        command: "browser",
        category: "Network",
        permission: "network",
        aliases: &["browser", "web", "www", "internet", "http"],
        associations: &["HTML", "HTM", "URL"],
    },
    BuiltinAppManifest {
        id: "app.colors",
        name: "Color Picker",
        glyph: "CP",
        command: "colors",
        category: "Tools",
        permission: "desktop",
        aliases: &["colors", "palette", "theme"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.display",
        name: "Display Settings",
        glyph: "DS",
        command: "display",
        category: "Settings",
        permission: "settings",
        aliases: &[
            "settings",
            "display",
            "accessibility",
            "network",
            "storage",
            "power",
        ],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.personalize",
        name: "Personalize",
        glyph: "P*",
        command: "personalize",
        category: "Settings",
        permission: "settings",
        aliases: &["wallpaper", "theme", "desktop"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.accounts",
        name: "Accounts",
        glyph: "U+",
        command: "accounts",
        category: "Settings",
        permission: "settings",
        aliases: &["account", "accounts", "users", "password", "login"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.crash",
        name: "Crash Viewer",
        glyph: "CV",
        command: "crash",
        category: "System",
        permission: "diagnostics",
        aliases: &["crash", "dump", "fault", "panic"],
        associations: &["DMP"],
    },
    BuiltinAppManifest {
        id: "app.logs",
        name: "Log Viewer",
        glyph: "LV",
        command: "logs",
        category: "System",
        permission: "diagnostics",
        aliases: &["logs", "kernel", "services", "events"],
        associations: &["LOG"],
    },
    BuiltinAppManifest {
        id: "app.profiler",
        name: "Boot Profiler",
        glyph: "BP",
        command: "profiler",
        category: "System",
        permission: "diagnostics",
        aliases: &["boot", "profiler", "startup", "timing"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.welcome",
        name: "Welcome",
        glyph: "W?",
        command: "welcome",
        category: "System",
        permission: "desktop",
        aliases: &["help", "cheatsheet", "shortcuts"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.guidemo",
        name: "GUI Demo",
        glyph: "UG",
        command: "guidemo",
        category: "Development",
        permission: "desktop",
        aliases: &["gui", "userspace", "sdk", "window"],
        associations: &[],
    },
    BuiltinAppManifest {
        id: "app.procdemo",
        name: "Process Demo",
        glyph: "P3",
        command: "procdemo",
        category: "Development",
        permission: "diagnostics",
        aliases: &["process", "signals", "jobs", "phase33"],
        associations: &[],
    },
];

fn populate_builtin_app_manifests(coolfs: &mut CoolFsBuilder) {
    for app in BUILTIN_APP_MANIFESTS {
        let dir = format!("/APPS/{}", app.command);
        coolfs.create_dir(&dir);
        let path = format!("{}/APP.CFG", dir);
        let manifest = builtin_manifest_text(app);
        coolfs.create_file(&path, manifest.as_bytes());
    }
}

fn builtin_manifest_text(app: &BuiltinAppManifest) -> String {
    format!(
        "id={}\nname={}\ncommand={}\nversion=builtin\nicon={}\ncategory={}\npermission={}\nexec={}\naliases={}\nassociations={}\n",
        app.id,
        app.name,
        app.command,
        app.glyph,
        app.category,
        app.permission,
        builtin_exec_path(app.command),
        app.aliases.join(","),
        app.associations.join(",")
    )
}

fn builtin_exec_path(command: &str) -> String {
    match command {
        "editor" | "notes" | "trash" | "screenshot" | "guidemo" | "procdemo" => {
            format!("/bin/{}", command)
        }
        _ => format!("internal:{}", command),
    }
}

const CF_MAGIC: [u8; 8] = *b"COOLFS1\0";
const CF_VERSION: u32 = 1;
const CF_BLOCK_SIZE: usize = 4096;
const CF_TOTAL_BLOCKS: u32 = 1024;
const CF_INODE_COUNT: u32 = 512;
const CF_INODE_SIZE: usize = 256;
const CF_DIRECT_BLOCKS: usize = 48;
const CF_INDIRECT_ENTRIES: usize = CF_BLOCK_SIZE / 4;
const CF_DIR_ENTRY_SIZE: usize = 32;
const CF_MAX_NAME_LEN: usize = 27;
const CF_INODE_TABLE_START: u32 = 1;
const CF_INODE_TABLE_BLOCKS: u32 =
    ((CF_INODE_COUNT as usize * CF_INODE_SIZE + CF_BLOCK_SIZE - 1) / CF_BLOCK_SIZE) as u32;
const CF_BITMAP_START: u32 = CF_INODE_TABLE_START + CF_INODE_TABLE_BLOCKS;
const CF_BITMAP_BLOCKS: u32 = 1;
const CF_DATA_START: u32 = CF_BITMAP_START + CF_BITMAP_BLOCKS;
const CF_KIND_DIR: u8 = 2;
const CF_KIND_FILE: u8 = 1;
const CF_KIND_FREE: u8 = 0;
const CF_ROOT_INODE: u32 = 0;
const CF_ROOT_UID: u32 = 0;
const CF_ROOT_GID: u32 = 0;
const CF_USER_UID: u32 = 1000;
const CF_USER_GID: u32 = 1000;
const CF_GUEST_UID: u32 = 1001;
const CF_DEFAULT_DIR_MODE: u16 = 0o755;
const CF_DEFAULT_FILE_MODE: u16 = 0o644;
const CF_DEFAULT_EXEC_MODE: u16 = 0o755;
const CF_INODE_UID_OFFSET: usize = 12 + CF_DIRECT_BLOCKS * 4;
const CF_INODE_GID_OFFSET: usize = CF_INODE_UID_OFFSET + 4;
const CF_INODE_MODE_OFFSET: usize = CF_INODE_GID_OFFSET + 4;
const CF_INODE_USED_BYTES: usize = CF_INODE_MODE_OFFSET + 4;

#[derive(Clone)]
struct CfInode {
    kind: u8,
    size: u32,
    direct: [u32; CF_DIRECT_BLOCKS],
    indirect: u32,
    uid: u32,
    gid: u32,
    mode: u16,
}

impl CfInode {
    fn new(kind: u8) -> Self {
        Self::with_metadata(
            kind,
            CF_ROOT_UID,
            CF_ROOT_GID,
            match kind {
                CF_KIND_DIR => CF_DEFAULT_DIR_MODE,
                CF_KIND_FILE => CF_DEFAULT_FILE_MODE,
                _ => 0,
            },
        )
    }

    fn with_metadata(kind: u8, uid: u32, gid: u32, mode: u16) -> Self {
        Self {
            kind,
            size: 0,
            direct: [0; CF_DIRECT_BLOCKS],
            indirect: 0,
            uid,
            gid,
            mode: mode & 0o777,
        }
    }
}

struct CfDirEntry {
    inode: u32,
    name: String,
}

struct CoolFsBuilder {
    image: Vec<u8>,
}

impl CoolFsBuilder {
    fn new() -> Self {
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
        cf_write_u32(&mut image, 48, CF_ROOT_INODE);

        let mut builder = Self { image };
        for block in 0..CF_DATA_START {
            builder.set_block_used(block, true);
        }
        builder.write_inode(CF_ROOT_INODE, &CfInode::new(CF_KIND_DIR));
        builder
    }

    fn create_dir(&mut self, path: &str) {
        let (uid, gid, mode) = default_metadata_for_path(path, true);
        self.create_dir_with_metadata(path, uid, gid, mode);
    }

    fn create_dir_with_metadata(&mut self, path: &str, uid: u32, gid: u32, mode: u16) {
        if self.resolve_path(path).is_some() {
            return;
        }
        let (parent, name) = split_parent_and_name(path);
        let parent_inode = self
            .resolve_path(&parent)
            .unwrap_or_else(|| panic!("missing parent directory {}", parent));
        let inode = self.alloc_inode_with_metadata(CF_KIND_DIR, uid, gid, mode);
        self.append_dir_entry(parent_inode, inode, &name);
    }

    fn create_file(&mut self, path: &str, data: &[u8]) {
        let (uid, gid, mode) = default_metadata_for_path(path, false);
        self.create_file_with_metadata(path, data, uid, gid, mode);
    }

    fn create_file_with_metadata(
        &mut self,
        path: &str,
        data: &[u8],
        uid: u32,
        gid: u32,
        mode: u16,
    ) {
        if let Some(inode) = self.resolve_path(path) {
            self.write_inode_bytes(inode, data);
            return;
        }
        let (parent, name) = split_parent_and_name(path);
        let parent_inode = self
            .resolve_path(&parent)
            .unwrap_or_else(|| panic!("missing parent directory {}", parent));
        let inode = self.alloc_inode_with_metadata(CF_KIND_FILE, uid, gid, mode);
        self.write_inode_bytes(inode, data);
        self.append_dir_entry(parent_inode, inode, &name);
    }

    fn into_image(self) -> Vec<u8> {
        let len = self.persist_len();
        self.image[..len].to_vec()
    }

    fn append_dir_entry(&mut self, parent_inode: u32, inode: u32, name: &str) {
        validate_name(name);
        let mut entries = self.read_dir_entries(parent_inode);
        if entries
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(name))
        {
            panic!("duplicate CoolFS entry {}", name);
        }
        entries.push(CfDirEntry {
            inode,
            name: name.to_string(),
        });
        self.write_dir_entries(parent_inode, &entries);
    }

    fn resolve_path(&self, path: &str) -> Option<u32> {
        let path = normalize_abs_path(path);
        if path == "/" {
            return Some(CF_ROOT_INODE);
        }
        let mut inode_idx = CF_ROOT_INODE;
        for component in path.split('/').filter(|component| !component.is_empty()) {
            let inode = self.read_inode(inode_idx)?;
            if inode.kind != CF_KIND_DIR {
                return None;
            }
            let entries = self.read_dir_entries(inode_idx);
            let entry = entries
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(component))?;
            inode_idx = entry.inode;
        }
        Some(inode_idx)
    }

    fn read_inode(&self, inode_idx: u32) -> Option<CfInode> {
        if inode_idx >= CF_INODE_COUNT {
            return None;
        }
        let off = self.inode_offset(inode_idx);
        let mut inode = CfInode::new(self.image[off]);
        inode.size = cf_read_u32(&self.image, off + 4);
        for idx in 0..CF_DIRECT_BLOCKS {
            inode.direct[idx] = cf_read_u32(&self.image, off + 8 + idx * 4);
        }
        inode.indirect = cf_read_u32(&self.image, off + 8 + CF_DIRECT_BLOCKS * 4);
        inode.uid = cf_read_u32(&self.image, off + CF_INODE_UID_OFFSET);
        inode.gid = cf_read_u32(&self.image, off + CF_INODE_GID_OFFSET);
        inode.mode = (cf_read_u32(&self.image, off + CF_INODE_MODE_OFFSET) as u16) & 0o777;
        Some(inode)
    }

    fn write_inode(&mut self, inode_idx: u32, inode: &CfInode) {
        let off = self.inode_offset(inode_idx);
        self.image[off] = inode.kind;
        self.image[off + 1..off + 4].fill(0);
        cf_write_u32(&mut self.image, off + 4, inode.size);
        for idx in 0..CF_DIRECT_BLOCKS {
            cf_write_u32(&mut self.image, off + 8 + idx * 4, inode.direct[idx]);
        }
        cf_write_u32(
            &mut self.image,
            off + 8 + CF_DIRECT_BLOCKS * 4,
            inode.indirect,
        );
        cf_write_u32(&mut self.image, off + CF_INODE_UID_OFFSET, inode.uid);
        cf_write_u32(&mut self.image, off + CF_INODE_GID_OFFSET, inode.gid);
        cf_write_u32(
            &mut self.image,
            off + CF_INODE_MODE_OFFSET,
            (inode.mode & 0o777) as u32,
        );
        self.image[off + CF_INODE_USED_BYTES..off + CF_INODE_SIZE].fill(0);
    }

    fn read_dir_entries(&self, inode_idx: u32) -> Vec<CfDirEntry> {
        let inode = self
            .read_inode(inode_idx)
            .unwrap_or_else(|| panic!("missing inode {}", inode_idx));
        assert_eq!(inode.kind, CF_KIND_DIR, "inode {} is not a dir", inode_idx);
        let bytes = self.read_inode_bytes(&inode);
        let mut entries = Vec::new();
        for chunk in bytes.chunks(CF_DIR_ENTRY_SIZE) {
            if chunk.len() < CF_DIR_ENTRY_SIZE {
                break;
            }
            let inode = cf_read_u32(chunk, 0);
            let name_len = chunk[4] as usize;
            if inode == 0 || name_len == 0 {
                continue;
            }
            let name = std::str::from_utf8(&chunk[5..5 + name_len])
                .expect("CoolFS dir entry name is not UTF-8");
            entries.push(CfDirEntry {
                inode,
                name: name.to_string(),
            });
        }
        entries
    }

    fn write_dir_entries(&mut self, inode_idx: u32, entries: &[CfDirEntry]) {
        let mut bytes = Vec::with_capacity(entries.len() * CF_DIR_ENTRY_SIZE);
        for entry in entries {
            validate_name(&entry.name);
            let mut raw = [0u8; CF_DIR_ENTRY_SIZE];
            cf_write_u32(&mut raw, 0, entry.inode);
            raw[4] = entry.name.len() as u8;
            raw[5..5 + entry.name.len()].copy_from_slice(entry.name.as_bytes());
            bytes.extend_from_slice(&raw);
        }
        self.write_inode_bytes(inode_idx, &bytes);
    }

    fn read_inode_bytes(&self, inode: &CfInode) -> Vec<u8> {
        let mut out = Vec::with_capacity(inode.size as usize);
        let mut remaining = inode.size as usize;
        for block in self.inode_data_blocks(inode) {
            if remaining == 0 {
                break;
            }
            let start = block as usize * CF_BLOCK_SIZE;
            let take = remaining.min(CF_BLOCK_SIZE);
            out.extend_from_slice(&self.image[start..start + take]);
            remaining -= take;
        }
        out
    }

    fn write_inode_bytes(&mut self, inode_idx: u32, bytes: &[u8]) {
        let max_blocks = CF_DIRECT_BLOCKS + CF_INDIRECT_ENTRIES;
        assert!(
            bytes.len() <= max_blocks * CF_BLOCK_SIZE,
            "CoolFS file too large: {} bytes",
            bytes.len()
        );
        let needed = (bytes.len() + CF_BLOCK_SIZE - 1) / CF_BLOCK_SIZE;
        let mut blocks = Vec::new();
        for _ in 0..needed {
            blocks.push(self.alloc_block());
        }
        let indirect = if needed > CF_DIRECT_BLOCKS {
            self.alloc_block()
        } else {
            0
        };

        for (idx, &block) in blocks.iter().enumerate() {
            let start = block as usize * CF_BLOCK_SIZE;
            self.image[start..start + CF_BLOCK_SIZE].fill(0);
            let byte_start = idx * CF_BLOCK_SIZE;
            let byte_end = (byte_start + CF_BLOCK_SIZE).min(bytes.len());
            self.image[start..start + byte_end - byte_start]
                .copy_from_slice(&bytes[byte_start..byte_end]);
        }
        if indirect != 0 {
            let start = indirect as usize * CF_BLOCK_SIZE;
            self.image[start..start + CF_BLOCK_SIZE].fill(0);
            for (idx, &block) in blocks[CF_DIRECT_BLOCKS..].iter().enumerate() {
                cf_write_u32(&mut self.image, start + idx * 4, block);
            }
        }

        let mut inode = self
            .read_inode(inode_idx)
            .unwrap_or_else(|| panic!("missing inode {}", inode_idx));
        let old_blocks = self.inode_data_blocks(&inode);
        let old_indirect = inode.indirect;
        inode.size = bytes.len() as u32;
        inode.direct = [0; CF_DIRECT_BLOCKS];
        inode.indirect = indirect;
        for (idx, &block) in blocks.iter().enumerate().take(CF_DIRECT_BLOCKS) {
            inode.direct[idx] = block;
        }
        self.write_inode(inode_idx, &inode);
        for block in old_blocks {
            self.set_block_used(block, false);
        }
        if old_indirect != 0 {
            self.set_block_used(old_indirect, false);
        }
    }

    fn inode_data_blocks(&self, inode: &CfInode) -> Vec<u32> {
        let needed = (inode.size as usize + CF_BLOCK_SIZE - 1) / CF_BLOCK_SIZE;
        let mut blocks = Vec::with_capacity(needed);
        for &block in inode.direct.iter().take(needed.min(CF_DIRECT_BLOCKS)) {
            if block != 0 {
                blocks.push(block);
            }
        }
        if needed > CF_DIRECT_BLOCKS && inode.indirect != 0 {
            let start = inode.indirect as usize * CF_BLOCK_SIZE;
            for idx in 0..needed - CF_DIRECT_BLOCKS {
                let block = cf_read_u32(&self.image, start + idx * 4);
                if block != 0 {
                    blocks.push(block);
                }
            }
        }
        blocks
    }

    fn alloc_inode_with_metadata(&mut self, kind: u8, uid: u32, gid: u32, mode: u16) -> u32 {
        for inode in 1..CF_INODE_COUNT {
            if self.read_inode(inode).map(|item| item.kind) == Some(CF_KIND_FREE) {
                self.write_inode(inode, &CfInode::with_metadata(kind, uid, gid, mode));
                return inode;
            }
        }
        panic!("CoolFS inode table full");
    }

    fn alloc_block(&mut self) -> u32 {
        for block in CF_DATA_START..CF_TOTAL_BLOCKS {
            if !self.block_used(block) {
                self.set_block_used(block, true);
                let start = block as usize * CF_BLOCK_SIZE;
                self.image[start..start + CF_BLOCK_SIZE].fill(0);
                return block;
            }
        }
        panic!("CoolFS image full");
    }

    fn block_used(&self, block: u32) -> bool {
        let byte = CF_BITMAP_START as usize * CF_BLOCK_SIZE + block as usize / 8;
        self.image[byte] & (1u8 << (block % 8)) != 0
    }

    fn set_block_used(&mut self, block: u32, used: bool) {
        let byte = CF_BITMAP_START as usize * CF_BLOCK_SIZE + block as usize / 8;
        let bit = 1u8 << (block % 8);
        if used {
            self.image[byte] |= bit;
        } else {
            self.image[byte] &= !bit;
        }
    }

    fn inode_offset(&self, inode_idx: u32) -> usize {
        CF_INODE_TABLE_START as usize * CF_BLOCK_SIZE + inode_idx as usize * CF_INODE_SIZE
    }

    fn persist_len(&self) -> usize {
        for block in (0..CF_TOTAL_BLOCKS).rev() {
            if self.block_used(block) {
                return (block as usize + 1) * CF_BLOCK_SIZE;
            }
        }
        CF_DATA_START as usize * CF_BLOCK_SIZE
    }
}

fn split_parent_and_name(path: &str) -> (String, String) {
    let path = normalize_abs_path(path);
    assert_ne!(path, "/", "cannot split root path");
    let slash = path.rfind('/').expect("absolute path without slash");
    let parent = if slash == 0 {
        String::from("/")
    } else {
        path[..slash].to_string()
    };
    let name = path[slash + 1..].to_string();
    validate_name(&name);
    (parent, name)
}

fn default_metadata_for_path(path: &str, is_dir: bool) -> (u32, u32, u16) {
    let normalized = normalize_abs_path(path);
    if normalized == "/Users/root" || normalized.starts_with("/Users/root/") {
        return (
            CF_USER_UID,
            CF_USER_GID,
            if is_dir { 0o700 } else { CF_DEFAULT_FILE_MODE },
        );
    }
    if normalized == "/Users/guest" || normalized.starts_with("/Users/guest/") {
        return (
            CF_GUEST_UID,
            CF_USER_GID,
            if is_dir { 0o700 } else { CF_DEFAULT_FILE_MODE },
        );
    }
    let user_owned = normalized == "/TMP"
        || normalized.starts_with("/TMP/")
        || normalized == "/Documents"
        || normalized.starts_with("/Documents/")
        || normalized == "/Pictures"
        || normalized.starts_with("/Pictures/")
        || normalized == "/Desktop"
        || normalized.starts_with("/Desktop/")
        || normalized == "/Trash"
        || normalized.starts_with("/Trash/")
        || normalized == "/Downloads"
        || normalized.starts_with("/Downloads/")
        || normalized == "/Packages"
        || normalized.starts_with("/Packages/");
    let executable = !is_dir && normalized.starts_with("/bin/") && !normalized.ends_with(".txt");
    let mode = if is_dir && normalized == "/TMP" {
        0o777
    } else if is_dir {
        CF_DEFAULT_DIR_MODE
    } else if executable {
        CF_DEFAULT_EXEC_MODE
    } else {
        CF_DEFAULT_FILE_MODE
    };
    if user_owned {
        (CF_USER_UID, CF_USER_GID, mode)
    } else {
        (CF_ROOT_UID, CF_ROOT_GID, mode)
    }
}

fn default_users_db() -> String {
    format!(
        "# coolOS users v1: name:uid:gid:role:home:passhash:login\nroot:1000:1000:admin:/Users/root:{}:enabled\nguest:1001:1000:user:/Users/guest:{}:enabled\n",
        password_hash("root", "cool"),
        password_hash("guest", "guest"),
    )
}

fn password_hash(name: &str, password: &str) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in name
        .bytes()
        .chain(std::iter::once(b':'))
        .chain(password.bytes())
    {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn normalize_abs_path(path: &str) -> String {
    assert!(
        path.starts_with('/'),
        "CoolFS path must be absolute: {}",
        path
    );
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => panic!("CoolFS image path may not contain ..: {}", path),
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return String::from("/");
    }
    let mut out = String::new();
    for part in parts {
        out.push('/');
        out.push_str(part);
    }
    out
}

fn validate_name(name: &str) {
    assert!(
        !name.is_empty() && name != "." && name != ".." && name.len() <= CF_MAX_NAME_LEN,
        "unsupported CoolFS name {}",
        name
    );
    assert!(
        name.bytes()
            .all(|byte| (0x20..=0x7e).contains(&byte) && byte != b'/'),
        "non-printable CoolFS name {}",
        name
    );
}

fn cf_read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn cf_write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
