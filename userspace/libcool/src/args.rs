#[derive(Clone, Copy)]
pub struct Args {
    rsp: u64,
    argc: usize,
}

impl Args {
    /// Build argv access from the initial userspace stack.
    ///
    /// The kernel lays the stack out as `argc, argv..., null, envp_null`.
    #[inline]
    pub unsafe fn from_stack(rsp: u64) -> Self {
        let argc = *(rsp as *const u64) as usize;
        Args { rsp, argc }
    }

    #[inline]
    pub const fn len(self) -> usize {
        self.argc
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.argc == 0
    }

    pub fn get(self, index: usize) -> Option<&'static [u8]> {
        if index >= self.argc {
            return None;
        }
        let ptr_slot = (self.rsp + 8 + index as u64 * 8) as *const u64;
        let ptr = unsafe { *ptr_slot } as *const u8;
        if ptr.is_null() {
            return None;
        }
        let len = unsafe { c_strlen(ptr) };
        Some(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    #[inline]
    pub fn program(self) -> Option<&'static [u8]> {
        self.get(0)
    }
}

unsafe fn c_strlen(mut s: *const u8) -> usize {
    let mut n = 0usize;
    while *s != 0 {
        n += 1;
        s = s.add(1);
    }
    n
}
