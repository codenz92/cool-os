extern crate alloc;

use alloc::vec::Vec;
use core::mem;

use crate::apps::theme;
use crate::wm::window::{Window, TITLE_H};

const EVENT_PACKET_SIZE: usize = 16;
const EVENT_KIND_KEY_CHAR: u8 = 1;
const EVENT_KIND_MOUSE_DOWN: u8 = 2;
const EVENT_KIND_CLOSE: u8 = 3;
const EVENT_KIND_RESIZE: u8 = 4;
const MAX_EVENTS: usize = 32;

pub struct UserGuiApp {
    pub window: Window,
    owner: usize,
    handle: u64,
    events: Vec<[u8; EVENT_PACKET_SIZE]>,
    event_waiters: Vec<usize>,
    last_content_w: u16,
    last_content_h: u16,
    close_requested_at: Option<u64>,
}

impl UserGuiApp {
    pub fn new(
        owner: usize,
        handle: u64,
        x: i32,
        y: i32,
        width: u16,
        height: u16,
        title: &'static str,
    ) -> Self {
        let width = width.clamp(160, 640);
        let height = height.clamp(96, 420);
        let window = Window::new(x, y, width as i32, height as i32 + TITLE_H, title);
        let mut app = UserGuiApp {
            window,
            owner,
            handle,
            events: Vec::new(),
            event_waiters: Vec::new(),
            last_content_w: width,
            last_content_h: height,
            close_requested_at: None,
        };
        app.clear_placeholder();
        app
    }

    #[inline]
    pub const fn owner(&self) -> usize {
        self.owner
    }

    #[inline]
    pub const fn handle(&self) -> u64 {
        self.handle
    }

    pub fn present(&mut self, pixels: &[u8]) -> bool {
        let width = self.content_width() as usize;
        let height = self.content_height() as usize;
        let Some(expected) = width.checked_mul(height).and_then(|n| n.checked_mul(4)) else {
            return false;
        };
        if pixels.len() != expected || self.window.buf.len() != width * height {
            return false;
        }

        for (dst, src) in self.window.buf.iter_mut().zip(pixels.chunks_exact(4)) {
            *dst = u32::from_le_bytes([src[0], src[1], src[2], src[3]]) & 0x00FF_FFFF;
        }
        self.window.mark_dirty_all();
        true
    }

    pub fn poll_event(&mut self, out: &mut [u8]) -> Option<usize> {
        if out.len() < EVENT_PACKET_SIZE || self.events.is_empty() {
            return None;
        }
        let packet = self.events.remove(0);
        out[..EVENT_PACKET_SIZE].copy_from_slice(&packet);
        Some(EVENT_PACKET_SIZE)
    }

    pub fn has_pending_event(&self) -> bool {
        !self.events.is_empty()
    }

    pub fn register_event_waiter(&mut self, task_id: usize) {
        crate::evented::add_waiter(&mut self.event_waiters, task_id);
    }

    pub fn unregister_event_waiter(&mut self, task_id: usize) {
        crate::evented::remove_waiter(&mut self.event_waiters, task_id);
    }

    pub fn request_close(&mut self) {
        if self.close_requested_at.is_none() {
            self.close_requested_at = Some(crate::interrupts::ticks());
            self.push_event(close_packet());
        }
    }

    pub fn close_timed_out(&self, now: u64, timeout_ticks: u64) -> bool {
        self.close_requested_at
            .map(|requested| now.wrapping_sub(requested) >= timeout_ticks)
            .unwrap_or(false)
    }

    pub fn handle_key(&mut self, c: char) {
        self.push_event(key_packet(c));
    }

    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        self.push_event(mouse_packet(1, lx, ly));
    }

    pub fn handle_scroll(&mut self, delta: i32) {
        let button = if delta > 0 { 4 } else { 5 };
        self.push_event(mouse_packet(button, 0, 0));
    }

    pub fn update(&mut self) {
        let width = self.content_width();
        let height = self.content_height();
        if width != self.last_content_w || height != self.last_content_h {
            self.last_content_w = width;
            self.last_content_h = height;
            self.push_event(resize_packet(width, height));
        }
    }

    fn content_width(&self) -> u16 {
        self.window.width.clamp(1, u16::MAX as i32) as u16
    }

    fn content_height(&self) -> u16 {
        (self.window.height - TITLE_H).clamp(1, u16::MAX as i32) as u16
    }

    fn push_event(&mut self, packet: [u8; EVENT_PACKET_SIZE]) {
        if self.events.len() >= MAX_EVENTS {
            self.events.remove(0);
        }
        self.events.push(packet);
        crate::evented::wake_tasks("gui-event", mem::take(&mut self.event_waiters));
    }

    fn clear_placeholder(&mut self) {
        let stride = self.window.width.max(1) as usize;
        let height = self.window.buf.len() / stride;
        theme::fill_app_background(&mut self.window.buf, stride, height);
        self.window.mark_dirty_all();
    }
}

fn key_packet(c: char) -> [u8; EVENT_PACKET_SIZE] {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    let mut utf8 = [0u8; 4];
    let encoded = c.encode_utf8(&mut utf8);
    packet[0] = EVENT_KIND_KEY_CHAR;
    packet[1] = encoded.len() as u8;
    packet[2..2 + encoded.len()].copy_from_slice(encoded.as_bytes());
    packet
}

fn mouse_packet(button: u8, lx: i32, ly: i32) -> [u8; EVENT_PACKET_SIZE] {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    let x = lx.clamp(0, u16::MAX as i32) as u16;
    let y = ly.clamp(0, u16::MAX as i32) as u16;
    packet[0] = EVENT_KIND_MOUSE_DOWN;
    packet[1] = button;
    packet[2..4].copy_from_slice(&x.to_le_bytes());
    packet[4..6].copy_from_slice(&y.to_le_bytes());
    packet
}

fn close_packet() -> [u8; EVENT_PACKET_SIZE] {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    packet[0] = EVENT_KIND_CLOSE;
    packet
}

fn resize_packet(width: u16, height: u16) -> [u8; EVENT_PACKET_SIZE] {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    packet[0] = EVENT_KIND_RESIZE;
    packet[2..4].copy_from_slice(&width.to_le_bytes());
    packet[4..6].copy_from_slice(&height.to_le_bytes());
    packet
}
