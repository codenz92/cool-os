use crate::{print, println};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
    fn as_usize(self) -> usize {
        self as usize
    }
}

static mut COMMAND_BUFFER: [char; 80] = ['\0'; 80];
static mut COMMAND_IDX: usize = 0;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(sf: InterruptStackFrame, _err: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", sf);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: spin::Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            spin::Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    match character {
                        '\n' => {
                            println!();
                            process_command();
                        }
                        '\u{0008}' => {
                            // Backspace
                            unsafe {
                                if COMMAND_IDX > 0 {
                                    COMMAND_IDX -= 1;
                                    COMMAND_BUFFER[COMMAND_IDX] = '\0';
                                    crate::vga_buffer::backspace();
                                }
                            }
                        }
                        c => {
                            print!("{}", c);
                            unsafe {
                                if COMMAND_IDX < 79 {
                                    COMMAND_BUFFER[COMMAND_IDX] = c;
                                    COMMAND_IDX += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

fn process_command() {
    unsafe {
        if match_command("clear") {
            crate::vga_buffer::clear_screen();
        } else if match_command("help") {
            println!("--- coolOS Help Menu ---");
            println!("clear  - Wipes the display");
            println!("reboot - Restarts the system");
            println!("help   - Shows this menu");
        } else if match_command("reboot") {
            reboot();
        } else if COMMAND_IDX > 0 {
            println!("Unknown command");
        }
        COMMAND_IDX = 0;
        for i in 0..80 {
            COMMAND_BUFFER[i] = '\0';
        }
        print!("> ");
    }
}

fn match_command(cmd: &str) -> bool {
    unsafe {
        let cmd_bytes = cmd.as_bytes();
        if COMMAND_IDX != cmd_bytes.len() {
            return false;
        }
        for i in 0..cmd_bytes.len() {
            if COMMAND_BUFFER[i] as u8 != cmd_bytes[i] {
                return false;
            }
        }
        true
    }
}

fn reboot() {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x64);
    unsafe {
        port.write(0xFEu8);
    }
}
