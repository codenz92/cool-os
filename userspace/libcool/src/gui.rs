use font8x8::UnicodeFonts;

use super::{evented, process, sys, Error, Result};

pub const EVENT_PACKET_SIZE: usize = 16;
pub const EVENT_KIND_KEY_CHAR: u8 = 1;
pub const EVENT_KIND_MOUSE_DOWN: u8 = 2;
pub const EVENT_KIND_CLOSE: u8 = 3;
pub const EVENT_KIND_RESIZE: u8 = 4;

include!("../../../src/font/modern8x8.rs");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    KeyChar { bytes: [u8; 4], len: usize },
    MouseDown { button: u8, x: u16, y: u16 },
    Close,
    Resize { width: u16, height: u16 },
}

pub struct Window {
    handle: u64,
    width: usize,
    height: usize,
}

impl Window {
    pub fn open(title: &[u8], width: u16, height: u16) -> Result<Self> {
        let dims = (width as u64) | ((height as u64) << 16);
        let mut ret = u64::MAX;
        for _ in 0..128 {
            ret = unsafe {
                sys::syscall3(
                    sys::GUI_OPEN,
                    title.as_ptr() as u64,
                    title.len() as u64,
                    dims,
                )
            };
            if ret != u64::MAX {
                break;
            }
            process::sleep_ms(1);
        }
        Error::from_ret(ret).map(|handle| Window {
            handle,
            width: width as usize,
            height: height as usize,
        })
    }

    #[inline]
    pub const fn handle(&self) -> u64 {
        self.handle
    }

    #[inline]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub const fn height(&self) -> usize {
        self.height
    }

    pub fn present(&self, pixels: &[u32]) -> Result<()> {
        if pixels.len() != self.width.saturating_mul(self.height) {
            return Err(Error::Invalid);
        }
        let bytes = unsafe {
            core::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)
        };
        let mut ret = u64::MAX;
        for _ in 0..64 {
            ret = unsafe {
                sys::syscall3(
                    sys::GUI_PRESENT,
                    self.handle,
                    bytes.as_ptr() as u64,
                    bytes.len() as u64,
                )
            };
            if ret != u64::MAX {
                break;
            }
            process::sleep_ms(1);
        }
        Error::from_ret(ret).map(|_| ())
    }

    pub fn poll_event(&mut self) -> Result<Option<Event>> {
        let mut packet = [0u8; EVENT_PACKET_SIZE];
        let ret = unsafe {
            sys::syscall3(
                sys::GUI_POLL_EVENT,
                self.handle,
                packet.as_mut_ptr() as u64,
                EVENT_PACKET_SIZE as u64,
            )
        };
        if ret == u64::MAX {
            return Err(Error::Failed);
        }
        if ret == 0 {
            return Ok(None);
        }
        let event = parse_event(&packet)?;
        if let Event::Resize { width, height } = event {
            self.width = width as usize;
            self.height = height as usize;
        }
        Ok(Some(event))
    }

    pub fn wait_event_ready(&self, timeout_ms: u64) -> Result<bool> {
        evented::wait_gui_event(self.handle, timeout_ms)
    }

    pub fn close(self) -> Result<()> {
        let handle = self.handle;
        core::mem::forget(self);
        close_handle(handle)
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let _ = close_handle(self.handle);
    }
}

pub struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: usize,
    height: usize,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u32], width: usize, height: usize) -> Self {
        Canvas {
            pixels,
            width,
            height,
        }
    }

    pub fn clear(&mut self, color: u32) {
        for pixel in self.pixels.iter_mut() {
            *pixel = color & 0x00ff_ffff;
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = (x + w).clamp(0, self.width as i32) as usize;
        let y1 = (y + h).clamp(0, self.height as i32) as usize;
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let color = color & 0x00ff_ffff;
        for row in y0..y1 {
            let start = row * self.width + x0;
            let end = row * self.width + x1;
            for pixel in &mut self.pixels[start..end] {
                *pixel = color;
            }
        }
    }

    pub fn border(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.rect(x, y, w, 1, color);
        self.rect(x, y + h - 1, w, 1, color);
        self.rect(x, y, 1, h, color);
        self.rect(x + w - 1, y, 1, h, color);
    }

    pub fn text(&mut self, x: i32, y: i32, text: &str, color: u32) {
        let mut cursor = x;
        for ch in text.chars() {
            self.char(cursor, y, ch, color);
            cursor += 8;
        }
    }

    pub fn char(&mut self, x: i32, y: i32, ch: char, color: u32) {
        let glyph = cool_modern_glyph_rows(ch);
        let color = color & 0x00ff_ffff;
        for (gy, &byte) in glyph.iter().enumerate() {
            for bit in 0..8usize {
                if byte & (1 << bit) == 0 {
                    continue;
                }
                let px = x + bit as i32;
                let py = y + gy as i32;
                if px >= 0 && py >= 0 {
                    let px = px as usize;
                    let py = py as usize;
                    if px < self.width && py < self.height {
                        self.pixels[py * self.width + px] = color;
                    }
                }
            }
        }
    }
}

fn cool_modern_glyph_rows(ch: char) -> [u8; 8] {
    clean_modern_ascii_glyph(ch).unwrap_or_else(|| {
        font8x8::BASIC_FONTS
            .get(ch)
            .or_else(|| font8x8::BASIC_FONTS.get(' '))
            .unwrap_or([0; 8])
    })
}

fn close_handle(handle: u64) -> Result<()> {
    let mut ret = u64::MAX;
    for _ in 0..64 {
        ret = unsafe { sys::syscall1(sys::GUI_CLOSE, handle) };
        if ret != u64::MAX {
            break;
        }
        process::sleep_ms(1);
    }
    Error::from_ret(ret).map(|_| ())
}

fn parse_event(packet: &[u8; EVENT_PACKET_SIZE]) -> Result<Event> {
    match packet[0] {
        EVENT_KIND_KEY_CHAR => {
            let len = packet[1] as usize;
            if len == 0 || len > 4 {
                return Err(Error::Invalid);
            }
            let mut bytes = [0u8; 4];
            bytes[..len].copy_from_slice(&packet[2..2 + len]);
            Ok(Event::KeyChar { bytes, len })
        }
        EVENT_KIND_MOUSE_DOWN => Ok(Event::MouseDown {
            button: packet[1],
            x: u16::from_le_bytes([packet[2], packet[3]]),
            y: u16::from_le_bytes([packet[4], packet[5]]),
        }),
        EVENT_KIND_CLOSE => Ok(Event::Close),
        EVENT_KIND_RESIZE => Ok(Event::Resize {
            width: u16::from_le_bytes([packet[2], packet[3]]),
            height: u16::from_le_bytes([packet[4], packet[5]]),
        }),
        _ => Err(Error::Invalid),
    }
}
