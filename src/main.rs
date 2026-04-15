#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod allocator;
mod framebuffer;
mod interrupts;
mod memory;
mod mouse;
mod vga_buffer;
mod wm;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    // Add two demo windows.
    wm::add_window(wm::window::Window::new(20, 20, 120, 80, "Terminal"));
    wm::add_window(wm::window::Window::new(60, 50, 100, 60, "System Info"));

    mouse::init();
    wm::init(); // triggers first repaint

    loop {
        x86_64::instructions::interrupts::without_interrupts(|| {
            wm::compose_if_needed();
        });
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
