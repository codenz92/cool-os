#![no_std]
#![no_main]

#[path = "../utilgui.rs"]
mod utilgui;

use libcool::prelude::*;

static mut PIXELS: [u32; utilgui::PIXELS] = [0; utilgui::PIXELS];

libcool::entry!(main);

fn main(args: Args) -> ! {
    let pixels = unsafe { &mut *core::ptr::addr_of_mut!(PIXELS) };
    utilgui::run_screenshot(args, pixels)
}
