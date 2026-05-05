#![no_std]
#![no_main]

#[path = "../utilgui.rs"]
mod utilgui;

use libcool::prelude::*;

static mut PIXELS: [u32; utilgui::PIXELS] = [0; utilgui::PIXELS];
static mut TEXT: [u8; utilgui::MAX_TEXT] = [0; utilgui::MAX_TEXT];

libcool::entry!(main);

fn main(args: Args) -> ! {
    let pixels = unsafe { &mut *core::ptr::addr_of_mut!(PIXELS) };
    let text = unsafe { &mut *core::ptr::addr_of_mut!(TEXT) };
    utilgui::run_editor(
        args,
        pixels,
        text,
        "Text Editor",
        b"/Documents/EDITOR.TXT",
        b"editor",
        b"Phase 22 editor smoke\n",
    )
}
