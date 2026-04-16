https://github.com/user-attachments/assets/a6491da6-a8f3-489c-a1ad-bf6abd71e81f
# coolOS

A 64-bit operating system kernel written in Rust. Boots bare-metal into a
graphical desktop with draggable windows, a taskbar, a PS/2 mouse cursor,
and four built-in applications — now with a preemptive scheduler, ring-3
userspace, and a SYSCALL/SYSRET interface.

---

# Current state — v1.9

The kernel boots into a graphical desktop at **1280×720, 24bpp** via a
`bootloader 0.11` linear framebuffer (VBE BIOS path). A terminal window opens
on boot. Right-clicking the desktop opens a context menu to launch additional
apps. A preemptive round-robin scheduler runs three kernel/user tasks driven by
the PIT timer at **100 Hz**:

| Task | Ring | Description |
| :--- | :--- | :---------- |
| **idle/wm** | 0 | The kernel boot stack — runs `compose_if_needed()` + `hlt`. |
| **counter** | 0 | Tight loop incrementing `BACKGROUND_COUNTER`. Visible in System Monitor. |
| **userspace** | 3 | Ring-3 stub: calls `sys_write` (prints to terminal) then `sys_exit`. |

On boot, `[ring 3] Hello from userspace!` appears in the terminal window —
proof that the SYSCALL/SYSRET path and the GDT ring-3 segments are working.

### What's working

| Subsystem | Details |
| :-------- | :------ |
| **Framebuffer** | `bootloader 0.11` linear framebuffer at ≥1280×720. 3bpp and 4bpp both handled. Shadow-buffer compositor — full frame rendered in a heap `Vec<u32>`, blitted per-row with correct bpp conversion. No tearing. |
| **PS/2 mouse** | Full hardware init (CCB, 0xF6/0xF4), 9-bit signed X/Y deltas, IRQ12 packet collection via atomics. |
| **Window manager** | Z-ordered windows, focus-on-click, title-bar drag, close button, per-window pixel back-buffer. |
| **Taskbar** | 24 px bar at the bottom; one button per open window. |
| **Context menu** | Right-click the desktop to spawn any of the four apps. |
| **Heap** | `LockedHeap` allocator — `String`, `Vec`, `Box` all work. 32 MiB heap to accommodate large shadow and window buffers. |
| **Paging** | 4-level `OffsetPageTable` + bootloader E820 frame allocator. All pages marked user-accessible for Phase 9 single-address-space model. |
| **IDT** | Breakpoint, Double Fault, Page Fault, General Protection Fault, Invalid Opcode, Timer (IRQ0), Keyboard (IRQ1), Mouse (IRQ12). |
| **Scheduler** | Preemptive round-robin at 100 Hz. Naked timer ISR saves all 15 GP registers + 5-word CPU interrupt frame, switches `rsp` to the next task's saved context, and `iretq`s into it. 64 KiB heap-allocated kernel stack per task. Idle task reuses the kernel boot stack. |
| **GDT + TSS** | Four segments (kernel code/data ring 0, user code/data ring 3) + TSS with RSP0 pointing to a dedicated 64 KiB ISR stack used when an IRQ fires during ring-3 execution. |
| **SYSCALL/SYSRET** | EFER.SCE enabled. STAR/LSTAR/SFMASK MSRs configured. Naked `syscall_entry` saves context, switches to a dedicated 64 KiB kernel syscall stack, dispatches on rax, restores context, and executes `sysretq`. |
| **Syscall table** | `0 exit`, `1 write`, `2 yield`, `3 getpid`. `sys_write` pushes bytes into a lock-free ring buffer; the compositor drains it into the terminal each frame. |
| **Userspace** | Ring-3 code executes via `jump_to_userspace` (iretq with user CS/SS). The demo stub makes a `write` syscall and an `exit` syscall without triple-faulting. |

### Applications

| App | How to open | Description |
| :-- | :---------- | :---------- |
| **Terminal** | Boot / right-click | Interactive shell. Type commands, press Enter. |
| **System Monitor** | Right-click | Live CPU vendor, heap usage, uptime. |
| **Text Viewer** | Right-click | Scrollable "About" doc; `j`/`k` to scroll. |
| **Color Picker** | Right-click | Clickable 16-colour EGA palette grid. |

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
# macOS:
brew install qemu
```

### Build and run

```bash
make run
```

The build is a two-step process: `cargo build` compiles the kernel ELF, then
`cargo run -p disk-image` wraps it into a BIOS-bootable `bios.img` using
`bootloader 0.11`'s `BiosBoot` builder.

Click inside the QEMU window to capture mouse input. Press **Ctrl+Alt+G** to
release it.

---

## Architecture

```
disk-image/
  src/main.rs      Host tool — wraps kernel ELF into bios.img via bootloader 0.11
src/
  main.rs          Kernel entry point — framebuffer init, GDT, heap, scheduler, main loop
  gdt.rs           GDT (ring-0/ring-3 segments) + TSS (RSP0 for ring-3 IRQ entry)
  interrupts.rs    IDT, PIC, PIT (100 Hz), keyboard/timer(naked)/mouse/fault handlers
  syscall.rs       SYSCALL/SYSRET — naked entry, dispatcher, lock-free output buffer,
                   jump_to_userspace (iretq trampoline)
  userspace.rs     Ring-3 demo task — user_stub (write + exit syscalls)
  memory.rs        Page table init, physical frame allocator, mark_all_user_accessible
  allocator.rs     Heap allocator (linked_list_allocator, 32 MiB)
  scheduler.rs     Preemptive scheduler — Task, Scheduler, SCHEDULER global, timer_schedule
  framebuffer.rs   Linear framebuffer driver — 3bpp/4bpp, draw_char, scroll
  vga_buffer.rs    Text layer over framebuffer — used by print!/panic handler
  mouse.rs         PS/2 mouse hardware init and packet decoder
  keyboard.rs      Lock-free ring buffer — IRQ handler pushes chars, compositor drains
  wm/
    mod.rs         Public WM API — request_repaint, compose_if_needed
    compositor.rs  WindowManager — shadow buffer, z-order, drag, taskbar,
                   context menu, syscall output drain, AppWindow enum dispatch, blit
    window.rs      Window struct — back-buffer, hit tests
  apps/
    terminal.rs    TerminalApp — keyboard input, shell commands, text render
    sysmon.rs      SysMonApp   — live CPU/heap/uptime/scheduler display
    textviewer.rs  TextViewerApp — scrollable static text
    colorpicker.rs ColorPickerApp — clickable EGA palette swatches
```

---

## Design notes

**Why Rust?** The `#[no_std]` ecosystem is mature enough for kernel work, and
memory safety at the kernel level eliminates whole categories of bugs
(use-after-free, buffer overflows) that make C kernel development painful. The
borrow checker enforces ownership of hardware resources at compile time.

**`bootloader 0.11` and the disk-image tool.** The old `bootimage` approach
(bootloader 0.9.x + `cargo bootimage`) shipped a fixed 320×200 VGA framebuffer.
Phase 6 replaced it with a host-side `disk-image` crate that calls
`BiosBoot::new(&kernel).set_boot_config(&cfg).create_disk_image(...)`,
requesting ≥1280×720. The bootloader negotiates a VBE mode with QEMU's SeaBIOS
and hands the kernel a `FrameBufferInfo` struct at boot time.

**3bpp vs 4bpp.** QEMU's standard VGA (`-vga std`) delivers a 24bpp (3
bytes/pixel) framebuffer even when 32bpp is requested. The compositor and all
direct-write paths now check `bytes_per_pixel` at runtime and write 3 or 4
bytes per pixel accordingly. The shadow buffer stays `u32` (0x00RRGGBB)
throughout; the bpp conversion happens only at blit time.

**Shadow-buffer compositing** renders the full frame — desktop fill, windows
back-to-front, cursor sprite — into a heap-allocated `Vec<u32>`, then blits
the finished frame to the hardware framebuffer row by row. The display sees
only complete frames, eliminating tearing and partial redraws.

**Preemptive scheduler (Phase 8).** The timer ISR (`timer_naked` in
`interrupts.rs`) is a `#[unsafe(naked)]` function using `naked_asm!`. It
pushes all 15 GP registers onto the current stack, calls `timer_inner` (which
handles ticks/repaint/EOI and delegates to `scheduler::timer_schedule`), then
does `mov rsp, rax` to switch to the winning task's stack before popping its
registers and executing `iretq`. New tasks are given a 64 KiB heap-allocated
kernel stack pre-populated with a fake 20-word interrupt frame so the first
`iretq` drops straight into the entry function.

**Ring-3 userspace (Phase 9).** The GDT now has four segments (kernel code
0x08, kernel data 0x10, user data 0x18, user code 0x20) plus a TSS whose RSP0
points to a dedicated 64 KiB ISR stack used when an IRQ fires during ring-3
execution. SYSCALL/SYSRET is enabled via EFER.SCE; STAR is set so that
SYSCALL enters kernel CS=0x08/SS=0x10 and SYSRET returns to user
CS=0x20/SS=0x18. The naked `syscall_entry` stub saves user RSP in r10,
switches to a private 64 KiB kernel syscall stack, builds a register frame,
calls the Rust `syscall_dispatch`, and restores with `pop rsp` + `sysretq`.
`sys_write` output goes through a lock-free ring buffer (same pattern as the
keyboard queue) that the compositor drains into the terminal each frame —
avoiding the deadlock that would result from locking WM from syscall context.
Phase 9 uses a single shared address space (all pages marked user-accessible);
Phase 10 will introduce per-process page tables.

---

## Roadmap

| Phase | Deliverable | Status |
| :---: | :---------- | :----- |
| 1 | Pixel framebuffer + font rendering | **Done** |
| 2 | PS/2 mouse driver + on-screen cursor | **Done** |
| 3 | Window manager — draggable windows, focus, close | **Done** |
| 4 | Desktop shell — taskbar, context menu, terminal app | **Done** |
| 5 | Applications — system monitor, text viewer, color picker | **Done** |
| 6 | High-resolution framebuffer via `bootloader 0.11` (1280×720) | **Done** |
| 7 | Input lag fixes — lock-free keyboard queue, scratch-buffer blit, release build | **Done** |
| 8 | Preemptive scheduler + context switching (100 Hz PIT) | **Done** |
| 9 | Ring-3 userspace + SYSCALL/SYSRET interface | **Done** |
| 10 | Per-process virtual memory + isolation | Planned |
| 11 | Filesystem (FAT32) + VFS + disk driver | Planned |
| 12 | ELF loader — real programs run from disk | Planned |
| 13 | Pipes + shared memory + IPC | Planned |
| 14 | USB HID — real hardware input | Planned |
| 15 | Networking — virtio-net, TCP/IP | Planned |

Full task checklists and technical notes in [ROADMAP.md](ROADMAP.md).
