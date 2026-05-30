use super::{sys, Error, Result};

pub const MODE_CANONICAL: u64 = 1 << 0;
pub const MODE_ECHO: u64 = 1 << 1;
pub const MODE_SIGNALS: u64 = 1 << 2;
pub const MODE_DEFAULT: u64 = MODE_CANONICAL | MODE_ECHO | MODE_SIGNALS;
pub const MODE_RAW: u64 = 0;

pub const CTL_GET_MODE: u64 = 0;
pub const CTL_SET_MODE: u64 = 1;
pub const CTL_GET_SIZE: u64 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

pub fn control(op: u64, arg1: u64, arg2: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall3(sys::TTY_CONTROL, op, arg1, arg2) };
    Error::from_ret(ret)
}

pub fn mode() -> Result<u64> {
    control(CTL_GET_MODE, 0, 0)
}

pub fn set_mode(mode: u64) -> Result<u64> {
    control(CTL_SET_MODE, mode, 0)
}

pub fn enter_raw_mode() -> Result<u64> {
    set_mode(MODE_RAW)
}

pub fn restore_mode(mode: u64) -> Result<()> {
    set_mode(mode).map(|_| ())
}

pub fn size() -> Result<Size> {
    let packed = control(CTL_GET_SIZE, 0, 0)?;
    Ok(Size {
        cols: (packed & 0xffff) as u16,
        rows: ((packed >> 16) & 0xffff) as u16,
    })
}
