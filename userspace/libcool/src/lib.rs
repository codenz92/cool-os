#![no_std]

pub const SDK_VERSION: u64 = 1;
pub const ABI_VERSION: u64 = 14;
pub const U64_MAX: u64 = u64::MAX;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Failed,
    Invalid,
}

impl Error {
    #[inline]
    pub const fn from_ret(ret: u64) -> Result<u64> {
        if ret == U64_MAX {
            Err(Error::Failed)
        } else {
            Ok(ret)
        }
    }
}

pub mod sys;
pub mod args;
pub use args::Args;
pub mod process;
pub mod thread;
#[allow(non_camel_case_types)]
pub mod posix;
pub mod libc;
pub mod memory;
pub mod io;
pub mod dynlink;
pub mod ipc;
pub mod fs;
pub mod time;
pub mod net;
pub mod event;
pub mod evented;
pub mod tty;
pub mod gui;
pub mod prelude;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::io::_print(core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::io::write_stdout(b"\n");
    }};
    ($fmt:expr) => {{
        $crate::print!(concat!($fmt, "\n"));
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!(concat!($fmt, "\n"), $($arg)*);
    }};
}

#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub extern "C" fn _start() -> ! {
            core::arch::naked_asm!(
                "mov rdi, rsp",
                // __libcool_entry is a normal SysV function. Enter it with the
                // same 16-byte stack alignment it would see after a call.
                "sub rsp, 8",
                "jmp {entry}",
                entry = sym __libcool_entry,
            );
        }

        extern "C" fn __libcool_entry(rsp: u64) -> ! {
            let args = unsafe { $crate::Args::from_stack(rsp) };
            $main(args)
        }

        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo) -> ! {
            $crate::process::abort()
        }
    };
}
