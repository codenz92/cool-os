#![no_std]
#![no_main]

use libcool::{gui, prelude::*};

const WIDTH: usize = 320;
const HEIGHT: usize = 180;
const BG: u32 = 0x0004_0c18;
const PANEL: u32 = 0x0008_1828;
const CYAN: u32 = 0x0088_ffcc;
const BLUE: u32 = 0x0024_70ff;
const WHITE: u32 = 0x00e8_fff8;
const MUTED: u32 = 0x0068_9db4;
const YELLOW: u32 = 0x00ff_d866;

static mut PIXELS: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

libcool::entry!(main);

fn main(_args: Args) -> ! {
    let mut window = match gui::Window::open(b"guidemo", WIDTH as u16, HEIGHT as u16) {
        Ok(window) => window,
        Err(_) => {
            println!("guidemo: open failed");
            exit(1);
        }
    };
    println!("guidemo: window opened");

    let pixels = unsafe { &mut *core::ptr::addr_of_mut!(PIXELS) };
    draw(pixels, 0, 0);
    if window.present(pixels).is_err() {
        println!("guidemo: present failed");
        exit(1);
    }
    println!("guidemo: presented frame");

    let mut clicks = 0u32;
    for tick in 0..180u32 {
        loop {
            match window.poll_event() {
                Ok(Some(gui::Event::Close)) => {
                    println!("guidemo: close event");
                    let _ = window.close();
                    exit(0);
                }
                Ok(Some(gui::Event::MouseDown { x, y, .. })) => {
                    clicks = clicks.saturating_add(1);
                    draw(pixels, clicks, tick);
                    draw_click_marker(pixels, x as i32, y as i32);
                    let _ = window.present(pixels);
                    println!("guidemo: mouse event");
                }
                Ok(Some(gui::Event::KeyChar { .. })) => {
                    draw(pixels, clicks, tick);
                    draw_key_flash(pixels);
                    let _ = window.present(pixels);
                    println!("guidemo: key event");
                }
                Ok(Some(gui::Event::Resize { width, height })) => {
                    println!("guidemo: resize event");
                    if width as usize == WIDTH && height as usize == HEIGHT {
                        draw(pixels, clicks, tick);
                        let _ = window.present(pixels);
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    println!("guidemo: event error");
                    let _ = window.close();
                    exit(1);
                }
            }
        }

        if tick % 30 == 0 {
            draw(pixels, clicks, tick);
            let _ = window.present(pixels);
        }
        sleep_ms(50);
    }

    println!("guidemo: done");
    let _ = window.close();
    exit(0);
}

fn draw(pixels: &mut [u32; WIDTH * HEIGHT], clicks: u32, tick: u32) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    canvas.clear(BG);
    canvas.rect(0, 0, WIDTH as i32, 4, CYAN);
    canvas.rect(0, 4, WIDTH as i32, 22, 0x0001_1b2d);
    canvas.text(14, 11, "USERSPACE GUI DEMO", WHITE);
    canvas.text(14, 34, "RING 3 OWNS THESE PIXELS", CYAN);
    canvas.text(14, 48, "WM OWNS CHROME + EVENTS", MUTED);

    canvas.rect(14, 70, 128, 54, PANEL);
    canvas.border(14, 70, 128, 54, BLUE);
    canvas.text(26, 84, "DRAW RECT", WHITE);
    canvas.rect(26, 100, 104, 10, CYAN);

    canvas.rect(156, 70, 148, 54, PANEL);
    canvas.border(156, 70, 148, 54, BLUE);
    canvas.text(168, 84, "EVENT QUEUE", WHITE);
    if clicks == 0 {
        canvas.text(168, 100, "CLICK WINDOW", YELLOW);
    } else {
        canvas.text(168, 100, "MOUSE OK", CYAN);
    }

    let x = 18 + ((tick as usize * 5) % (WIDTH - 64)) as i32;
    canvas.rect(x, 144, 42, 12, 0x00ff_66aa);
    canvas.border(x - 2, 142, 46, 16, WHITE);
    canvas.text(14, 164, "libcool::gui open/present/poll/close", MUTED);
}

fn draw_click_marker(pixels: &mut [u32; WIDTH * HEIGHT], x: i32, y: i32) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    canvas.border(x - 8, y - 8, 16, 16, YELLOW);
    canvas.rect(x - 1, y - 6, 2, 12, YELLOW);
    canvas.rect(x - 6, y - 1, 12, 2, YELLOW);
}

fn draw_key_flash(pixels: &mut [u32; WIDTH * HEIGHT]) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    canvas.rect(244, 10, 54, 14, YELLOW);
    canvas.text(250, 13, "KEY", BG);
}
