/// Virtual Filesystem layer (Phase 11).
///
/// A minimal fd table backed by the FAT32 parser.  Supports `open`, `read`,
/// and `close`.  Write via the VFS is deferred to a later phase.
///
/// File descriptors 0–2 are reserved (stdin/stdout/stderr) and always return
/// errors; `open` allocates fds starting at 3.

extern crate alloc;
use alloc::vec::Vec;
use spin::Mutex;

// ── Open-file representation ──────────────────────────────────────────────────

struct OpenFile {
    data:   Vec<u8>,
    offset: usize,
}

// ── FD table ──────────────────────────────────────────────────────────────────

const MAX_FDS: usize = 16;

struct FdTable {
    files: [Option<OpenFile>; MAX_FDS],
}

impl FdTable {
    const fn new() -> Self {
        // Can't use array-init with non-Copy types in const context; rely on
        // the None variant being representable as all zeros.
        Self {
            files: [
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
            ],
        }
    }

    fn alloc(&mut self, file: OpenFile) -> Option<usize> {
        // fds 0-2 are reserved; start searching from 3.
        for fd in 3..MAX_FDS {
            if self.files[fd].is_none() {
                self.files[fd] = Some(file);
                return Some(fd);
            }
        }
        None
    }

    fn get_mut(&mut self, fd: usize) -> Option<&mut OpenFile> {
        if fd < MAX_FDS { self.files[fd].as_mut() } else { None }
    }

    fn close(&mut self, fd: usize) {
        if fd < MAX_FDS { self.files[fd] = None; }
    }
}

static FD_TABLE: Mutex<FdTable> = Mutex::new(FdTable::new());

// ── Public VFS API ────────────────────────────────────────────────────────────

/// Open the file at `path` (absolute, e.g. `/bin/hello.txt`).
/// Returns the file descriptor on success, or `usize::MAX` on failure.
pub fn vfs_open(path: &str) -> usize {
    match crate::fat32::read_file(path) {
        Some(data) => {
            let file = OpenFile { data, offset: 0 };
            FD_TABLE.lock().alloc(file).unwrap_or(usize::MAX)
        }
        None => usize::MAX,
    }
}

/// Read up to `len` bytes from `fd` into `buf`.
/// Returns the number of bytes read, or `usize::MAX` on error.
pub fn vfs_read(fd: usize, buf: &mut [u8], len: usize) -> usize {
    let mut table = FD_TABLE.lock();
    let file = match table.get_mut(fd) {
        Some(f) => f,
        None => return usize::MAX,
    };
    let available = file.data.len().saturating_sub(file.offset);
    let to_read = len.min(available).min(buf.len());
    buf[..to_read].copy_from_slice(&file.data[file.offset..file.offset + to_read]);
    file.offset += to_read;
    to_read
}

/// Close the file descriptor `fd`.
pub fn vfs_close(fd: usize) {
    FD_TABLE.lock().close(fd);
}
