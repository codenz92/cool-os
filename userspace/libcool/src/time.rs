use super::sys;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

pub fn now() -> Option<DateTime> {
    let packed = unsafe { sys::syscall0(sys::TIME) };
    if packed == 0 {
        return None;
    }
    Some(DateTime {
        year: (packed >> 32) as u16,
        month: (packed >> 24) as u8,
        day: (packed >> 16) as u8,
        hour: (packed >> 8) as u8,
        minute: packed as u8,
    })
}
