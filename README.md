# coolOS 🚀

A minimal, 64-bit operating system kernel written in Rust. 

This project demonstrates the fundamentals of OS development, including hardware interrupt handling, VGA text-mode graphics, and a basic interactive shell.

## 🛠 Features

* **Custom IDT**: Handles CPU exceptions (Breakpoint, Double Fault).
* **Hardware Interrupts**: Manages the 8259 PIC for Timer and Keyboard events.
* **VGA Driver**: A thread-safe VGA buffer writer with support for scrolling and colors.
* **Interactive Shell**: A basic CLI that processes user input (try the `clear` command!).
* **No Standard Library**: Built entirely with `#[no_std]` and `#[no_main]`.

## 🚀 Getting Started

### Prerequisites

You will need the Rust nightly toolchain and the following components:

```bash
rustup component add rust-src
cargo install bootimage
brew install qemu  # For macOS users