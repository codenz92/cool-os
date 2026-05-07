<img width="1392" height="864" alt="image" src="https://github.com/user-attachments/assets/5824d8c0-d0d8-4b79-b650-9fec05fa0c70" />


# coolOS

A 64-bit operating system kernel written in Rust. Boots bare-metal into a
graphical desktop with draggable and resizable windows, a taskbar, a start
menu, hardware input, kernel and userspace GUI applications, a preemptive
scheduler, ring-3 userspace, per-process virtual memory, process isolation, a
CoolFS root filesystem with VFS/syscalls and uid/gid/mode enforcement, a FAT32
legacy import mount, an ELF loader with `exec`, task-local cwd and fd-mapped
stdio, and IPC with pipes, shared memory, and per-task fd tables.

---

# Current state — v7.20

The kernel boots into a graphical desktop at **1280×720, 24bpp** via a
`bootloader 0.11` linear framebuffer (VBE BIOS path). A terminal window opens
on boot. Right-clicking the desktop opens a context menu to launch additional
apps, and the shell also exposes desktop icons plus a start menu/taskbar flow,
global keyboard shortcuts, a `Ctrl+Space` launcher/search palette, a task
switcher overlay, edge/keyboard window snapping, taskbar previews/actions,
boot-splash-style GUI login/lock screen with first-run setup, session restore,
Accounts settings, File Manager
drag/drop/open-with actions, a shared clipboard, notification center, userspace
app lifecycle tracking with System Monitor close/kill/path controls, and
desktop settings that persist to the CoolFS root.
A preemptive round-robin scheduler is driven by the PIT timer at **288 Hz**;
the kernel boot stack remains the idle/window-manager context, the boot path
performs a synchronous CoolFS read check, and the terminal can also spawn
additional ring-3 ELF tasks from disk with `exec`. The shell also has a real
package/app manifest path:
UTF-8 `.PKG` manifests can install apps into `/APPS/<command>/APP.CFG`,
contribute launcher aliases and file associations, launch a declared userspace
executable through `exec=`, and be removed without leaving stale launcher
entries. Manifest permission labels become launch-time task capabilities, and
the VFS/syscall layer enforces filesystem, network, desktop, and execute access.
The desktop now starts behind a compositor-owned greeter; the active session is
backed by a persistent CoolFS user database, so
Terminal commands, launched ELF tasks, and package apps inherit the logged-in
user's uid/gid and non-admin users cannot mutate protected ownership or service
state without switching back to an admin session. Phase 31 adds a first-run
admin handoff, account create/disable/role/password/delete flows, login
throttling, and persistence smoke coverage for those account records. Phase 32
keeps copied kernel mappings supervisor-only for ring-3 tasks, removes broad
lazy lower-half page allocation, and turns denied user pointers into task
faults instead of kernel crashes. Phase 33 exposes process control through ABI
v6: userspace can signal tasks and process groups, STOP/CONT changes scheduler
eligibility, and Terminal jobs can bind to real processes. Phase 34 adds
per-terminal TTY ownership, routes userspace stdout/stderr back to the launching
terminal, and gives the shell foreground process groups with Ctrl+C/Ctrl+Z,
`fg`, `bg`, and prompt blocking until foreground work stops or exits. Phases
35-39 add canonical TTY stdin for `read(0)`, a real `/bin/sh` userspace shell,
ABI v7 `spawn_args`, coreutils-style `/bin` tools, verified utility-app save
paths, and a `/RECOVERY` repair/report surface. Phases 40-44 move the shell
and platform closer to a normal desktop OS: ABI v8 adds task cwd, metadata,
rename, writable file descriptors, fd-mapped child stdio, sync, and RTC time;
`/bin/sh` now supports quoting, relative paths, redirection, and one-stage
pipelines; `/bin` includes practical file/text/date/devkit tools; sysreport can
write `/LOGS/SYSREPORT.TXT`; and the generated image ships `/SDK` docs and
templates. Phases 45-56 add compositor smoothness, evented terminal work, and
a richer native browser renderer:
timer ticks now request
paced frames instead of unconditional full redraws, mouse-only motion uses a
hardware cursor overlay fast path, active input temporarily boosts full-frame
pacing to 144 Hz while idle work stays at 36 Hz, the main loop polls input
before background maintenance, and `compositor`/`smoothness` telemetry exposes
frame source, adaptive pacing, budget, damage, and cursor overlay counters. ABI
v9 adds `poll(desc, count, timeout_ms)` so ring-3 programs can block on pipes,
TTY stdin, TCP sockets, GUI events, and child exit without spin/yield loops; ABI
v10 adds `tty_control(op, arg1, arg2)` so foreground ring-3 programs can query
terminal geometry and switch between canonical and raw TTY input. The Browser
now has a bounded CSS cascade/layout pass for tag/class/id/inline selectors,
CSS-derived alignment, colors, backgrounds, indentation, hidden content, image
sizing hints, PNG/JPEG/GIF/WebP metadata handling, GET-form query construction,
DOM-backed form controls with live value state, keyboard focus/editing, reset
handling, real URL-encoded POST request bodies over the shared HTTP/TLS stack,
persistent Browser cookie/session state under `/CONFIG/BROWSER.COOKIES`, a
redacted `browser://session` inspection page, CSS2 box-model layout with
bounded margins, padding, borders, backgrounds, percentage widths, and smoke
fixtures for the browser engine, CSS layout, forms, DOM events, DOM-backed form
interaction, POST form pages, session state, and box-model pages.

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
render headings, lists, block quotes, simple tables, CSS-styled text blocks,
bounded inline PNG images, image dimensions/placeholders for common image
formats, and HTML forms that submit GET query URLs or POST bodies, and keep
session history, persistent local bookmarks, and persistent browser cookies.
HTTPS uses a
no_std TLS 1.3 client over the kernel TCP stack with hardware RNG entropy,
RTC-backed certificate validity checks, X.509 chain validation against the
built-in trust roots, and SAN-first hostname validation coverage.

### What's working

| Subsystem | Details |
| :-------- | :------ |
| **Framebuffer** | `bootloader 0.11` linear framebuffer at ≥1280×720. 3bpp and 4bpp both handled. Shadow-buffer compositor with dirty row-span blits, adaptive 36/144 Hz frame pacing, and a hardware cursor overlay fast path that restores/draws only cursor rectangles for mouse-only motion. No tearing. |
| **PS/2 mouse** | Full hardware init (CCB, 0xF6/0xF4), 9-bit signed X/Y deltas, IRQ12 packet collection via atomics. |
| **Window manager** | Z-ordered windows, focus-on-click, title-bar drag, edge snapping, keyboard snapping, task switcher overlay, minimise/maximise/restore, resize grip, close button, taskbar previews/right-click actions, per-window pixel back-buffer. |
| **Desktop shell** | Wallpaper, desktop icons, right-click context menu, start menu, taskbar window buttons, configurable shortcuts, launcher/search palette, login/lock greeter, Accounts settings, notification center, File Manager drag/drop/open-with routing, shared clipboard plumbing, userspace app lifecycle tracking with System Monitor controls, persistent settings, session restore, clock, and adaptive compositor smoothness telemetry. |
| **Heap** | `LockedHeap` allocator — `String`, `Vec`, `Box` all work. 32 MiB heap to accommodate large shadow and window buffers. |
| **Paging / VMM** | 4-level `OffsetPageTable` + global `BootInfoFrameAllocator`. Per-process PML4 cloned from kernel upper half; private user-space mappings in lower half. `vmm::` module exposes `new_process_pml4`, `map_page_in`, `map_region`, `switch_to`. |
| **IDT** | Breakpoint, Double Fault, Page Fault with user-task termination/crashdump handling for invalid ring-3 accesses, General Protection Fault, Invalid Opcode, Timer (IRQ0), Keyboard (IRQ1), Mouse (IRQ12). |
| **Scheduler** | Preemptive round-robin at 288 Hz. Each task carries `pml4: Option<PhysFrame>` plus `uid`, `gid`, capability credentials, current working directory, process group, controlling TTY, pending signal state, and a private 64 KiB kernel stack. The scheduler switches CR3 when needed and updates TSS RSP0 to the selected task's stack so ring-3 IRQ frames never share one global stack. Task lifecycle now distinguishes ready/running/blocked/stopped/exited/reaped states, records parents and exit codes, supports blocking `waitpid`/reaping, supports kernel-side task termination, and backs blocking pipe/TTY reads with `block_current` / `unblock(id)`. |
| **TTY sessions** | Each Terminal owns a kernel TTY with canonical/raw input modes, echo and signal flags, cell geometry, a dedicated output queue, and a foreground process group. Userspace `read(0)` plus stdout/stderr route through the task's controlling TTY by default, fd mappings can override 0/1/2 for pipes and redirection, `exec` blocks the prompt as a foreground job, background jobs keep their terminal output route, and Ctrl+C/Ctrl+Z signal the foreground group when signal mode is enabled. |
| **Process isolation** | Two user processes share the same user-stack virtual address (`0x7FFF_0010_0000`) but map it to different physical frames. Guard pages (kernel-only) sit below each stack. |
| **GDT + TSS** | Four segments (kernel code/data ring 0, user code/data ring 3) + TSS. RSP0 starts on a fallback ISR stack and is updated on every context switch to the selected task's private kernel stack for ring-3 IRQ entry. |
| **SYSCALL/SYSRET** | EFER.SCE enabled. STAR/LSTAR/SFMASK MSRs configured. Naked `syscall_entry` saves context, switches to the currently scheduled task's private kernel stack top, dispatches on rax, restores context, and executes `sysretq`. |
| **Syscall table** | `0 exit`, `1 write`, `2 yield`, `3 getpid`, `4 mmap(addr, len, flags)`, `5 open(path, len)`, `6 read(fd, buf, len)`, `7 close(fd)`, `8 exec(path, len)`, `9 pipe(fds_ptr)`, `10 dup(fd)`, `11 shmem_create(len)`, `12 shmem_map(id)`, `13 waitpid(pid, status_ptr)`, `14 spawn(path, len)`, `15 sleep_ms(ms)`, `16 abi_version()`, `17 dns_resolve(host, len)`, `18 http_get(host, len)`, `19 socket(domain, type, proto)`, `20 connect(socket, ipv4, port)`, `21 send(socket, buf, len)`, `22 recv(socket, buf, len)`, `23 gui_open(title, len, dims)`, `24 gui_present(handle, pixels, len)`, `25 gui_poll_event(handle, packet, len)`, `26 gui_close(handle)`, `27 fs_write_file(desc)`, `28 fs_create_dir(path, len)`, `29 fs_delete_tree(path, len)`, `30 fs_list_dir(desc)`, `31 screenshot(path, len, flags)`, `32 signal(pid, signal)`, `33 setpgid(pid, pgid)`, `34 getpgid(pid)`, `35 signal_group(pgid, signal)`, `36 spawn_args(desc)`, `37 chdir(path, len)`, `38 getcwd(buf, len)`, `39 stat(desc)`, `40 rename(desc)`, `41 open_write(path, len)`, `42 spawn_fds_args(desc)`, `43 sync()`, `44 time()`, `45 poll(desc, count, timeout_ms)`, and `46 tty_control(op, arg1, arg2)`. `sys_read(0)` reads from the current task's controlling TTY when assigned unless fd 0 is mapped; `sys_write` writes stdout/stderr to that TTY, falls back to the compositor ring for orphaned output, or writes pipe/file descriptors through the VFS fd table. |
| **Userspace** | Ring-3 code can run either as the original isolation stubs or as real ELF64 binaries loaded from `/bin`. The `libcool` SDK crate now provides no_std entry/argv setup plus process, signal/process-group, file, pipe, evented poll, TTY mode/size, mmap, shared-memory, event, DNS/HTTP, TCP socket, filesystem utility, screenshot, time, and userspace GUI wrappers. `/bin/sh` reads stdin from the TTY, tracks the kernel cwd, parses quoting and escapes, runs builtins, resolves bare commands under `/bin`, supports `<`/`>` redirection and one-stage `|` pipelines, and can launch argv/fd-capable children with `spawn_args` or `spawn_fds_args`. `/bin/ls`, `/bin/cat`, `/bin/echo`, `/bin/pwd`, `/bin/mkdir`, `/bin/touch`, `/bin/rm`, `/bin/writefile`, `/bin/cp`, `/bin/mv`, `/bin/grep`, `/bin/head`, `/bin/tail`, `/bin/date`, `/bin/uname`, `/bin/clear`, `/bin/stat`, `/bin/sync`, `/bin/devkit`, `/bin/polldemo`, and `/bin/tuidemo` cover practical command-line, evented, and terminal-mode workflows. `sys_exec` replaces the current userspace image in-place by swapping CR3 and rewriting the saved syscall return frame. Shared memory (`sys_shmem_create`/`sys_shmem_map`) maps a region of physical frames into the caller's address space at a fixed VA. |
| **ELF loader** | Validates ELF64 headers, maps `PT_LOAD` segments into a fresh address space, allocates a private user stack, builds an initial `argc/argv/envp` stack frame, and can either spawn a new task or prepare an image for `sys_exec`. |
| **ATA PIO driver** | Primary-bus slave device (QEMU `if=ide,index=1`). LBA28 PIO reads/writes, BSY/DRQ polling with bounded retries and software reset recovery, nIEN=1 (device interrupts disabled). Wrapped in `without_interrupts` to prevent preemption mid-transfer. |
| **CoolFS layer** | Native coolOS root filesystem mounted at `/`, stored directly at LBA 0 of the attached OS disk. It has a CoolFS superblock, fixed inode table with durable `uid`/`gid`/mode metadata, block bitmap, 4 KiB blocks, direct plus indirect data blocks, directory records, a 64-slot block cache with dirty 4 KiB writeback, VFS read/write/create/rename/delete/copy routing, stats, and boot self-tests. |
| **FAT32 layer** | Optional legacy import mount at `/FAT`, formatted in a separate 8 MiB-offset disk region. BPB parsing, FAT chain walking, short-name and long-filename lookup, directory traversal, cluster→sector mapping, mutation helpers, free-space stats, and `fsck` remain available without being required for CoolFS boot. |
| **VFS** | CoolFS-root path routing, `/FAT` legacy routing, CoolFS read/write/execute permission enforcement, task-local cwd resolution, and task-local fd tables (16 slots, with explicit 0/1/2 mappings for child stdio) backed by shared file/pipe/shmem objects. `vfs_open` reads whole files into heap buffers after access checks; `vfs_open_write` buffers writable file descriptors and commits through safe CoolFS writes on close/exit; `vfs_pipe` allocates a 512-byte kernel ring buffer and returns per-task read/write fds; `vfs_read_blocking` blocks tasks on empty pipes and wakes them on write/EOF; `ipc` and `spawn_fds_args` selectively inherit pipe/file fds into child processes; `vfs_shmem_create`/`vfs_shmem_map` manage a shared memory region pool indexed by ID. |
| **Networking** | Legacy PCI virtio-net driver for QEMU user networking, polling RX/TX virtqueues, Ethernet framing, ARP cache, IPv4, ICMP echo, UDP DNS queries, minimal TCP client sockets, userspace socket syscalls, HTTP/1.1, and verified TLS 1.3 HTTPS for the native browser/terminal path. |
| **Kernel services** | Persistent kernel log buffer flushed to `/LOGS/KERNEL.TXT`, crash-screen log tail, sysreport generation to `/LOGS/SYSREPORT.TXT`, central device registry for PCI/USB/system devices, installable package/app manifests with file associations, networking status, ACPI power-control status foundation, and a credentialed service supervisor that restarts failed services under service uid/gid 200. |
| **Applications** | Terminal, System Monitor, Text Viewer, Color Picker, File Manager, Web Browser, ring-3 Notes, Text Editor, Trash Bin, Screenshot, Process Demo, and GUI Demo. Text-file opens route into `/bin/editor <path>` with kernel viewer fallback, while File Manager exposes explicit Open With Editor/Viewer actions. |
| **Disk image** | `disk-image/src/fs_image.rs` builds `fs.img` as a 64 MiB raw OS disk: native CoolFS starts at LBA 0 with root-owned system paths, user-owned writable paths, executable `/bin` ELFs, `/RECOVERY` boot/repair docs, `/SDK` devkit docs/templates, `/Packages/guidemo.pkg`, and `/Documents/package-demo.p25`; an optional FAT32 `/FAT` import region starts at 8 MiB. The Makefile attaches it to QEMU as the IDE slave. |

### Applications

| App | How to open | Description |
| :-- | :---------- | :---------- |
| **Terminal** | Launcher / right-click | Interactive shell. Type commands, press Enter. |
| **System Monitor** | Right-click | Live CPU vendor, heap usage, uptime, scheduler counts, USB/input status, and userspace app lifecycle controls for close, kill, and app path. |
| **Text Viewer** | Right-click | Scrollable "About" doc; `j`/`k` to scroll. |
| **Color Picker** | Right-click | Clickable 16-colour EGA palette grid. |
| **File Manager** | Right-click / desktop icon | Browse and mutate the CoolFS root with breadcrumbs, recursive search, sorting, multi-select, clipboard copy/cut/paste, Trash-backed delete, properties, inline text editing, Open With Editor/Viewer, and ELF launch routing. |
| **Web Browser** | Launcher / desktop icon | Native HTTP/HTTPS/local-file browser with address/search bar, redirects, decoded chunked responses, headings/lists/quotes/tables, CSS2-style cascade and box-model hints for tag/class/id/inline selectors, styled text blocks, direct and HTML-sourced inline PNG previews, image metadata/placeholders for JPEG/GIF/WebP, clickable links/forms, session history, visible TLS trust-root status, persistent bookmarks, persistent cookies, and `browser://session`. |
| **Accounts** | Launcher / Display Settings Users tab | Admin account management for first-run setup, account creation, role changes, enable/disable, password reset, and deletion. |
| **Trash Bin** | Launcher / desktop icon / `exec /bin/trash` | Ring-3 GUI utility that lists deleted items staged in `/Trash` and can permanently empty them. |
| **Screenshot** | Launcher / desktop icon / `exec /bin/screenshot` | Ring-3 GUI utility that queues a focused-window PPM capture to `/Pictures`. |
| **Process Demo** | Launcher / `exec /bin/procdemo` | Ring-3 process-control proof for spawn, process groups, USR1, STOP/CONT, group TERM, and `waitpid`. |
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
| `setup <user> <pass>` | Complete first-run admin setup and replace the default `root` handoff if needed |
| `account <op>` | Admin account management: `list`, `add`, `enable`, `disable`, `role`, `pass`, and `delete` |
| `perm <path>` | Print owner, group, mode, type, and size |
| `chmod <mode> <path>` | Change a CoolFS inode mode |
| `chown <uid>[:gid] <path>` | Change a CoolFS inode owner/group |
| `exec <path> [args...]` | Run a userspace ELF as the terminal's foreground job |
| `sh` | Start `/bin/sh` as a foreground userspace shell on the terminal TTY |
| `ps` | List scheduler tasks, uid, ring, status, and exit codes |
| `kill <pid>` | Terminate a non-idle task and mark it exited |
| `wait <pid>` | Reap an exited task and print its exit code |
| `reap` | Reap all exited tasks visible to the shell |
| `proc` | Print pid/ppid/pgid, credentials, pending signal, wake tick, and task state |
| `signal <pid\|-pgid> <term\|int\|usr1\|stop\|cont>` | Deliver a signal to one task or every controllable task in a process group |
| `pgroup <pid> [group]` | View or change a task's process group |
| `tty` | Print the current terminal session, foreground process group, and TTY buffers |
| `jobs` | List background jobs, including process-bound jobs with pid/state |
| `job run <path> [args...]` | Launch an ELF as a process-bound background job |
| `job <cancel\|pause\|resume> <id\|last>` | Control a background job; process jobs map to TERM/STOP/CONT |
| `fg [id\|last]` | Resume a process-bound job in the foreground |
| `bg [id\|last]` | Resume a process-bound job in the background |
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
| `browser [url]` | Open the native Web Browser to a URL or `browser://home` |
| `power [reboot\|shutdown\|sleep]` | Print or request power-control actions |
| `log` | Flush and print the kernel log tail |
| `logs` | Open the in-terminal log view |
| `profiler` | Print boot/session profiler events |
| `diagnostics` | Print kernel, profiler, service, compositor, heap, filesystem, VFS, and crash diagnostics |
| `sysreport [write]` | Print the generated system report or write it to `/LOGS/SYSREPORT.TXT` |
| `devkit` | Print SDK paths, ABI version, and userspace template locations |
| `compositor` | Print FPS, frame pacing, frame budget, damage, and cursor overlay telemetry |
| `smoothness` | Alias for compositor pacing/latency telemetry |
| `fsck` | Print CoolFS-root consistency plus optional legacy FAT32 import summary |
| `recovery [repair\|fsck-on-boot on\|fsck-on-boot off]` | Show recovery status, write a repair report, or toggle boot fsck |
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

For the smoothest QEMU pointer path, use the phase 46 tablet-input and
adaptive-refresh profile:

```bash
make run-smooth
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
                    /Desktop, /Downloads, /Trash, /LOGS, /SDK, and process-control demos
src/
  main.rs          Kernel entry point — framebuffer init, GDT, heap, scheduler,
                   input-first idle loop
  gdt.rs           GDT (ring-0/ring-3 segments) + TSS (RSP0 for ring-3 IRQ entry)
  interrupts.rs    IDT, PIC, PIT (288 Hz passive frame ticks), IRQ masking,
                   keyboard/timer(naked)/mouse/fault handlers
  syscall.rs       SYSCALL/SYSRET — naked entry, dispatcher, lock-free output buffer,
                   jump_to_userspace (iretq trampoline); syscalls including
                   open/read/write/close/exec/pipe/signal/process groups,
                   cwd/stat/rename/open_write/spawn_fds_args/sync/time
  elf.rs           ELF64 loader — parse headers, map PT_LOAD segments, build user images
  userspace.rs     Two isolated ring-3 processes — spawn_user_process(pid), user_stub
  memory.rs        Page table init, BootInfoFrameAllocator (with next/init_from),
                   mark_all_user_accessible
  vmm.rs           Virtual Memory Manager — global frame alloc, new_process_pml4,
                   map_page_in, map_region, switch_to, switch_to_boot, alloc_zeroed_frame
  allocator.rs     Heap allocator (linked_list_allocator, 32 MiB)
  scheduler.rs     Preemptive scheduler — Task (with pml4 field), Scheduler,
                   SCHEDULER global, timer_schedule, spawn_with_pml4, cwd,
                   waitpid/reap, STOP/CONT, process groups, controlling TTYs,
                   and signal delivery
  tty.rs           Kernel TTY registry — per-terminal output buffers and foreground pgid
  jobs.rs          Background job registry, including process-bound jobs controlled by signals
  ata.rs           ATA PIO driver — LBA28 read/write sector, BSY/DRQ polling,
                   bounded reset retry, nIEN disable
  coolfs.rs        CoolFS — native root filesystem, raw ATA block backend,
                   inodes, bitmap, direct/indirect blocks, directories, safe writes, stats/check
  fat32.rs         FAT32 — optional legacy /FAT import mount at the 8 MiB region
  vfs.rs           VFS — CoolFS root routing, /FAT compatibility routing,
                    task-local fd tables over shared file/pipe/shmem objects,
                    task-local cwd resolution, fd-mapped child stdio,
                    selective child-fd inheritance, vfs_open/vfs_open_write/vfs_pipe/vfs_read/vfs_write/vfs_close,
                    vfs_shmem_create/vfs_shmem_map
  klog.rs          Kernel log ring buffer + /LOGS/KERNEL.TXT flushing
  sysreport.rs     System report generator for diagnostics and /LOGS/SYSREPORT.TXT
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
  security.rs      CoolFS-backed local users, password hashes, login throttling,
                   first-run setup, account mutation, and session credentials
  shortcuts.rs     /CONFIG/SHORTCUT.CFG global shortcut loader
  framebuffer.rs   Linear framebuffer driver — 3bpp/4bpp, draw_char, scroll
  vga_buffer.rs    Text layer over framebuffer — used by print!/panic handler
  mouse.rs         PS/2 mouse hardware init and packet decoder
  keyboard.rs      Modifier-aware lock-free input queue — IRQ/USB handlers push key events,
                   compositor drains them into global shortcuts or focused apps
  wm/
    mod.rs         Public WM API — adaptive frame pacing, repaint requests,
                   compose_if_needed, userspace GUI bridge
    compositor.rs  WindowManager — shadow buffer, z-order, drag, taskbar,
                   context menu, syscall output drain, AppWindow enum dispatch,
                   dirty-span blit, hardware cursor overlay, frame-source and
                   frame-budget telemetry
    window.rs      Window struct — back-buffer, hit tests
  apps/
    terminal.rs    TerminalApp — keyboard input, shell commands, text render
    sysmon.rs      SysMonApp   — live CPU/heap/uptime/scheduler/app lifecycle controls
    displaysettings.rs DisplaySettingsApp — display/personalization/security settings,
                   including the Users/Accounts panel
    textviewer.rs  TextViewerApp — scrollable static text
    colorpicker.rs ColorPickerApp — clickable EGA palette swatches
    usergui.rs     UserGuiApp — compositor-owned window/surface/event queue for ring-3 apps
    utilities.rs   UtilityApp — Trash Bin, Screenshot, Notes, and Text Editor
userspace/
  libcool/         no_std userspace SDK — entry, argv, syscalls, files, pipes,
                   signals, process groups, evented poll, TTY control, mmap, shmem, events, networking,
                   filesystem utilities, time, GUI, print!/println!
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
    src/bin/procdemo.rs `/bin/procdemo` — process group and signal-control demo
    src/bin/procsleep.rs `/bin/procsleep` — long-running helper used by jobs/tests
    src/bin/polldemo.rs `/bin/polldemo` — poll readiness demo
    src/bin/tuidemo.rs `/bin/tuidemo` — raw TTY and ANSI terminal demo
    src/bin/sh.rs `/bin/sh` — userspace shell with cwd, redirection, and one-stage pipes
    src/bin/cp.rs `/bin/cp` — streaming userspace file copy
    src/bin/mv.rs `/bin/mv` — userspace rename wrapper
    src/bin/grep.rs `/bin/grep` — line matcher for files or stdin
    src/bin/head.rs `/bin/head` — first-lines reader for files or stdin
    src/bin/tail.rs `/bin/tail` — last-lines reader for files
    src/bin/date.rs `/bin/date` — RTC timestamp syscall wrapper
    src/bin/uname.rs `/bin/uname` — ABI/platform identity
    src/bin/stat.rs `/bin/stat` — metadata inspection
    src/bin/sync.rs `/bin/sync` — writeback barrier wrapper
    src/bin/devkit.rs `/bin/devkit` — SDK path and template helper
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
starts on a fallback ISR stack and is updated to the scheduled task's private
kernel stack for ring-3 IRQ entry. SYSCALL/SYSRET is enabled via EFER.SCE; STAR is set so that
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
root-owned, keeps `/TMP` shared-writable, marks user-facing locations such as
`/Documents`, `/Pictures`, `/Desktop`, `/Trash`, `/Downloads`, and `/Packages`
as uid/gid 1000, and sets execute bits on `/bin` ELF files. Every scheduler task carries
credentials and capabilities; `exec` requires execute permission, VFS file and
directory operations check mode bits, and userspace network/desktop syscalls are
gated by task capabilities. Package manifest `permission=` labels are converted
to launch-time grants, so package-launched apps run with bounded access instead
of inheriting unrestricted shell authority. Terminal commands `whoami`, `perm`,
`chmod`, and `chown` expose the model for smoke tests and manual inspection.

**Login, sessions, and service supervision (Phase 29).** `/CONFIG/USERS.DB`
stores persistent users with hashed passwords, roles, homes, and login state.
The default admin session is `root` uid/gid 1000 with password `cool`, and
`guest` uid 1001 uses password `guest` with non-admin caps. `/Users/root` and
`/Users/guest` are created with owner-only access, new files honor the session
umask, `login`/`logout`/`passwd`/`id`/`groups`/`umask` expose the model in the
Terminal, package installs and service mutations require admin credentials, and
the service supervisor reports per-service credentials and deterministic restart
state through `services`.

**GUI login and lock screen (Phase 30).** The desktop now boots into a
boot-splash-style compositor greeter instead of exposing the session
immediately. The greeter authenticates through the same `/CONFIG/USERS.DB`
model as Terminal `login`, supports keyboard and mouse account selection, masks
passwords, blocks ordinary desktop input while locked, and lets the Start menu
or Terminal `lock` and `logout` commands return to the same splash login screen.

**First-run setup and account management (Phase 31).** The default `root/cool`
handoff is now treated as a first-run state. `setup <user> <pass>` can create
or convert the first real admin account, disable the default handoff, and write
the result back to `/CONFIG/USERS.DB`. Admins can manage accounts through the
Terminal `account` command or the Accounts settings panel, with checks that keep
one enabled admin account available, protect built-in records, enforce stronger
new passwords, and throttle repeated failed login attempts.

**User/kernel isolation hardening (Phase 32).** Process address spaces still
copy the kernel's upper-half PML4 entries so syscall and interrupt entry can run
without rebuilding mappings, but those copied entries stay supervisor-only for
ring-3 tasks. User-pointer copying now checks canonical ranges and mapped user
permissions before kernel code dereferences buffers, denied filesystem,
desktop, and execute syscalls fail cleanly, and invalid ring-3 memory faults
terminate the offending task with crashdump metadata instead of panicking the
kernel.

**Process control and jobs (Phase 33).** ABI v6 adds `signal`, `setpgid`,
`getpgid`, and `signal_group`, with `libcool` wrappers for userspace programs.
The scheduler now treats `STOP` as a real non-runnable state and `CONT` as a
resume signal, applies process-control permission checks, and routes `TERM`/`INT`
through normal task cleanup so `waitpid` observes signal-style exit codes.
Terminal `signal`, `pgroup`, `job run`, `job pause`, `job resume`, and
`job cancel` expose the same machinery interactively, while `/bin/procdemo`
verifies spawn, process groups, USR1, STOP/CONT, group TERM, and wait/reap from
ring 3.

**TTY sessions and foreground jobs (Phase 34).** Every Terminal window owns a
kernel TTY with a private output queue and foreground process group. `exec`
assigns the child a process group and controlling TTY before unblocking it,
routes stdout/stderr back to the launching Terminal, and holds the prompt until
that foreground group exits or stops. Background `job run` processes keep their
TTY binding for output, while `tty`, `fg`, `bg`, Ctrl+C, and Ctrl+Z expose the
job-control surface interactively.

**Userspace shell, core tools, and recovery (Phases 35-39).** The TTY now has a
canonical input queue, so foreground ring-3 programs can block in `read(0)` and
receive typed lines with echo, backspace, enter, and EOF behavior. `/bin/sh`
runs as a normal foreground userspace process on that TTY, and ABI v7 adds
`spawn_args` so it can launch children with argv. `/bin/ls`, `/bin/cat`,
`/bin/echo`, `/bin/pwd`, `/bin/mkdir`, `/bin/touch`, `/bin/rm`, and
`/bin/writefile` provide basic external file/text commands. Utility app smokes
now verify editor saves, Trash emptying, and Screenshot output by reading files
back from CoolFS. The disk image also carries `/RECOVERY/README.TXT` and
`/RECOVERY/BOOT.CFG`; the `recovery` command reports boot/recovery state,
recreates standard directories, writes `/RECOVERY/LAST-REPAIR.TXT`, and toggles
`storage.fsck_on_boot`.

**Shell semantics, durability, observability, and SDK (Phases 40-44).** ABI v8
adds task-local `chdir`/`getcwd`, `stat`, `rename`, writable file descriptors,
fd-mapped `spawn_fds_args`, `sync`, and RTC `time`. Kernel syscalls now resolve
relative paths against each task's cwd, and writable fd buffers commit through
CoolFS safe writes when closed or when a task exits. `/bin/sh` has a real parser
for quotes, escapes, redirection, and one-stage pipelines, resolves bare
commands to `/bin/<name>`, and can wire child stdin/stdout through inherited
pipe or file descriptors. The toolset now includes `/bin/cp`, `/bin/mv`,
`/bin/grep`, `/bin/head`, `/bin/tail`, `/bin/date`, `/bin/uname`, `/bin/clear`,
`/bin/stat`, `/bin/sync`, and `/bin/devkit`. Terminal diagnostics gained
`sysreport [write]` and `devkit`, `/LOGS/SYSREPORT.TXT` gives a persistent
report bundle, and the generated image ships `/SDK/README.TXT`,
`/SDK/APP_TEMPLATE.RS`, and `/SDK/PACKAGE_TEMPLATE.PKG`.

**Compositor latency and smoothness (Phase 45).** The timer IRQ now requests a
passive frame tick instead of forcing a full repaint every interrupt; normal
events still request explicit full repaint. Mouse packets distinguish plain
motion from button/scroll/drag work, so simple cursor movement can restore the
old cursor rectangle from the clean shadow scene and draw the new cursor
directly to the hardware framebuffer without recomposing windows. The idle loop
polls USB input before service/deferred/network maintenance and limits deferred
work to a smaller budget so input-to-pixel latency wins. `compositor` and
`smoothness` show full-frame count, cursor-fast count, passive frame cadence,
damage rows/pixels, and cursor overlay pixel counts.

**Adaptive high refresh (Phase 46).** The desktop keeps the Phase 45 idle cadence
at 36 Hz, but real input and explicit repaint work extend a 750 ms active boost
window that paces full frames at 144 Hz. Explicit repaints mark the pacing clock
so the compositor avoids immediate duplicate passive frames after an input-driven
frame. Delayed startup commands are checked on due ticks instead of forcing a
full compose every idle loop. `compositor`/`smoothness` now reports pacing mode,
target/idle/active Hz, remaining boost time, target frame-budget ticks, and
budget misses.

**Evented userspace runtime (Phase 47).** ABI v9 adds `poll` descriptors for fd,
socket, GUI, child, and TTY sources. Kernel pipes, TTY canonical input, TCP
sockets, user GUI event queues, and child exits now keep waiter sets and wake
blocked tasks when readiness changes. `libcool::evented` exposes `PollDesc`,
`poll`, and focused wait helpers, and `/bin/polldemo` smokes timeout, pipe, and
child-exit readiness.

**Terminal/TUI platform (Phase 48).** ABI v10 adds `tty_control` for foreground
TTY mode and size queries. Kernel TTYs now track canonical/raw mode, echo,
signal delivery, and terminal cell geometry; TerminalApp forwards raw control
bytes and cursor-key escape sequences when a foreground app disables canonical
mode. Terminal output handles a practical VT subset for SGR colors, cursor
movement, and screen/line clearing. `libcool::tty` exposes mode/size helpers,
and `/bin/tuidemo` smokes raw single-key input without Enter plus ANSI-rendered
status text.

**Browser rendering phases (49-56).** The native Browser now has a more explicit
HTML/CSS rendering path: style blocks and inline styles are parsed into bounded
rules for tag, class, id, and simple compound selectors; computed style drives
hidden content, alignment, indentation, text color, backgrounds, preformatted
text, image width/height hints, and CSS2 box-model fields for margins, padding,
borders, fixed/percentage widths, and bounded heights. The Browser layout pass
now keeps content boxes separate from painted border/background boxes, wraps
text inside percentage-width boxes, and uses the laid-out box rects for
link/control hit testing. Image handling keeps the PNG decoder bounded while
recognizing PNG/JPEG/GIF/WebP dimensions for previews/placeholders. Forms render
text/search/email fields, checkboxes, radios, selects, textareas, and buttons,
preserve checked/default values, and now bind rendered controls to a bounded
DOM/form-state model. Clicks and keyboard focus edit live control values,
checkbox/radio/select/reset controls update state across reflow, GET forms
submit encoded live values, and POST forms now send
`application/x-www-form-urlencoded` request bodies through the same HTTP/TLS
response path used by normal page loads. Browser GET/POST page loads now share
a persistent cookie jar in `/CONFIG/BROWSER.COOKIES`; `Set-Cookie` handling
covers Domain, Path, Secure, and Max-Age deletion, and `browser://session`
shows redacted session state. The Browser can also be launched from Terminal
with `browser [url]`. The fixture targets from `make
smoke-phase49-browser-engine` through `make smoke-phase56-css-box-model` boot
pages or internal Browser diagnostics; the Phase 54 target submits the fixture
form over HTTPS, Phase 55 opens the session-state surface, and Phase 56 renders
the CSS box-model fixture.

**Per-process virtual memory (Phase 10).** Each user task owns a PML4 cloned
from the kernel's boot PML4 (upper-half entries 256–511 copied; lower half
empty). `vmm::new_process_pml4` handles the clone; `vmm::map_page_in` / `vmm::map_region`
insert PTEs into any address space by constructing a temporary `OffsetPageTable`
over the target PML4 frame. The scheduler writes the winning task's PML4 into
CR3 on every context switch. User stacks are mapped at `0x7FFF_0010_0000` in the
lower half — L4 index 0xFF, which the kernel never populates — so two processes
at the same VA have completely separate physical frames. A kernel-only guard page
sits below each stack. Phase 32 removed broad lazy lower-half allocation:
invalid user faults now terminate the offending task with a crashdump record,
while kernel faults still panic.

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
| 8 | Preemptive scheduler + context switching (288 Hz PIT) | **Done** |
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
| 29 | Login, sessions, and service supervision — CoolFS user DB, home ownership, umask, admin-gated mutations, credentialed services | **Done** |
| 30 | GUI login and lock screen — compositor greeter, locked-input gate, lock/logout shell hooks, smoke coverage | **Done** |
| 31 | First-run setup and account management — admin handoff, Accounts UI, account CLI, login throttling, persistence smoke | **Done** |
| 32 | User/kernel isolation hardening — supervisor kernel mappings, checked user pointers, user-fault crashdumps | **Done** |
| 33 | Process control and jobs — signals, process groups, STOP/CONT, process-bound jobs | **Done** |
| 34 | TTY sessions and foreground job control — per-terminal output routing, foreground pgids, Ctrl+C/Ctrl+Z, `fg`/`bg` | **Done** |
| 35 | Real TTY stdin — canonical `read(0)` input for foreground userspace processes | **Done** |
| 36 | Userspace shell — `/bin/sh` on the controlling TTY with argv-capable child launch | **Done** |
| 37 | Coreutils command set — external `/bin` file/text commands driven by `/bin/sh` | **Done** |
| 38 | Utility reliability — editor, Trash, and Screenshot smokes verify persisted output | **Done** |
| 39 | Recovery path — `/RECOVERY`, repair reports, and fsck-on-boot controls | **Done** |
| 40 | Shell semantics — cwd-aware paths, quoting, redirection, and one-stage pipelines | **Done** |
| 41 | Filesystem durability and metadata — writable fds, `stat`, `rename`, and `sync` | **Done** |
| 42 | App consistency — diagnostics/help surfaces expose the current runtime and devkit paths | **Done** |
| 43 | Observability — generated sysreports written under `/LOGS` | **Done** |
| 44 | Developer platform — `/SDK` docs/templates plus `/bin/devkit` | **Done** |
| 45 | Compositor latency and smoothness — passive frame ticks, cursor overlay fast path, input-first loop, telemetry | **Done** |
| 46 | Adaptive high refresh — 144 Hz active pacing, 36 Hz idle pacing, frame-budget telemetry | **Done** |
| 47 | Evented userspace runtime — ABI v9 poll for fd/socket/GUI/child/TTY readiness | **Done** |
| 48 | Terminal/TUI platform — ABI v10 TTY control, raw mode, ANSI rendering, `/bin/tuidemo` | **Done** |
| 49 | Browser engine foundation — bounded CSS cascade, styled line boxes, richer HTML fixture coverage | **Done** |
| 50 | CSS layout pass — selector specificity, colors/backgrounds, indentation, alignment, hidden content | **Done** |
| 51 | Browser forms — HTML5 control rendering and GET query submit URLs | **Done** |
| 52 | DOM/event foundation — clickable link/form/button hit boxes plus browser event fixtures | **Done** |
| 53 | DOM-backed browser forms — live control state, keyboard editing, resets, and staged POST bodies | **Done** |
| 54 | Browser POST submission — URL-encoded request bodies through the shared HTTP/TLS loader | **Done** |
| 55 | Browser session state — persistent cookie jar, session-aware GET/POST, and `browser://session` | **Done** |
| 56 | CSS2 box model and reflow — margins, padding, borders, percentage widths, and box hit testing | **Done** |

Full task checklists and technical notes in [ROADMAP.md](ROADMAP.md).
