#![no_std]
#![no_main]

use libcool::{fs, gui, prelude::*};

const WIDTH: usize = 480;
const HEIGHT: usize = 260;
const BG_TOP: u32 = 0x0006_1020;
const BG_BOTTOM: u32 = 0x0002_0711;
const PANEL: u32 = 0x000b_1c2d;
const PANEL_ALT: u32 = 0x0010_2a3d;
const BORDER: u32 = 0x0030_6178;
const ACCENT: u32 = 0x007d_f7ff;
const ACCENT_ALT: u32 = 0x006c_a8ff;
const TEXT: u32 = 0x00e8_faff;
const MUTED: u32 = 0x0085_a8ba;
const SUCCESS: u32 = 0x0068_f0a0;
const WARNING: u32 = 0x00ff_d166;

const SYSTEM_DIR: &[u8] = b"/SYSTEM";
const ENGINE_DIR: &[u8] = b"/SYSTEM/BROWSER-ENGINE";
const HOST_READY: &[u8] = b"/SYSTEM/BROWSER-ENGINE/HOST.READY";
const HOST_REQUEST: &[u8] = b"/SYSTEM/BROWSER-ENGINE/HOST.REQUEST";
const HOST_LOG: &[u8] = b"/LOGS/BROWSER-ENGINE-HOST.TXT";

static mut PIXELS: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

libcool::entry!(main);

fn main(_args: Args) -> ! {
    let _ = fs::create_dir(SYSTEM_DIR);
    let _ = fs::create_dir(ENGINE_DIR);

    let mut request = [0u8; 512];
    let request_len = fs::read_file(HOST_REQUEST, &mut request).unwrap_or(0);
    let request_text = core::str::from_utf8(&request[..request_len]).unwrap_or("request=missing\n");

    let _ = fs::write_file(
        HOST_READY,
        b"engine_host ready=yes backend=test process=browserhost surface=rgba-gui input=gui-events title=\"coolOS Engine Host Test\"\n",
    );
    let _ = fs::write_file(HOST_LOG, host_log(request_text).as_bytes());
    println!("browserhost: ready /SYSTEM/BROWSER-ENGINE/HOST.READY");

    let mut window = match gui::Window::open(b"Browser Engine Host", WIDTH as u16, HEIGHT as u16) {
        Ok(window) => window,
        Err(_) => {
            println!("browserhost: open failed");
            exit(1);
        }
    };
    println!("browserhost: window opened");

    let pixels = unsafe { &mut *core::ptr::addr_of_mut!(PIXELS) };
    draw(pixels, request_text, 0, false, false);
    if window.present(pixels).is_err() {
        println!("browserhost: present failed");
        exit(1);
    }
    println!("browserhost: presented test surface");

    let mut clicks = 0u32;
    let mut key_seen = false;
    for tick in 0..900u32 {
        loop {
            match window.poll_event() {
                Ok(Some(gui::Event::Close)) => {
                    println!("browserhost: close event");
                    let _ = window.close();
                    exit(0);
                }
                Ok(Some(gui::Event::MouseDown { x, y, .. })) => {
                    clicks = clicks.saturating_add(1);
                    draw(pixels, request_text, clicks, key_seen, true);
                    draw_click_marker(pixels, x as i32, y as i32);
                    let _ = window.present(pixels);
                    println!("browserhost: mouse event");
                }
                Ok(Some(gui::Event::KeyChar { .. })) => {
                    key_seen = true;
                    draw(pixels, request_text, clicks, key_seen, true);
                    let _ = window.present(pixels);
                    println!("browserhost: key event");
                }
                Ok(Some(gui::Event::Resize { width, height })) => {
                    println!("browserhost: resize event");
                    if width as usize == WIDTH && height as usize == HEIGHT {
                        draw(pixels, request_text, clicks, key_seen, true);
                        let _ = window.present(pixels);
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    println!("browserhost: event error");
                    let _ = window.close();
                    exit(1);
                }
            }
        }

        if tick % 60 == 0 {
            draw(pixels, request_text, clicks, key_seen, false);
            let _ = window.present(pixels);
        }
        let _ = window.wait_event_ready(50);
    }

    println!("browserhost: done");
    let _ = window.close();
    exit(0);
}

fn host_log(request: &str) -> heapless_string::String<768> {
    let mut out = heapless_string::String::<768>::new();
    push(&mut out, "coolOS browser engine host log\n");
    push(&mut out, "phase=97\n");
    push(&mut out, "backend=test\n");
    push(&mut out, "process=browserhost\n");
    push(&mut out, "ipc=file-bridge\n");
    push(&mut out, "surface=rgba-gui\n");
    push(&mut out, "input=gui-events\n");
    push(&mut out, "status=ready\n");
    push(&mut out, "request:\n");
    push(&mut out, request);
    out
}

fn draw(
    pixels: &mut [u32; WIDTH * HEIGHT],
    request: &str,
    clicks: u32,
    key_seen: bool,
    event_flash: bool,
) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    for y in 0..HEIGHT {
        let mix = y as u32;
        let r = ((BG_TOP >> 16) & 0xff)
            .saturating_mul((HEIGHT as u32).saturating_sub(mix))
            .saturating_add(((BG_BOTTOM >> 16) & 0xff).saturating_mul(mix))
            / HEIGHT as u32;
        let g = ((BG_TOP >> 8) & 0xff)
            .saturating_mul((HEIGHT as u32).saturating_sub(mix))
            .saturating_add(((BG_BOTTOM >> 8) & 0xff).saturating_mul(mix))
            / HEIGHT as u32;
        let b = (BG_TOP & 0xff)
            .saturating_mul((HEIGHT as u32).saturating_sub(mix))
            .saturating_add((BG_BOTTOM & 0xff).saturating_mul(mix))
            / HEIGHT as u32;
        canvas.rect(0, y as i32, WIDTH as i32, 1, (r << 16) | (g << 8) | b);
    }

    canvas.rect(0, 0, WIDTH as i32, 3, ACCENT);
    canvas.text(18, 18, "coolOS Browser Engine Host", TEXT);
    canvas.text(18, 34, "deterministic Phase 97 test backend", MUTED);

    canvas.rect(18, 58, 444, 74, PANEL);
    canvas.border(18, 58, 444, 74, BORDER);
    canvas.rect(18, 58, 444, 2, ACCENT_ALT);
    canvas.text(34, 76, "BRIDGE", ACCENT);
    canvas.text(34, 94, "load + resize request received", TEXT);
    canvas.text(34, 110, first_request_line(request), MUTED);

    canvas.rect(18, 150, 136, 58, PANEL_ALT);
    canvas.border(18, 150, 136, 58, BORDER);
    canvas.text(34, 168, "SURFACE", ACCENT);
    canvas.text(34, 186, "RGBA GUI", SUCCESS);

    canvas.rect(172, 150, 136, 58, PANEL_ALT);
    canvas.border(172, 150, 136, 58, BORDER);
    canvas.text(188, 168, "INPUT", ACCENT);
    if key_seen {
        canvas.text(188, 186, "KEY EVENT", SUCCESS);
    } else if clicks > 0 {
        canvas.text(188, 186, "MOUSE EVENT", SUCCESS);
    } else {
        canvas.text(188, 186, "WAITING", WARNING);
    }

    canvas.rect(326, 150, 136, 58, PANEL_ALT);
    canvas.border(326, 150, 136, 58, BORDER);
    canvas.text(342, 168, "FALLBACK", ACCENT);
    canvas.text(342, 186, "NATIVE READY", SUCCESS);

    if event_flash {
        canvas.rect(360, 20, 84, 16, WARNING);
        canvas.text(368, 24, "EVENT", BG_BOTTOM);
    }

    canvas.text(18, 230, "HOST.READY + BROWSER-ENGINE-HOST.TXT written", MUTED);
}

fn draw_click_marker(pixels: &mut [u32; WIDTH * HEIGHT], x: i32, y: i32) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    canvas.border(x - 8, y - 8, 16, 16, WARNING);
    canvas.rect(x - 1, y - 6, 2, 12, WARNING);
    canvas.rect(x - 6, y - 1, 12, 2, WARNING);
}

fn first_request_line(request: &str) -> &str {
    request.lines().next().unwrap_or("request=missing")
}

fn push(out: &mut heapless_string::String<768>, text: &str) {
    let _ = out.push_str(text);
}

mod heapless_string {
    use core::fmt;

    pub struct String<const N: usize> {
        len: usize,
        bytes: [u8; N],
    }

    impl<const N: usize> String<N> {
        pub const fn new() -> Self {
            Self {
                len: 0,
                bytes: [0; N],
            }
        }

        pub fn push_str(&mut self, text: &str) -> Result<(), ()> {
            let src = text.as_bytes();
            let room = N.saturating_sub(self.len);
            let n = core::cmp::min(room, src.len());
            if n > 0 {
                self.bytes[self.len..self.len + n].copy_from_slice(&src[..n]);
                self.len += n;
            }
            if n == src.len() {
                Ok(())
            } else {
                Err(())
            }
        }

        pub fn as_bytes(&self) -> &[u8] {
            &self.bytes[..self.len]
        }
    }

    impl<const N: usize> AsRef<[u8]> for String<N> {
        fn as_ref(&self) -> &[u8] {
            self.as_bytes()
        }
    }

    impl<const N: usize> fmt::Display for String<N> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let text = core::str::from_utf8(self.as_bytes()).unwrap_or("");
            f.write_str(text)
        }
    }
}
