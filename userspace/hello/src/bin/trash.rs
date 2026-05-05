#![no_std]
#![no_main]

#[path = "../utilgui.rs"]
mod utilgui;

use libcool::prelude::*;

static mut PIXELS: [u32; utilgui::PIXELS] = [0; utilgui::PIXELS];
static mut LISTING: [u8; utilgui::LIST_BYTES] = [0; utilgui::LIST_BYTES];

libcool::entry!(main);

fn main(args: Args) -> ! {
    let pixels = unsafe { &mut *core::ptr::addr_of_mut!(PIXELS) };
    let listing = unsafe { &mut *core::ptr::addr_of_mut!(LISTING) };
    utilgui::run_trash(args, pixels, listing)
}
