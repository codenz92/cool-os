<img width="1392" height="864" alt="image" src="https://github.com/user-attachments/assets/5824d8c0-d0d8-4b79-b650-9fec05fa0c70" />


# coolOS

A 64-bit operating system kernel written in Rust. Boots bare-metal into a
graphical desktop with draggable and resizable windows, a taskbar, a start
menu, hardware input, kernel and userspace GUI applications, a preemptive
scheduler, ring-3 userspace, per-process virtual memory, process isolation, a
FAT32 filesystem with VFS/syscalls, a native CoolFS mount, an ELF loader with
`exec`, and IPC with pipes, shared memory, and per-task fd tables.

---

# Current state — v5.6

The kernel boots into a graphical desktop at **1280×720, 24bpp** via a
`bootloader 0.11` linear framebuffer (VBE BIOS path). A terminal window opens
on boot. Right-clicking the desktop opens a context menu to launch additional
apps, and the shell also exposes desktop icons plus a start menu/taskbar flow,
global keyboard shortcuts, a `Ctrl+Space` launcher/search palette, a task
switcher overlay, edge/keyboard window snapping, taskbar previews/actions,
session restore, File Manager drag/drop between windows, a shared clipboard,
notification center, and desktop settings that persist to the FAT32 disk.
A preemptive round-robin scheduler runs five boot tasks driven by the PIT
timer at **100 Hz**; the terminal can also spawn additional ring-3 ELF tasks
from disk with `exec`:

| Task | Ring | Description |
| :--- | :--- | :---------- |
| **idle/wm** | 0 | The kernel boot stack — runs `compose_if_needed()` + `hlt`. |
| **counter** | 0 | Tight loop incrementing `BACKGROUND_COUNTER`. Visible in System Monitor. |
| **fs-test** | 0 | One-shot: reads `/bin/hello.txt` from disk and prints it, then blocks. |
| **user1** | 3 | Own PML4 + private user stack. Writes sentinel `0xDEAD0001`, reads it back, prints `[ring3 pid=1] sentinel ok`. |
| **user2** | 3 | Own PML4 + private user stack at the same VA as user1. Writes `0xDEAD0002` — cannot see user1's memory. |

On boot, the contents of `/bin/hello.txt` are printed to the console by the
`fs-test` task. Both `[ring3 pid=1] sentinel ok` and `[ring3 pid=2] sentinel ok`
appear in the terminal, proving process isolation: same virtual address,
different physical frames. Typing `exec /bin/hello` launches a real userspace
ELF from disk, `exec /bin/exec` demonstrates `sys_exec` by replacing a running
userspace image with `/bin/hello`, `exec /bin/pipe` exercises per-process
anonymous pipes, `ipc` allocates a fresh shared pipe and launches the
reader/writer demo pair by inheriting one pipe end into each child, `keydemo`
streams focused terminal keystrokes into a userspace process over an inherited
pipe, and `exec /bin/read` exercises userspace `open/read/close` against the
FAT32-backed VFS. With QEMU virtio networking enabled, `exec /bin/wget
http://example.com/` resolves DNS, opens a TCP socket, fetches the HTTP
response, and streams it to the terminal. The native Web Browser app can open
HTTP, HTTPS, and local HTML pages, follow redirects, decode chunked responses,
render headings, lists, block quotes, simple tables, and bounded inline PNG
images, and keep session history plus persistent local bookmarks. HTTPS uses a
no_std TLS 1.3 client over the kernel TCP stack with hardware RNG entropy,
RTC-backed certificate validity checks, X.509 chain validation against the
built-in trust roots, and SAN-first hostname validation coverage.

### What's working

| Subsystem | Details |
| :-------- | :------ |
| **Framebuffer** | `bootloader 0.11` linear framebuffer at ≥1280×720. 3bpp and 4bpp both handled. Shadow-buffer compositor — full frame rendered in a heap `Vec<u32>`, blitted per-row with correct bpp conversion. No tearing. |
| **PS/2 mouse** | Full hardware init (CCB, 0xF6/0xF4), 9-bit signed X/Y deltas, IRQ12 packet collection via atomics. |
| **Window manager** | Z-ordered windows, focus-on-click, title-bar drag, edge snapping, keyboard snapping, task switcher overlay, minimise/maximise/restore, resize grip, close button, taskbar previews/right-click actions, per-window pixel back-buffer. |
| **Desktop shell** | Wallpaper, desktop icons, right-click context menu, start menu, taskbar window buttons, configurable shortcuts, launcher/search palette, notification center, File Manager drag/drop, shared clipboard plumbing, persistent settings, session restore, and clock. |
| **Heap** | `LockedHeap` allocator — `String`, `Vec`, `Box` all work. 32 MiB heap to accommodate large shadow and window buffers. |
| **Paging / VMM** | 4-level `OffsetPageTable` + global `BootInfoFrameAllocator`. Per-process PML4 cloned from kernel upper half; private user-space mappings in lower half. `vmm::` module exposes `new_process_pml4`, `map_page_in`, `map_region`, `switch_to`. |
| **IDT** | Breakpoint, Double Fault, Page Fault (lazy allocator for user faults), General Protection Fault, Invalid Opcode, Timer (IRQ0), Keyboard (IRQ1), Mouse (IRQ12). |
| **Scheduler** | Preemptive round-robin at 100 Hz. Each task carries `pml4: Option<PhysFrame>`; the scheduler calls `vmm::switch_to` on context switch when `Some`. 64 KiB heap-allocated kernel stack per task. Task lifecycle now distinguishes ready/running/blocked/exited/reaped states, records parents and exit codes, supports `waitpid`/reaping, supports kernel-side task termination, and backs blocking pipe reads with `block_current` / `unblock(id)`. |
| **Process isolation** | Two user processes share the same user-stack virtual address (`0x7FFF_0010_0000`) but map it to different physical frames. Guard pages (kernel-only) sit below each stack. |
| **GDT + TSS** | Four segments (kernel code/data ring 0, user code/data ring 3) + TSS with RSP0 pointing to a dedicated 64 KiB ISR stack used when an IRQ fires during ring-3 execution. |
| **SYSCALL/SYSRET** | EFER.SCE enabled. STAR/LSTAR/SFMASK MSRs configured. Naked `syscall_entry` saves context, switches to the currently scheduled task's private kernel stack top, dispatches on rax, restores context, and executes `sysretq`. |
| **Syscall table** | `0 exit`, `1 write`, `2 yield`, `3 getpid`, `4 mmap(addr, len, flags)`, `5 open(path, len)`, `6 read(fd, buf, len)`, `7 close(fd)`, `8 exec(path, len)`, `9 pipe(fds_ptr)`, `10 dup(fd)`, `11 shmem_create(len)`, `12 shmem_map(id)`, `13 waitpid(pid, status_ptr)`, `14 spawn(path, len)`, `15 sleep_ms(ms)`, `16 abi_version()`, `17 dns_resolve(host, len)`, `18 http_get(host, len)`, `19 socket(domain, type, proto)`, `20 connect(socket, ipv4, port)`, `21 send(socket, buf, len)`, `22 recv(socket, buf, len)`, `23 gui_open(title, len, dims)`, `24 gui_present(handle, pixels, len)`, `25 gui_poll_event(handle, packet, len)`, `26 gui_close(handle)`, `27 fs_write_file(desc)`, `28 fs_create_dir(path, len)`, `29 fs_delete_tree(path, len)`, `30 fs_list_dir(desc)`, `31 screenshot(path, len, flags)`. `sys_write` writes to stdout/stderr through the compositor ring buffer or to pipe write-ends through the VFS fd table. |
| **Userspace** | Ring-3 code can run either as the original isolation stubs or as real ELF64 binaries loaded from `/bin`. The `libcool` SDK crate now provides no_std entry/argv setup plus process, file, pipe, mmap, shared-memory, event, DNS/HTTP, TCP socket, filesystem utility, screenshot, and userspace GUI wrappers. `sys_exec` replaces the current userspace image in-place by swapping CR3 and rewriting the saved syscall return frame. `keydemo` relays fixed-size key and click event packets into a userspace child over a pipe. Shared memory (`sys_shmem_create`/`sys_shmem_map`) maps a region of physical frames into the caller's address space at a fixed VA. |
| **ELF loader** | Validates ELF64 headers, maps `PT_LOAD` segments into a fresh address space, allocates a private user stack, builds an initial `argc/argv/envp` stack frame, and can either spawn a new task or prepare an image for `sys_exec`. |
| **ATA PIO driver** | Primary-bus slave device (QEMU `if=ide,index=1`). LBA28 PIO reads, BSY/DRQ polling with timeout, nIEN=1 (device interrupts disabled). Wrapped in `without_interrupts` to prevent preemption mid-transfer. |
| **FAT32 layer** | BPB parsing, FAT chain walking, short-name and long-filename lookup, directory traversal, cluster→sector mapping, create/write/rename/delete/copy helpers, temp-write+rename safe writes, free-space stats, and a basic `fsck` consistency summary. `fat32::read_file(path)` returns `Option<Vec<u8>>`. |
| **CoolFS layer** | Native coolOS filesystem mounted at `/COOL`, backed by `/COOLFS.IMG`. It has a CoolFS superblock, fixed inode table, block bitmap, direct data blocks, directory records, VFS read/write/create/rename/delete/copy routing, stats, and boot self-tests. |
| **VFS** | Task-local fd tables (16 slots, fds 0–2 reserved) backed by shared file/pipe/shmem objects. `vfs_open` reads whole files into heap buffers; `vfs_pipe` allocates a 512-byte kernel ring buffer and returns per-task read/write fds; `vfs_read_blocking` blocks tasks on empty pipes and wakes them on write/EOF; `ipc` selectively inherits pipe ends into child processes; `vfs_shmem_create`/`vfs_shmem_map` manage a shared memory region pool indexed by ID. |
| **Networking** | Legacy PCI virtio-net driver for QEMU user networking, polling RX/TX virtqueues, Ethernet framing, ARP cache, IPv4, ICMP echo, UDP DNS queries, minimal TCP client sockets, userspace socket syscalls, HTTP/1.1, and verified TLS 1.3 HTTPS for the native browser/terminal path. |
| **Kernel services** | Persistent kernel log buffer flushed to `/LOGS/KERNEL.TXT`, crash-screen log tail, central device registry for PCI/USB/system devices, package/app metadata and file associations, networking status, and ACPI power-control status foundation. |
| **Applications** | Terminal, System Monitor, Text Viewer, Color Picker, File Manager, Web Browser, ring-3 Notes, Text Editor, Trash Bin, Screenshot, and GUI Demo. |
| **Disk image** | `disk-image/src/fs_image.rs` builds `fs.img` (64 MiB FAT32) with `/COOLFS.IMG`, `/bin/hello.txt`, `/bin/hello`, `/bin/exec`, `/bin/pipe`, `/bin/piperd`, `/bin/pipewr`, `/bin/keyecho`, `/bin/read`, `/bin/terminal`, `/bin/netdemo`, `/bin/wget`, `/bin/sdkdemo`, `/bin/guidemo`, `/bin/notes`, `/bin/editor`, `/bin/trash`, and `/bin/screenshot`. The Makefile attaches it to QEMU as the IDE slave. |

### Applications

| App | How to open | Description |
| :-- | :---------- | :---------- |
| **Terminal** | Launcher / right-click | Interactive shell. Type commands, press Enter. |
| **System Monitor** | Right-click | Live CPU vendor, heap usage, uptime, scheduler counts, and USB/input status. |
| **Text Viewer** | Right-click | Scrollable "About" doc; `j`/`k` to scroll. |
| **Color Picker** | Right-click | Clickable 16-colour EGA palette grid. |
| **File Manager** | Right-click / desktop icon | Browse and mutate the FAT32 disk image with breadcrumbs, recursive search, sorting, multi-select, clipboard copy/cut/paste, Trash-backed delete, properties, text editing, and ELF launch routing. |
| **Web Browser** | Launcher / desktop icon | Native HTTP/HTTPS/local-file browser with address/search bar, redirects, decoded chunked responses, headings/lists/quotes/tables, direct and HTML-sourced inline PNG previews, clickable links, session history, visible TLS trust-root status, and persistent bookmarks. |
| **Trash Bin** | Launcher / desktop icon / `exec /bin/trash` | Ring-3 GUI utility that lists deleted items staged in `/Trash` and can permanently empty them. |
| **Screenshot** | Launcher / desktop icon / `exec /bin/screenshot` | Ring-3 GUI utility that queues a focused-window PPM capture to `/Pictures`. |
| **Notes** | Launcher / desktop icon / `exec /bin/notes` | Ring-3 persistent scratchpad backed by `/Documents/NOTES.TXT`. |
| **Text Editor** | Launcher / desktop icon / `exec /bin/editor` | Ring-3 persistent text editor backed by `/Documents/EDITOR.TXT`, with save and cursor controls. |
| **GUI Demo** | Launcher / `exec /bin/guidemo` | First ring-3 windowed app. It opens a compositor window, presents its own pixel buffer, and polls keyboard/mouse/close events through `libcool::gui`. |

### Desktop shortcuts

| Shortcut | Action |
| :------- | :----- |
| **Ctrl+Space** | Open launcher/search palette for apps, paths, and commands. |
| **Ctrl+Alt+M** | Toggle notification center. |
| **Alt+Tab** | Cycle focus to the previous visible window. |
| **Alt+F4** | Close the focused window. |
| **F5** | Refresh desktop state and rebuild the wallpaper. |
| **Ctrl+Alt+Left/Right** | Snap the focused window to the left or right half. |
| **Ctrl+Alt+Up/Down** | Snap the focused window to full height or the bottom half. |
| **Ctrl+Esc** | Toggle the start menu. |
| **Ctrl+W** | Close the focused window. |
| **Ctrl+F** | Open File Manager at `/`. |
| **Ctrl+N** | Open Terminal. |
| **Ctrl+R** | Refresh desktop state and rebuild the wallpaper. |

Shortcut bindings are loaded from `/CONFIG/SHORTCUT.CFG`. Display Settings and
Personalize write their state to `/CONFIG/DESK.CFG`, and the compositor writes
window session state to `/CONFIG/SESSION.CFG`, so desktop state survives reboot.

### Terminal commands

| Command | Description |
| :------ | :---------- |
| `help` | List available commands |
| `echo <text>` | Print text |
| `exec <path> [args...]` | Load a userspace ELF from disk and spawn it with argv |
| `ps` | List scheduler tasks, ring, status, and exit codes |
| `kill <pid>` | Terminate a non-idle task and mark it exited |
| `wait <pid>` | Reap an exited task and print its exit code |
| `reap` | Reap all exited tasks visible to the shell |
| `ipc` | Create a shared pipe and launch the userspace reader/writer demo |
| `keydemo` | Send fixed-size WM key and click event packets to `/bin/keyecho` over an inherited pipe until `~` closes it |
| `term` | Launch a userspace terminal as a ring-3 process with pipe-based input (Ctrl+D to exit) |
| `info` | CPU vendor and heap usage |
| `devices` | Print the central PCI/USB/system device registry |
| `net` | Print network adapter/protocol status |
| `netproto` | Print ARP/IPv4/ICMP/UDP/TCP protocol status |
| `dns <host>` | Resolve a host through the network DNS API |
| `ping <host>` | Send an ICMP echo request |
| `http <host-or-url> [path]` | Fetch an HTTP response through the kernel network API |
| `https <host-or-url> [path]` | Fetch an HTTPS response through the verified kernel TLS client |
| `power [reboot\|shutdown\|sleep]` | Print or request power-control actions |
| `log` | Flush and print the kernel log tail |
| `fsck` | Print FAT32 consistency and root-entry summary |
| `coolfs` | Print CoolFS mount and inode/block usage |
| `df` | Print FAT32 and CoolFS used/free/total space |
| `shortcuts` | Print configured global shortcuts |
| `clip [text]` | Show clipboard summary or copy text |
| `paste` | Paste shared clipboard text |
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

For virtio networking in QEMU:

```bash
make run-net
```

The build process compiles the kernel ELF, compiles the userspace ELF binaries
in `userspace/hello/`, wraps the kernel into a BIOS-bootable `bios.img`, and
builds `fs.img` with the userspace binaries embedded into `/bin`.

Click inside the QEMU window to capture mouse input. Press **Ctrl+Alt+G** to
release it.

---

## Architecture

```
disk-image/
  src/main.rs      Host tool — wraps kernel ELF into bios.img via bootloader 0.11
  src/fs_image.rs  Host tool — builds fs.img (64 MiB FAT32) with /bin/hello.txt,
                    /bin/hello, /bin/exec, /bin/pipe, /bin/piperd, /bin/pipewr,
                    /bin/keyecho, /bin/read, /bin/terminal, /bin/netdemo, /bin/wget,
                    /bin/sdkdemo, /bin/guidemo, /bin/notes, /bin/editor,
                    /bin/trash, and /bin/screenshot
src/
  main.rs          Kernel entry point — framebuffer init, GDT, heap, scheduler, main loop
  gdt.rs           GDT (ring-0/ring-3 segments) + TSS (RSP0 for ring-3 IRQ entry)
  interrupts.rs    IDT, PIC, PIT (100 Hz), IRQ masking, keyboard/timer(naked)/mouse/fault handlers
  syscall.rs       SYSCALL/SYSRET — naked entry, dispatcher, lock-free output buffer,
                   jump_to_userspace (iretq trampoline); sys_open/read/write/close/exec/pipe
  elf.rs           ELF64 loader — parse headers, map PT_LOAD segments, build user images
  userspace.rs     Two isolated ring-3 processes — spawn_user_process(pid), user_stub
  memory.rs        Page table init, BootInfoFrameAllocator (with next/init_from),
                   mark_all_user_accessible
  vmm.rs           Virtual Memory Manager — global frame alloc, new_process_pml4,
                   map_page_in, map_region, switch_to, switch_to_boot, alloc_zeroed_frame
  allocator.rs     Heap allocator (linked_list_allocator, 32 MiB)
  scheduler.rs     Preemptive scheduler — Task (with pml4 field), Scheduler,
                   SCHEDULER global, timer_schedule, spawn_with_pml4, waitpid/reap
  ata.rs           ATA PIO driver — LBA28 read_sector, BSY/DRQ polling, nIEN disable
  fat32.rs         FAT32 — BPB, FAT chain, LFN/short-name lookup,
                   read/write/create/rename/delete/copy, safe_write_file, stats/check
vfs.rs           VFS — task-local fd tables over shared file/pipe/shmem objects,
                    selective child-fd inheritance, vfs_open/vfs_pipe/vfs_read/vfs_write/vfs_close,
                    vfs_shmem_create/vfs_shmem_map
  klog.rs          Kernel log ring buffer + /LOGS/KERNEL.TXT flushing
  notifications.rs Desktop notification queue used by USB/task/filesystem events
  clipboard.rs     Shared text/path clipboard service
  device_registry.rs Central PCI/USB/system device table
  net.rs           Ethernet/ARP/IPv4/ICMP/UDP/DNS/TCP network stack
  entropy.rs       RDRAND-backed kernel entropy source for TLS handshakes
  tls.rs           TLS 1.3 HTTPS client over kernel TCP sockets
  tls_roots.rs     Built-in DER trust roots for certificate validation
  virtio_net.rs    Legacy PCI virtio-net driver over polling virtqueues
  acpi.rs          Power-control status foundation
  app_metadata.rs  App/package metadata and file associations
  shortcuts.rs     /CONFIG/SHORTCUT.CFG global shortcut loader
  framebuffer.rs   Linear framebuffer driver — 3bpp/4bpp, draw_char, scroll
  vga_buffer.rs    Text layer over framebuffer — used by print!/panic handler
  mouse.rs         PS/2 mouse hardware init and packet decoder
  keyboard.rs      Modifier-aware lock-free input queue — IRQ/USB handlers push key events,
                   compositor drains them into global shortcuts or focused apps
  wm/
    mod.rs         Public WM API — request_repaint, compose_if_needed, userspace GUI bridge
    compositor.rs  WindowManager — shadow buffer, z-order, drag, taskbar,
                   context menu, syscall output drain, AppWindow enum dispatch, blit
    window.rs      Window struct — back-buffer, hit tests
  apps/
    terminal.rs    TerminalApp — keyboard input, shell commands, text render
    sysmon.rs      SysMonApp   — live CPU/heap/uptime/scheduler display
    textviewer.rs  TextViewerApp — scrollable static text
    colorpicker.rs ColorPickerApp — clickable EGA palette swatches
    usergui.rs     UserGuiApp — compositor-owned window/surface/event queue for ring-3 apps
    utilities.rs   UtilityApp — Trash Bin, Screenshot, Notes, and Text Editor
userspace/
  libcool/         no_std userspace SDK — entry, argv, syscalls, files, pipes,
                   mmap, shmem, events, networking, filesystem utilities, GUI, print!/println!
  hello/
    src/main.rs    `/bin/hello` — minimal userspace ELF that writes and exits
    src/bin/exec.rs `/bin/exec` — userspace `sys_exec` demo that replaces itself with `/bin/hello`
    src/bin/pipe.rs `/bin/pipe` — anonymous pipe round-trip demo
    src/bin/piperd.rs `/bin/piperd` — userspace shared-pipe reader demo
    src/bin/pipewr.rs `/bin/pipewr` — userspace shared-pipe writer demo
    src/bin/keyecho.rs `/bin/keyecho` — userspace WM key/click event packet demo
    src/bin/read.rs `/bin/read` — userspace VFS demo that opens and reads `/bin/motd.txt`
    src/bin/netdemo.rs `/bin/netdemo` — userspace DNS/HTTP syscall demo
    src/bin/wget.rs `/bin/wget` — userspace TCP socket HTTP client
    src/bin/sdkdemo.rs `/bin/sdkdemo` — libcool SDK coverage demo
    src/bin/guidemo.rs `/bin/guidemo` — ring-3 GUI window demo
    src/bin/notes.rs `/bin/notes` — ring-3 notes utility
    src/bin/editor.rs `/bin/editor` — ring-3 text editor
    src/bin/trash.rs `/bin/trash` — ring-3 Trash utility
    src/bin/screenshot.rs `/bin/screenshot` — ring-3 screenshot utility
    linker.ld      Fixed-address linker script for the userspace ELF binaries
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
`iretq` drops straight into the entry function. The scheduler also tracks each
task's kernel stack top so syscall entry can follow the currently running task
instead of relying on one shared syscall stack.

**Ring-3 userspace (Phase 9).** The GDT now has four segments (kernel code
0x08, kernel data 0x10, user data 0x18, user code 0x20) plus a TSS whose RSP0
points to a dedicated 64 KiB ISR stack used when an IRQ fires during ring-3
execution. SYSCALL/SYSRET is enabled via EFER.SCE; STAR is set so that
SYSCALL enters kernel CS=0x08/SS=0x10 and SYSRET returns to user
CS=0x20/SS=0x18. The naked `syscall_entry` stub saves user RSP in r10,
switches to the currently scheduled task's private kernel stack top, builds a
register frame, calls the Rust `syscall_dispatch`, and restores with `pop rsp`
and `sysretq`.
`sys_write` output goes through a lock-free ring buffer (same pattern as the
keyboard queue) that the compositor drains into the terminal each frame —
avoiding the deadlock that would result from locking WM from syscall context.

**FAT32 + VFS (Phase 11).** A 64 MiB FAT32 disk image (`fs.img`) is built at
compile time by a host-side `fs-image` tool and attached to QEMU as the IDE
primary-bus slave (`if=ide,index=1`). The ATA PIO driver targets ports
0x1F0–0x1F7; it sets nIEN=1 in the Device Control Register (0x3F6) before
issuing any command so the drive never fires IRQ14. Unused PIC IRQs (including
IRQ14/15) are masked after PIC initialisation to prevent unhandled interrupt
vectors from reaching the CPU. The FAT32 layer parses the BPB, walks the FAT
chain, resolves short-name and long-filename absolute paths, and supports the
shell/file-manager mutation helpers. A thin VFS layer wraps reads into a
16-slot FD table. Syscalls 5–7 (`open`, `read`, `close`) expose the VFS to
ring-3 code, and the kernel's `fs-test` task reads `/bin/hello.txt` on boot.

**Per-process virtual memory (Phase 10).** Each user task owns a PML4 cloned
from the kernel's boot PML4 (upper-half entries 256–511 copied; lower half
empty). `vmm::new_process_pml4` handles the clone; `vmm::map_page_in` / `vmm::map_region`
insert PTEs into any address space by constructing a temporary `OffsetPageTable`
over the target PML4 frame. The scheduler writes the winning task's PML4 into
CR3 on every context switch. User stacks are mapped at `0x7FFF_0010_0000` in the
lower half — L4 index 0xFF, which the kernel never populates — so two processes
at the same VA have completely separate physical frames. A kernel-only guard page
sits below each stack. The `#PF` handler lazily allocates zeroed frames for
not-present user-mode faults in the lower half; protection violations and kernel
faults still panic.

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
| 10 | Per-process virtual memory + isolation | **Done** |
| 11 | Filesystem (FAT32) + VFS + disk driver | **Done** |
| 12 | ELF loader — real programs run from disk | **Done** |
| 13 | Pipes + shared memory + IPC | **Done** |
| 14 | USB HID — real hardware input | **Done** |
| 15 | Networking — virtio-net, TCP/IP | **Done** |
| 16 | UI polish — desktop shell, launcher, taskbar, settings | **Done** |
| 17 | Browser foundation — HTTP/1.1, redirects, chunked responses, local browser UX | **Done** |
| 18 | HTTPS/TLS foundation — TLS 1.3, certificate validation, browser/terminal integration | **Done** |
| 19 | Browser rendering — inline PNG image previews and trust hardening | **Done** |
| 20 | Userspace SDK — `libcool` wrappers and `/bin/sdkdemo` coverage | **Done** |
| 21 | Userspace GUI runtime — window/surface/event syscalls and `/bin/guidemo` | **Done** |
| 22 | Userspace utility suite — Notes, Editor, Trash, Screenshot as ring-3 apps | **Done** |

Full task checklists and technical notes in [ROADMAP.md](ROADMAP.md).
