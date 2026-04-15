# coolOS

A 64-bit operating system kernel written in Rust. Boots bare-metal into a
graphical desktop with draggable windows, a taskbar, a PS/2 mouse cursor,
and four built-in applications — all running as kernel-mode code with no
scheduler and no userspace. Yet.

---

## Current state — v1.5

The kernel boots directly into a graphical desktop at **320×200, 256 colours**
(VGA Mode 13h). A terminal window opens on boot. Right-clicking the desktop
opens a context menu to launch additional apps.

### What's working

| Subsystem | Details |
| :-------- | :------ |
| **Framebuffer** | VGA Mode 13h, 8bpp, shadow-buffer compositor — full frame rendered in RAM, blitted to `0xA0000` in one `memcpy`. No tearing. |
| **PS/2 mouse** | Full hardware init (CCB, 0xF6/0xF4), 9-bit signed X/Y deltas, IRQ12 packet collection via atomics. |
| **Window manager** | Z-ordered windows, focus-on-click, title-bar drag, close button, per-window pixel back-buffer. |
| **Taskbar** | 12 px bar at the bottom; one button per open window. |
| **Context menu** | Right-click the desktop to spawn any of the four apps. |
| **Heap** | `LockedHeap` allocator — `String`, `Vec`, `Box` all work. |
| **Paging** | 4-level `OffsetPageTable` + bootloader E820 frame allocator. |
| **IDT** | Breakpoint, Double Fault, Timer (IRQ0), Keyboard (IRQ1), Mouse (IRQ12). |

### Applications

| App | How to open | Description |
| :-- | :---------- | :---------- |
| **Terminal** | Boot / right-click | Interactive shell. Type commands, press Enter. |
| **System Monitor** | Right-click | Live CPU vendor, heap usage, uptime. |
| **Text Viewer** | Right-click | Scrollable "About" doc; `j`/`k` to scroll. |
| **Color Picker** | Right-click | Clickable 16-colour VGA palette grid. |

### Terminal commands

| Command | Description |
| :------ | :---------- |
| `help` | List available commands |
| `echo <text>` | Print text |
| `info` | CPU vendor and heap usage |
| `uptime` | Timer ticks and seconds since boot |
| `clear` | Clear the terminal |
| `reboot` | Hardware reset |

---

## Getting started

### Prerequisites

```bash
rustup toolchain install nightly
rustup component add rust-src
cargo install bootimage
# macOS:
brew install qemu
```

### Build and run

```bash
make run
```

Click inside the QEMU window to capture mouse input. Press **Ctrl+Alt+G** to release it.

---

## Architecture

```
src/
  main.rs            Kernel entry point — heap init, window setup, main loop
  interrupts.rs      IDT, PIC, keyboard/timer/mouse handlers
  memory.rs          Page table init, physical frame allocator
  allocator.rs       Heap allocator (linked_list_allocator wrapper)
  framebuffer.rs     VGA Mode 13h pixel driver — put_pixel, draw_char, scroll
  vga_buffer.rs      Text layer over framebuffer — used by panic handler
  mouse.rs           PS/2 mouse hardware init and packet decoder
  wm/
    mod.rs           Public WM API — request_repaint, compose_if_needed
    compositor.rs    WindowManager — shadow buffer, z-order, drag, taskbar,
                     context menu, AppWindow enum dispatch
    window.rs        Window struct — back-buffer, hit tests
  apps/
    terminal.rs      TerminalApp — keyboard input, shell commands, text render
    sysmon.rs        SysMonApp   — live CPU/heap/uptime display
    textviewer.rs    TextViewerApp — scrollable static text
    colorpicker.rs   ColorPickerApp — clickable VGA palette swatches
```

All app code runs in kernel mode (ring 0). There is no scheduler, no privilege
separation, and no system call interface. That is what the roadmap is for.

---

## Roadmap

| Phase | Deliverable | Status |
| :---: | :---------- | :----- |
| 1 | Pixel framebuffer + font rendering | **Done** |
| 2 | PS/2 mouse driver + on-screen cursor | **Done** |
| 3 | Window manager — draggable windows, focus, close | **Done** |
| 4 | Desktop shell — taskbar, context menu, terminal app | **Done** |
| 5 | Applications — system monitor, text viewer, color picker | **Done** |
| 6 | High-resolution framebuffer (Limine / UEFI GOP) | Planned |
| 7 | Preemptive scheduler + context switching | Planned |
| 8 | Ring-3 userspace + syscall interface | Planned |
| 9 | Per-process virtual memory + isolation | Planned |
| 10 | Filesystem (FAT32) + VFS + disk driver | Planned |
| 11 | ELF loader — real programs run from disk | Planned |
| 12 | Pipes + shared memory + IPC | Planned |
| 13 | USB HID — real hardware input | Planned |
| 14 | Networking — virtio-net, TCP/IP | Planned |

Full task checklists and technical notes in [ROADMAP.md](ROADMAP.md).

---

## Design notes

**Why Rust?** The `#[no_std]` ecosystem is mature enough for kernel work, and
memory safety at the kernel level eliminates whole categories of bugs (use-after-free,
buffer overflows) that make C kernel development painful. The borrow checker enforces
ownership of hardware resources at compile time.

**Why Mode 13h (for now)?** The `bootloader 0.9.x` crate's `vga_320x200` feature is
the lowest-friction path to a pixel framebuffer. Upgrading to a modern UEFI/Limine
framebuffer (Phase 6) requires rewriting `framebuffer.rs`, the compositor's layout
constants, and the font renderer — it's a deliberate, self-contained migration.

**Shadow-buffer compositing** renders the full frame — desktop fill, windows
back-to-front, cursor sprite — into a heap-allocated buffer, then blits the finished
frame to VGA in a single `ptr::copy_nonoverlapping`. The display sees only complete
frames, eliminating tearing and partial redraws.

**No processes (yet)** — all apps are Rust structs dispatched from the WM's main
loop. Phase 7 (scheduler) and Phase 8 (userspace) replace this with real concurrent
processes. Until then, a crash in any app takes down the whole kernel, which is
expected and fine for this stage of development.
