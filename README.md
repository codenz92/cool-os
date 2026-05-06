<img width="1392" height="864" alt="image" src="https://github.com/user-attachments/assets/5824d8c0-d0d8-4b79-b650-9fec05fa0c70" />


# coolOS

A 64-bit operating system kernel written in Rust. Boots bare-metal into a
graphical desktop with draggable and resizable windows, a taskbar, a start
menu, hardware input, kernel and userspace GUI applications, a preemptive
scheduler, ring-3 userspace, per-process virtual memory, process isolation, a
CoolFS root filesystem with VFS/syscalls and uid/gid/mode enforcement, a FAT32
legacy import mount, an ELF loader with `exec`, and IPC with pipes, shared
memory, and per-task fd tables.

---

# Current state — v6.2

The kernel boots into a graphical desktop at **1280×720, 24bpp** via a
`bootloader 0.11` linear framebuffer (VBE BIOS path). A terminal window opens
on boot. Right-clicking the desktop opens a context menu to launch additional
apps, and the shell also exposes desktop icons plus a start menu/taskbar flow,
global keyboard shortcuts, a `Ctrl+Space` launcher/search palette, a task
switcher overlay, edge/keyboard window snapping, taskbar previews/actions,
session restore, File Manager drag/drop/open-with actions, a shared clipboard,
notification center, userspace app lifecycle tracking with System Monitor
close/kill/path controls, and desktop settings that persist to the CoolFS root.
A preemptive round-robin scheduler is driven by the PIT timer at **100 Hz**;
the kernel boot stack remains the idle/window-manager context, the boot path
performs a synchronous CoolFS read check, and the terminal can also spawn
additional ring-3 ELF tasks from disk with `exec`. The shell also has a real
package/app manifest path:
UTF-8 `.PKG` manifests can install apps into `/APPS/<command>/APP.CFG`,
contribute launcher aliases and file associations, launch a declared userspace
executable through `exec=`, and be removed without leaving stale launcher
entries. Manifest permission labels become launch-time task capabilities, and
the VFS/syscall layer enforces filesystem, network, desktop, and execute access.

| Context | Mode | Description |
| :------ | :--- | :---------- |
| **idle/wm** | ring 0 | Kernel boot stack and idle context; runs `compose_if_needed()` and `hlt`. |
| **boot fs check** | ring 0 | Synchronous `fs_test_once()`: reads `/bin/hello.txt` from CoolFS and prints it during boot. |
| **timer counter** | IRQ0 | PIT timer increments `BACKGROUND_COUNTER`. Visible in System Monitor. |
| **user1** | ring 3 | Own PML4 + private user stack. Writes sentinel `0xDEAD0001`, reads it back, prints `[ring3 pid=1] sentinel ok`. |
| **user2** | ring 3 | Own PML4 + private user stack at the same VA as user1. Writes `0xDEAD0002` — cannot see user1's memory. |

On boot, the contents of `/bin/hello.txt` are printed to the console by the
synchronous filesystem check. Both `[ring3 pid=1] sentinel ok` and `[ring3 pid=2] sentinel ok`
appear in the terminal, proving process isolation: same virtual address,
different physical frames. Typing `exec /bin/hello` launches a real userspace
ELF from disk, `exec /bin/exec` demonstrates `sys_exec` by replacing a running
userspace image with `/bin/hello`, `exec /bin/pipe` exercises per-process
anonymous pipes, `ipc` allocates a fresh shared pipe and launches the
reader/writer demo pair by inheriting one pipe end into each child, `keydemo`
streams focused terminal keystrokes into a userspace process over an inherited
pipe, and `exec /bin/read` exercises userspace `open/read/close` against the
CoolFS-backed VFS. With QEMU virtio networking enabled, `exec /bin/wget
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
| **Desktop shell** | Wallpaper, desktop icons, right-click context menu, start menu, taskbar window buttons, configurable shortcuts, launcher/search palette, notification center, File Manager drag/drop/open-with routing, shared clipboard plumbing, userspace app lifecycle tracking with System Monitor controls, persistent settings, session restore, and clock. |
| **Heap** | `LockedHeap` allocator — `String`, `Vec`, `Box` all work. 32 MiB heap to accommodate large shadow and window buffers. |
| **Paging / VMM** | 4-level `OffsetPageTable` + global `BootInfoFrameAllocator`. Per-process PML4 cloned from kernel upper half; private user-space mappings in lower half. `vmm::` module exposes `new_process_pml4`, `map_page_in`, `map_region`, `switch_to`. |
| **IDT** | Breakpoint, Double Fault, Page Fault (lazy allocator for user faults), General Protection Fault, Invalid Opcode, Timer (IRQ0), Keyboard (IRQ1), Mouse (IRQ12). |
| **Scheduler** | Preemptive round-robin at 100 Hz. Each task carries `pml4: Option<PhysFrame>` plus `uid`, `gid`, and capability credentials; the scheduler calls `vmm::switch_to` on context switch when `Some`. 64 KiB heap-allocated kernel stack per task. Task lifecycle now distinguishes ready/running/blocked/exited/reaped states, records parents and exit codes, supports `waitpid`/reaping, supports kernel-side task termination, and backs blocking pipe reads with `block_current` / `unblock(id)`. |
| **Process isolation** | Two user processes share the same user-stack virtual address (`0x7FFF_0010_0000`) but map it to different physical frames. Guard pages (kernel-only) sit below each stack. |
| **GDT + TSS** | Four segments (kernel code/data ring 0, user code/data ring 3) + TSS with RSP0 pointing to a dedicated 64 KiB ISR stack used when an IRQ fires during ring-3 execution. |
| **SYSCALL/SYSRET** | EFER.SCE enabled. STAR/LSTAR/SFMASK MSRs configured. Naked `syscall_entry` saves context, switches to the currently scheduled task's private kernel stack top, dispatches on rax, restores context, and executes `sysretq`. |
| **Syscall table** | `0 exit`, `1 write`, `2 yield`, `3 getpid`, `4 mmap(addr, len, flags)`, `5 open(path, len)`, `6 read(fd, buf, len)`, `7 close(fd)`, `8 exec(path, len)`, `9 pipe(fds_ptr)`, `10 dup(fd)`, `11 shmem_create(len)`, `12 shmem_map(id)`, `13 waitpid(pid, status_ptr)`, `14 spawn(path, len)`, `15 sleep_ms(ms)`, `16 abi_version()`, `17 dns_resolve(host, len)`, `18 http_get(host, len)`, `19 socket(domain, type, proto)`, `20 connect(socket, ipv4, port)`, `21 send(socket, buf, len)`, `22 recv(socket, buf, len)`, `23 gui_open(title, len, dims)`, `24 gui_present(handle, pixels, len)`, `25 gui_poll_event(handle, packet, len)`, `26 gui_close(handle)`, `27 fs_write_file(desc)`, `28 fs_create_dir(path, len)`, `29 fs_delete_tree(path, len)`, `30 fs_list_dir(desc)`, `31 screenshot(path, len, flags)`. `sys_write` writes to stdout/stderr through the compositor ring buffer or to pipe write-ends through the VFS fd table. |
| **Userspace** | Ring-3 code can run either as the original isolation stubs or as real ELF64 binaries loaded from `/bin`. The `libcool` SDK crate now provides no_std entry/argv setup plus process, file, pipe, mmap, shared-memory, event, DNS/HTTP, TCP socket, filesystem utility, screenshot, and userspace GUI wrappers. `sys_exec` replaces the current userspace image in-place by swapping CR3 and rewriting the saved syscall return frame. `keydemo` relays fixed-size key and click event packets into a userspace child over a pipe. Shared memory (`sys_shmem_create`/`sys_shmem_map`) maps a region of physical frames into the caller's address space at a fixed VA. |
| **ELF loader** | Validates ELF64 headers, maps `PT_LOAD` segments into a fresh address space, allocates a private user stack, builds an initial `argc/argv/envp` stack frame, and can either spawn a new task or prepare an image for `sys_exec`. |
| **ATA PIO driver** | Primary-bus slave device (QEMU `if=ide,index=1`). LBA28 PIO reads, BSY/DRQ polling with timeout, nIEN=1 (device interrupts disabled). Wrapped in `without_interrupts` to prevent preemption mid-transfer. |
| **CoolFS layer** | Native coolOS root filesystem mounted at `/`, stored directly at LBA 0 of the attached OS disk. It has a CoolFS superblock, fixed inode table with durable `uid`/`gid`/mode metadata, block bitmap, 4 KiB blocks, direct plus indirect data blocks, directory records, a 64-slot block cache with dirty 4 KiB writeback, VFS read/write/create/rename/delete/copy routing, stats, and boot self-tests. |
| **FAT32 layer** | Optional legacy import mount at `/FAT`, formatted in a separate 8 MiB-offset disk region. BPB parsing, FAT chain walking, short-name and long-filename lookup, directory traversal, cluster→sector mapping, mutation helpers, free-space stats, and `fsck` remain available without being required for CoolFS boot. |
| **VFS** | CoolFS-root path routing, `/FAT` legacy routing, CoolFS read/write/execute permission enforcement, and task-local fd tables (16 slots, fds 0–2 reserved) backed by shared file/pipe/shmem objects. `vfs_open` reads whole files into heap buffers after access checks; `vfs_pipe` allocates a 512-byte kernel ring buffer and returns per-task read/write fds; `vfs_read_blocking` blocks tasks on empty pipes and wakes them on write/EOF; `ipc` selectively inherits pipe ends into child processes; `vfs_shmem_create`/`vfs_shmem_map` manage a shared memory region pool indexed by ID. |
| **Networking** | Legacy PCI virtio-net driver for QEMU user networking, polling RX/TX virtqueues, Ethernet framing, ARP cache, IPv4, ICMP echo, UDP DNS queries, minimal TCP client sockets, userspace socket syscalls, HTTP/1.1, and verified TLS 1.3 HTTPS for the native browser/terminal path. |
| **Kernel services** | Persistent kernel log buffer flushed to `/LOGS/KERNEL.TXT`, crash-screen log tail, central device registry for PCI/USB/system devices, installable package/app manifests with file associations, networking status, and ACPI power-control status foundation. |
| **Applications** | Terminal, System Monitor, Text Viewer, Color Picker, File Manager, Web Browser, ring-3 Notes, Text Editor, Trash Bin, Screenshot, and GUI Demo. Text-file opens route into `/bin/editor <path>` with kernel viewer fallback, while File Manager exposes explicit Open With Editor/Viewer actions. |
| **Disk image** | `disk-image/src/fs_image.rs` builds `fs.img` as a 64 MiB raw OS disk: native CoolFS starts at LBA 0 with root-owned system paths, user-owned writable paths, executable `/bin` ELFs, `/Packages/guidemo.pkg`, and `/Documents/package-demo.p25`; an optional FAT32 `/FAT` import region starts at 8 MiB. The Makefile attaches it to QEMU as the IDE slave. |

### Applications

| App | How to open | Description |
| :-- | :---------- | :---------- |
| **Terminal** | Launcher / right-click | Interactive shell. Type commands, press Enter. |
| **System Monitor** | Right-click | Live CPU vendor, heap usage, uptime, scheduler counts, USB/input status, and userspace app lifecycle controls for close, kill, and app path. |
| **Text Viewer** | Right-click | Scrollable "About" doc; `j`/`k` to scroll. |
| **Color Picker** | Right-click | Clickable 16-colour EGA palette grid. |
| **File Manager** | Right-click / desktop icon | Browse and mutate the CoolFS root with breadcrumbs, recursive search, sorting, multi-select, clipboard copy/cut/paste, Trash-backed delete, properties, inline text editing, Open With Editor/Viewer, and ELF launch routing. |
| **Web Browser** | Launcher / desktop icon | Native HTTP/HTTPS/local-file browser with address/search bar, redirects, decoded chunked responses, headings/lists/quotes/tables, direct and HTML-sourced inline PNG previews, clickable links, session history, visible TLS trust-root status, and persistent bookmarks. |
| **Trash Bin** | Launcher / desktop icon / `exec /bin/trash` | Ring-3 GUI utility that lists deleted items staged in `/Trash` and can permanently empty them. |
| **Screenshot** | Launcher / desktop icon / `exec /bin/screenshot` | Ring-3 GUI utility that queues a focused-window PPM capture to `/Pictures`. |
| **Notes** | Launcher / desktop icon / `exec /bin/notes [path]` | Ring-3 scratchpad backed by `/documents/notes.txt` by default, with New, Open, Save, and Save As document flow. |
| **Text Editor** | Launcher / desktop icon / `exec /bin/editor [path]` | Ring-3 text editor backed by `/documents/editor.txt` by default, or any absolute file path passed as argv, with New, Open, Save, Save As, and cursor controls. |
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
| `write <path> <text>` | Create or overwrite a text file |
| `rm <path>` | Remove a file or empty folder |
| `hash <path>` | Print file length and byte-sum for storage checks |
| `whoami` | Print current user and task capabilities |
| `perm <path>` | Print owner, group, mode, type, and size |
| `chmod <mode> <path>` | Change a CoolFS inode mode |
| `chown <uid>[:gid] <path>` | Change a CoolFS inode owner/group |
| `exec <path> [args...]` | Load a userspace ELF from disk and spawn it with argv |
| `ps` | List scheduler tasks, uid, ring, status, and exit codes |
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
| `fsck` | Print CoolFS-root consistency plus optional legacy FAT32 import summary |
| `coolfs` | Print CoolFS root mount and inode/block usage |
| `df` | Print CoolFS `/` and optional FAT32 `/FAT` used/free/total space |
| `pkg [list\|install <path-or-id>\|remove <id>\|run <id> [args...]]` | Manage built-in and manifest-installed packages. |
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
builds `fs.img` with the userspace binaries embedded into the native CoolFS `/bin`.

Click inside the QEMU window to capture mouse input. Press **Ctrl+Alt+G** to
release it.

---

## Architecture

```
disk-image/
  src/main.rs      Host tool — wraps kernel ELF into bios.img via bootloader 0.11
  src/fs_image.rs  Host tool — builds fs.img (64 MiB raw OS disk) with native
                    CoolFS at LBA 0 plus an optional /FAT import region:
                    /bin, /CONFIG, /APPS, /Documents, /Packages, /Pictures,
                    /Desktop, /Downloads, /Trash, and /LOGS
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
  coolfs.rs        CoolFS — native root filesystem, raw ATA block backend,
                   inodes, bitmap, direct/indirect blocks, directories, safe writes, stats/check
  fat32.rs         FAT32 — optional legacy /FAT import mount at the 8 MiB region
  vfs.rs           VFS — CoolFS root routing, /FAT compatibility routing,
                    task-local fd tables over shared file/pipe/shmem objects,
                    selective child-fd inheritance, vfs_open/vfs_pipe/vfs_read/vfs_write/vfs_close,
                    vfs_shmem_create/vfs_shmem_map
  klog.rs          Kernel log ring buffer + /LOGS/KERNEL.TXT flushing
  notifications.rs Desktop notification queue used by USB/task/filesystem events
  app_lifecycle.rs Persistent app recents/settings plus runtime userspace app ownership
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
    sysmon.rs      SysMonApp   — live CPU/heap/uptime/scheduler/app lifecycle controls
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

**Native CoolFS root + VFS (Phase 27).** A 64 MiB raw OS disk (`fs.img`) is built
at compile time by a host-side `fs-image` tool and attached to QEMU as the IDE
primary-bus slave (`if=ide,index=1`). CoolFS starts directly at LBA 0 and owns
the `/` namespace; the legacy FAT32 import region starts at 8 MiB and remains
available at `/FAT` without being part of the boot path. The ATA PIO driver targets ports
0x1F0–0x1F7; it sets nIEN=1 in the Device Control Register (0x3F6) before
issuing any command so the drive never fires IRQ14. Unused PIC IRQs (including
IRQ14/15) are masked after PIC initialisation to prevent unhandled interrupt
vectors from reaching the CPU. CoolFS uses fixed inodes, a block bitmap, 4 KiB
blocks, direct blocks, and a single indirect block for larger userspace
binaries. A 64-slot block cache writes dirty 4 KiB blocks back to the native
disk backend. The VFS routes normal absolute paths to CoolFS, routes `/FAT/*` to
the optional import mount, and wraps reads into a 16-slot FD table. Syscalls 5–7 (`open`,
`read`, `close`) expose the VFS to ring-3 code, and the synchronous boot
filesystem check reads `/bin/hello.txt` from CoolFS.

**Users, permissions, and app sandboxing (Phase 28).** CoolFS stores durable
`uid`, `gid`, and `rwx` mode bits in each inode. The generated disk image marks
system paths such as `/bin`, `/CONFIG`, `/APPS`, `/LOGS`, and `/DEV` as
root-owned, marks user-writable locations such as `/TMP`, `/Documents`,
`/Pictures`, `/Desktop`, `/Trash`, `/Downloads`, and `/Packages` as uid/gid
1000, and sets execute bits on `/bin` ELF files. Every scheduler task carries
credentials and capabilities; `exec` requires execute permission, VFS file and
directory operations check mode bits, and userspace network/desktop syscalls are
gated by task capabilities. Package manifest `permission=` labels are converted
to launch-time grants, so package-launched apps run with bounded access instead
of inheriting unrestricted shell authority. Terminal commands `whoami`, `perm`,
`chmod`, and `chown` expose the model for smoke tests and manual inspection.

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
| 23 | App lifecycle + file-open plumbing — process/window ownership, close cleanup, editor argv routing | **Done** |
| 24 | App platform polish — editor document flow, File Manager Open With, System Monitor lifecycle controls, deterministic utility smokes | **Done** |
| 25 | Package platform — installable app manifests, dynamic launcher entries, package associations, package launch/remove smoke | **Done** |
| 26 | CoolFS root filesystem — `/` backed by CoolFS with `/FAT` compatibility | **Done** |
| 27 | Native CoolFS disk backend — CoolFS at LBA 0, optional `/FAT` import region, remount persistence smoke | **Done** |
| 28 | Users, permissions, and app sandboxing — CoolFS uid/gid/mode, task credentials, package grants, syscall enforcement | **Done** |

Full task checklists and technical notes in [ROADMAP.md](ROADMAP.md).
