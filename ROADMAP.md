# coolOS Roadmap

The goal is to evolve coolOS from a kernel-mode GUI demo into a real desktop
operating system — one that can load and run user programs, manage storage, and
support multiple processes without any one of them being able to crash the machine.

Phases 1–29 are complete. Phase 29 adds a persistent user database, login
sessions, home directory ownership, umask handling, admin-gated mutations, and
credentialed service supervision.

---

## ✅ Phases 1–9 — Complete

| Phase | Deliverable |
| :---: | :---------- |
| 1 | Pixel framebuffer (Mode 13h, 320×200, 8bpp) |
| 2 | PS/2 mouse driver + on-screen cursor |
| 3 | Window manager — shadow compositor, z-order, drag |
| 4 | Desktop shell — taskbar, context menu, terminal |
| 5 | Four built-in apps running as kernel-mode modules |
| 6 | High-res linear framebuffer via `bootloader 0.11` — 1280×720, 3/4bpp |
| 7 | Fluid input — lock-free keyboard queue, scratch-buffer blit, release build |
| 8 | Preemptive scheduler — naked timer ISR, round-robin context switching, 100 Hz PIT |
| 9 | Ring-3 userspace — GDT + TSS, SYSCALL/SYSRET, syscall table, iretq trampoline |

### Phase 7 implementation notes

- Removed `without_interrupts` wrapper from the main loop — it was blocking
  all IRQs (mouse, keyboard) for the entire frame blit, causing visible lag.
- Added lock-free keyboard ring buffer (`src/keyboard.rs`): the PS/2 IRQ
  handler was deadlocking by trying to acquire `WM.lock()` while `compose()`
  already held it. IRQ handler now just pushes chars into an atomic queue;
  `compose()` drains it at frame start.
- Replaced per-pixel volatile MMIO writes with a row scratch buffer:
  each row is converted from `u32` shadow pixels to packed BGR bytes into a
  stack-allocated `[u8; 5120]` (fast RAM→RAM), then flushed with one
  `copy_nonoverlapping`. Reduces framebuffer write transactions per frame from
  ~691,200 to 720 bulk copies.
- Switched to `--release` build: LLVM vectorises the pixel conversion loop
  with SSE2/AVX, removing bounds checks. Combined with the above, roughly
  10–20× faster than the debug blit.

### Phase 6 implementation notes

- Replaced `bootloader 0.9.x` + `cargo bootimage` with a host-side `disk-image`
  crate that calls `BiosBoot::new(&kernel).set_boot_config(&cfg).create_disk_image(...)`.
- `BootConfig` requests ≥1280×720; actual resolution negotiated at runtime via VBE.
- `framebuffer.rs` rewritten: accepts base address, width, height, stride, bpp, and
  pixel format from `bootloader_api::info::FrameBufferInfo` at boot time.
- Shadow buffer allocated from heap as `Vec<u32>` (width × height × 4 bytes).
- Compositor blit handles both 3bpp (24-bit, QEMU `-vga std`) and 4bpp (32-bit).
- Font rendered at 2× scale (8×8 glyph → 16×16 pixels) for readability at 1280×720.
- `build-std` moved from `.cargo/config.toml` into Makefile `-Z` flags to prevent
  it bleeding into the host-side `disk-image` crate build.
- Heap increased from 1 MiB to 32 MiB to accommodate the ~3.5 MiB shadow buffer
  and per-window pixel back-buffers.
- Full exception handler coverage added to IDT: page fault (with CR2), general
  protection fault, invalid opcode — all print diagnostics via `println!`.
- Debug console mirroring added (`-debugcon stdio`, port 0xE9) so `println!` output
  appears in the host terminal even when the desktop is rendering.

---

## ✅ Phase 8 — Preemptive Scheduler

**Goal:** Multiple concurrent execution contexts sharing the CPU via timer-driven
preemption. This is the hardest single phase and everything from Phase 9 onwards
depends on it.

- [x] Define a `Task` struct: kernel stack, saved register state (all GP registers +
      `rflags`, `rsp`, `rip`), task ID, status (`Ready` / `Running` / `Blocked`).
- [x] Allocate a fixed kernel stack per task (e.g. 64 KB from the heap).
- [x] Implement context switching — a naked timer ISR (`timer_naked` in
      `src/interrupts.rs`) pushes all 15 GP registers, calls `timer_inner` to
      get the next task's RSP, switches the stack pointer, pops the new task's
      registers, and `iretq`s back into it.
- [x] Build a simple round-robin run-queue (`Vec<Task>` in `src/scheduler.rs`).
- [x] Hook the timer IRQ (IRQ0) to call the scheduler: save the interrupted task's
      full register frame, pick the next `Ready` task, switch context.
- [x] `TaskStatus::Blocked` variant and structural support for `block()` / `unblock(id)`
      exist; full wiring to I/O events deferred to Phase 9.
- [x] Port the existing main loop (compositor tick + `hlt`) to run as the idle task
      (task 0, uses the kernel boot stack — no separate allocation needed).
- [x] Historical verification: `counter_task` (then task 1) incremented
      `BACKGROUND_COUNTER` in a tight loop while the WM loop (idle task) ran the
      desktop; the System Monitor displayed the counter in cyan, confirming both
      tasks made progress. The current kernel increments this counter from IRQ0.

**Exit criteria:** at least two kernel tasks preempt each other correctly under the
timer; no stack corruption; `hlt` in the idle task still fires when no other task is runnable.

### Phase 8 implementation notes

- Replaced the `extern "x86-interrupt"` timer handler with a `#[unsafe(naked)]`
  function (`timer_naked`) that manually pushes all 15 GP registers, calls the
  Rust helper `timer_inner` (which increments `TICKS`, requests a repaint, and
  sends PIC EOI), then does `mov rsp, rax` to switch stacks before popping the
  new task's registers and executing `iretq`. `sym timer_inner` in the
  `naked_asm!` block is the correct way to call a Rust function from a naked ISR.
- IDT timer entry set via `set_handler_addr(VirtAddr::new(timer_naked as *const () as u64))`
  instead of `set_handler_fn` because the naked function does not conform to the
  `extern "x86-interrupt"` ABI.
- New `src/scheduler.rs` owns `Task` (64 KiB heap stack + saved `stack_ptr`),
  `Scheduler` (round-robin `Vec<Task>`), and `pub static SCHEDULER: spin::Mutex<Scheduler>`.
  `Scheduler::empty()` is `const fn` so the global can be initialised without a heap.
- New-task stack initialisation writes a 20-word (160-byte) fake context block:
  15 zeroed GP-register slots followed by a synthetic 5-word interrupt frame
  (RIP = entry fn, CS/SS read live via inline asm, RFLAGS = 0x202, RSP = stack_top − 8).
  On first restore the `iretq` jumps straight to the entry function with correct
  System V AMD64 ABI stack alignment.
- Idle task (index 0) is the kernel boot stack — `stack_ptr` starts as 0 and is
  written on the very first timer preemption, before any switch-away can occur.
- Scheduler initialisation is wrapped in `without_interrupts` to prevent a
  deadlock if the timer fires while `SCHEDULER.lock()` is held during `spawn`.
- `timer_schedule` returns `current_rsp` unchanged when the task list is empty,
  making timer IRQs that fire before task initialisation completely harmless.
- `#[unsafe(naked)]` / `naked_asm!` (stable since Rust 1.88 nightly) replaced
  the old `#[naked]` + `asm!` + `options(noreturn)` spelling.

---

## ✅ Phase 9 — Userspace & System Calls

**Goal:** Ring-3 execution and a minimal syscall interface so that code outside the
kernel can request kernel services without being able to crash it.

- [x] Set up the GDT with four segments: kernel code (ring 0), kernel data (ring 0),
      user code (ring 3), user data (ring 3). Load via `lgdt`.
- [x] Set up the TSS — populate `rsp0` with a dedicated 64 KiB ISR stack so that
      IRQs/exceptions from ring 3 switch to a valid kernel stack.
- [x] Implement `SYSCALL`/`SYSRET` (set `STAR`, `LSTAR`, `SFMASK` MSRs). The syscall
      entry stub saves user registers, dispatches on `rax`, and returns.
- [x] Initial syscall table: `0 exit`, `1 write` (to terminal), `2 yield`, `3 getpid`.
- [x] Implement `jump_to_userspace(entry: u64, user_stack: u64)` — push a fake
      `iretq` frame (user CS/SS, `rflags` with IF set, entry RIP, user RSP) and `iretq`.
- [x] Verify: a minimal Rust userspace stub (syscall via `asm!`) calls `write` to
      print `[ring 3] Hello from userspace!` to the terminal, then calls `exit`.

**Exit criteria:** the kernel can jump to a ring-3 stub; the stub can make a
`write` syscall that prints to the terminal window; an illegal memory access in
userspace generates a #PF that the kernel handles without crashing.

### Phase 9 implementation notes

- New `src/gdt.rs`: `GlobalDescriptorTable` built with `Descriptor::kernel_code_segment`
  (0x08), `Descriptor::kernel_data_segment` (0x10), `Descriptor::user_data_segment`
  (0x18), `Descriptor::user_code_segment` (0x20), and a 64-bit TSS descriptor (0x28).
  `CS::set_reg` / `SS::set_reg` / `load_tss` called after `lgdt`. TSS `privilege_stack_table[0]`
  points to the top of a static 64 KiB `ISR_STACK`; the CPU switches to this on any
  IRQ/exception entry from ring 3.
- STAR MSR: bits[47:32] = 0x08 (kernel CS), bits[63:48] = 0x10 (SYSRET base).
  SYSCALL → CS=0x08, SS=0x10; SYSRET → CS=0x20|RPL3, SS=0x18|RPL3.
- `syscall_entry` (naked): saves user RSP in r10, switches to a static 64 KiB
  `SYSCALL_KERNEL_STACK` via `mov rsp, [rip + SYSCALL_KERNEL_STACK_TOP]`, pushes
  user RSP/RIP(rcx)/RFLAGS(r11) + callee-saved regs, shuffles rax/rdi/rsi/rdx into
  rdi/rsi/rdx/rcx for the SysV ABI call to `syscall_dispatch`, then restores with
  `pop rsp` + `sysretq`.
- `sys_write` pushes bytes into a lock-free ring buffer (`SYSCALL_OUTPUT`, same design
  as `keyboard.rs`). The compositor drains it into the terminal at the start of each
  `compose()` call — avoiding the WM lock deadlock that would arise if `sys_write`
  tried to acquire `WM.lock()` while the idle/WM task already holds it.
- `sys_exit` marks the current scheduler task `Blocked`. The naked handler still
  sysretqs back to ring 3; the stub then spins with `core::hint::spin_loop()` until
  the next timer tick, at which point the scheduler permanently switches away (Blocked
  tasks are never selected as `next`).
- `mark_all_user_accessible` (new in `memory.rs`) walks all four levels of the active
  page table and sets `USER_ACCESSIBLE` on every present PTE, then flushes the TLB.
  Phase 9 is a single-address-space model — the user stub lives in the kernel binary
  and the user stack is a kernel static; making all pages user-accessible lets ring-3
  code execute and access data without a #PF. Phase 10 replaces this with per-process
  page tables.
- PIT reprogrammed to 100 Hz (`init_pit(100)` in `interrupts.rs`) as part of Phase 8
  fix: divisor = 1,193,180 / 100 = 11,931. Renders go from ~9 fps to ~50 fps.

---

## ✅ Phase 10 — Virtual Memory per Process

**Goal:** Each process gets its own isolated page-table hierarchy so processes
cannot read or corrupt each other's memory.

- [x] Extend the `Task` struct with a `PhysFrame` pointing to its top-level PML4.
- [x] On task creation, clone the kernel's PML4 entries into the new process's PML4
      (so kernel mappings are shared), leaving user-space entries empty.
- [x] On context switch, load the new process's PML4 physical address into `cr3`.
      Flush the TLB (or use PCID/ASID to avoid full flushes).
- [x] Implement `mmap(addr, len, flags)` — find free virtual pages in the process's
      address space, allocate physical frames, insert PTEs.
- [x] Implement lazy allocation: map pages as present only on first access; handle
      `#PF` by allocating and mapping the faulting page.
- [x] Guard pages: map a kernel-only page below each stack to catch overflows.
- [x] Verify: two userspace processes with the same virtual addresses for their stacks
      and data cannot read each other's values.

**Exit criteria:** two concurrently running userspace processes are fully isolated;
a write to an unmapped address in one process does not affect the other.

### Phase 10 implementation notes

- New `src/vmm.rs` module holds a global `spin::Mutex<Option<BootInfoFrameAllocator>>`
  and the physical-memory offset.  All page-table work (frame allocation, PML4
  creation, page mapping, CR3 switching) goes through `vmm::`.
- `BootInfoFrameAllocator` gains `next()` and `init_from(regions, start)` so the
  heap can be initialised with one allocator instance and the VMM gets a second
  instance that picks up at the next free frame — no frames are double-allocated.
- `Task` gains `pml4: Option<PhysFrame>`.  Kernel tasks (`None`) share the boot PML4.
  User tasks (`Some`) get their own PML4.  The scheduler calls `vmm::switch_to` on
  every context switch to a task with `Some(pml4)`.
- `vmm::new_process_pml4()` allocates a zeroed 4 KiB frame and shallow-copies L4
  entries 256–511 from the active (boot) PML4.  Lower-half entries start empty so
  each process's user mappings are private.
- User stacks live at `USER_STACK_TOP = 0x0000_7FFF_0010_0000` (L4 index 0xFF —
  confirmed empty in the boot PML4).  Each process gets `USER_STACK_SIZE = 64 KiB`
  of writable, user-accessible pages mapped there, backed by private physical frames.
- Guard page: one kernel-only (`PRESENT`, no `WRITABLE`, no `USER_ACCESSIBLE`) page
  mapped at `USER_STACK_BOTTOM - 4096`.  A ring-3 stack overflow hits a protection-
  violation `#PF` which the fault handler does not lazily recover.
- Lazy `#PF` handler: if the fault is not-present + user-mode + lower-canonical-half,
  allocates a zeroed frame and maps it into the current process's PML4.  All other
  faults (protection violations, kernel faults) still panic.
- `sys_mmap(addr, len, flags)` (syscall 4): maps `len` bytes at `addr` in the
  calling process's address space.  `flags & 1` controls writability.
- `sys_getpid()` (syscall 3) now returns `scheduler.current` (the task index).
- Isolation proof: `userspace.rs` spawns two processes (`pid=1`, `pid=2`), both
  entering `user_stub` at the same kernel `.text` virtual address and using the
  same user stack VA.  Each writes `0xDEAD_0000 + pid` to the stack-top slot and
  reads it back.  Both print `sentinel ok` to the terminal, confirming their stacks
  map to different physical frames.

---

## Phase 11 — Filesystem & Storage ✓

**Goal:** Programs and data live on disk. The kernel can load files by name.

- [x] Write an ATA PIO driver to read 512-byte sectors from a virtual disk image.
- [x] Implement a FAT32 layer — BPB parsing, FAT chain walking, short-name/LFN
      directory traversal, file lookup by absolute path, cluster-to-sector mapping,
      and file-manager mutations for create/write/rename/delete/copy.
- [x] Expose a VFS layer: `vfs_open(path)`, `vfs_read(fd, buf, len)`, `vfs_close(fd)`.
- [x] Map VFS operations to syscalls: `sys_open` (5), `sys_read` (6), `sys_close` (7).
- [x] Build a 64 MiB FAT32 disk image in the Makefile using a host-side `fs-image`
      tool (`fatfs` crate) and attach it to QEMU as the IDE primary-bus slave.

**Implementation notes:**

- `src/ata.rs`: targets primary ATA bus, slave device (0xB0 in DRIVE_HDR).
  Writes `0x02` to the Device Control Register (port `0x3F6`) before each
  command to assert nIEN=1, preventing the drive from firing IRQ14.  Uses
  LBA28 mode with BSY→select→DRQ polling; two independent 10 M-iteration
  timeout loops return `false` without hanging.
- `src/fat32.rs`: `Bpb::load()` parses the boot sector.  `fat_next()` chases
  FAT32 chains 4 bytes at a time.  Directory scanning assembles LFN fragments
  and falls back to short names. `read_file(path)` walks `/`-split components
  top-down and returns `Option<Vec<u8>>`; create/write/rename/delete/copy
  helpers back the desktop shell and File Manager.
- `src/vfs.rs`: a 16-slot `FdTable` protected by a `spin::Mutex`.  `vfs_open`
  calls `fat32::read_file` and caches the entire file in a heap `Vec`; `vfs_read`
  copies into the caller's buffer with an offset cursor.
- `interrupts.rs`: `mask_unused_irqs()` called after PIC init masks IRQ3–7 on
  PIC1 and IRQ8–11, IRQ13–15 on PIC2.  Only IRQ0 (timer), IRQ1 (keyboard),
  IRQ2 (cascade), and IRQ12 (mouse) remain unmasked, preventing unhandled
  interrupt vectors from triggering `#GP → #DF`.
- `vmm.rs`: added `switch_to_boot()` which stores the boot PML4 physical address
  at `vmm::init` time and writes it to CR3 when the scheduler resumes a kernel
  task (`pml4 = None`).

**Exit criteria met at the time:** the `fs-test` kernel task opened
`/bin/hello.txt` from the FAT32 image on boot and printed its contents to the
console. Phase 26 later superseded this path with CoolFS at `/` and a
synchronous `fs_test_once()` boot check.

---

## Phase 12 — ELF Loader & Process Spawning

**Goal:** The kernel can load a compiled ELF binary from disk, map it into a new
address space, and jump to its entry point.

- [x] Parse ELF64 headers — validate magic, machine type (`x86_64`), entry point.
- [x] Walk `PT_LOAD` segments: allocate virtual pages in the process's address space,
      read segment data from the file into those pages, set PTE flags from segment
      flags (`R`, `W`, `X`).
- [x] Allocate a user stack and map it.
- [x] Build an `argv`/`envp` array on the user stack in the System V AMD64 ABI layout.
- [x] Create a new `Task`, set its `rip` to the ELF entry point and `rsp` to the top
      of the user stack, add it to the run-queue.
- [x] Add a `sys_exec(path)` syscall that calls the ELF loader and replaces the
      calling process's address space.
- [x] Compile a minimal `hello` binary (Rust `#![no_std]` + syscall shim) and
      ship it in `/bin/hello` on the disk image.
- [x] Add an `exec <path>` command to the terminal app.

**Exit criteria:** typing `exec /bin/hello` in the terminal spawns a real
userspace process that prints to the screen and exits cleanly.

**Current status:** complete.

### Phase 12 implementation notes

- `src/elf.rs` now validates ELF64 headers, walks `PT_LOAD` segments, allocates
  a fresh per-process PML4, maps a private user stack, builds a minimal
  `argc=1` / `argv[0]=path` / empty-`envp` startup frame, and can either spawn
  a new task or prepare a loaded image for `sys_exec`.
- `scheduler.rs` gained `spawn_user`, which builds an initial ring-3 interrupt
  frame directly instead of going through a trampoline stub.
- `syscall.rs` now exposes syscall 8, `exec(path, len)`. It loads a new ELF
  image, updates the current task's `pml4`, switches CR3 immediately, and
  rewrites the saved syscall return frame so `sysretq` enters the new image.
- `vmm::new_process_pml4()` now clones from the boot/kernel PML4 rather than
  the currently active user CR3. Without that fix, `sys_exec` inherited stale
  user mappings and collided while remapping the new stack/segments.
- The host-side build now produces two user binaries: `/bin/hello` prints a
  line and exits; `/bin/exec` demonstrates true in-place `sys_exec` by replacing
  itself with `/bin/hello`.

---

## ✅ Phase 13 — Inter-Process Communication

**Goal:** Processes can send data to each other and to the GUI without going through
the kernel's internal Rust data structures.

- [x] Implement anonymous pipes — a fixed-size ring buffer in kernel memory; `sys_pipe`
      returns two file descriptors (read end, write end).
- [x] Block a reader when the pipe is empty; unblock it when the writer produces data.
- [x] Implement shared memory — `sys_shmem_create(len)` allocates physical frames and
      maps them into the caller's address space; `sys_shmem_map(id)` maps the same
      frames into another process.
- [x] Design a simple message-passing protocol so GUI apps can send window events
      (key presses, mouse clicks) to user processes via a pipe rather than via the
      kernel's internal WM dispatch.
- [x] Port one existing built-in app (e.g. Terminal) to run as a real userspace
      process communicating with the WM over a pipe.

**Exit criteria:** a userspace terminal process receives keyboard events from the
WM via a pipe and writes output back via `sys_write`; the WM renders it without
any shared Rust state.

**Current status:** complete. Terminal ported: `term` command in kernel TerminalApp
spawns `/bin/terminal` as a ring-3 process with a pipe for stdin; the
userspace terminal reads keyboard event packets (same format as keydemo), echoes
input locally, processes commands (help/clear/echo/exec/info/uptime), and writes
output via `sys_write` which the compositor drains into the kernel TerminalApp
window. The `keydemo` command still works for event-packet streaming to
keyecho.

All Phase 13 items done.

---

## Phase 14 — USB & Modern Input

**Goal:** Input works on real hardware, not just in QEMU with PS/2 emulation.

- [x] Write an xHCI host controller driver — detect the MMIO BAR via the PCI config
      space, initialise the command ring, event ring, and default control transfer rings.
- [x] Implement USB enumeration — detect connected devices, read device/configuration
      descriptors, and identify boot HID keyboard/mouse interfaces plus interrupt IN
      endpoints.
- [x] Finish the USB HID runtime — switch devices into a usable protocol, configure
      interrupt endpoints, and feed keyboard/mouse events into the existing input path.
- [x] Remove the PS/2 driver dependency for systems that do not support it — ACPI
      FADT IAPC_BOOT_ARCH bit 1 is read; PS/2 fallback is only enabled when the
      hardware reports an 8042-compatible controller.

**Exit criteria:** coolOS boots on real x86_64 hardware and accepts keyboard and
mouse input via USB.

---

## ✅ Phase 15 — Networking

**Goal:** The kernel can send and receive Ethernet frames; userspace can open TCP
connections.

- [x] Write a virtio-net driver (MMIO or PCI) to transmit and receive raw Ethernet
      frames.
- [x] Implement ARP, IPv4, ICMP (ping), UDP, and TCP in the kernel or as a userspace
      network stack over shared memory.
- [x] Expose `sys_socket`, `sys_connect`, `sys_send`, `sys_recv` syscalls.
- [x] Ship a `wget` binary in `/bin/` as a proof-of-concept.

**Exit criteria:** `exec /bin/wget http://example.com/` fetches a real HTTP
response over virtio-net/QEMU user networking and writes it to the terminal.

**Current status:** complete.

### Phase 15 implementation notes

- `src/virtio_net.rs` binds QEMU's legacy PCI virtio-net device, enables I/O
  decode and bus mastering, allocates contiguous DMA memory for RX/TX virtqueues,
  posts RX buffers, and transmits Ethernet frames through polling queue notify.
- `src/net.rs` owns the Ethernet stack: ARP cache, static QEMU user-net IPv4
  config (`10.0.2.15/24`, gateway `10.0.2.2`, DNS `10.0.2.3`), ICMP echo,
  UDP DNS queries, multi-A-record DNS handling, and a minimal TCP client state
  machine for SYN/SYN-ACK/ACK, payload ACKs, and close-on-FIN.
- Syscalls 19-22 expose TCP stream sockets to ring-3 programs:
  `socket(domain, type, proto)`, `connect(socket, ipv4, port)`,
  `send(socket, buf, len)`, and `recv(socket, buf, len)`.
- `/bin/wget` parses `http://host/path`, resolves DNS when needed, connects to
  port 80, sends an HTTP/1.0 request, and streams the response through stdout.
- `make run-net` and `make smoke-net-wget` attach `virtio-net-pci` with QEMU
  user networking; the smoke also attaches USB input so command injection works
  on ACPI systems without PS/2 fallback.

---

## Milestone summary

| Phase | Deliverable | Depends on |
| :---: | :---------- | :--------- |
| 6 | High-resolution framebuffer (`bootloader 0.11`, VBE) | 1–5 |
| 7 | Input lag fixes — keyboard queue, scratch blit, release build | 6 |
| 8 | Preemptive scheduler, context switching | 7 |
| 9 | Ring-3 userspace + syscall interface | 8 |
| 10 | Per-process virtual memory, isolation | 9 |
| 11 | Filesystem (FAT32), VFS, disk driver | 10 |
| 12 | ELF loader, `exec`, real user programs | 11 |
| 13 | Pipes, shared memory, IPC | 12 |
| 14 | USB HID — real hardware input | 9 |
| 15 | Networking (virtio-net, TCP/IP) | 13 |
| 16 | UI Polish — desktop surface, window chrome, taskbar & start menu | 12 |
| 17 | Browser Foundation — HTTP/1.1, redirects, chunked responses, browser UX | 15, 16 |
| 18 | HTTPS/TLS Foundation — verified TLS 1.3 over kernel TCP | 15, 17 |

---

## ✅ Phase 16 — UI Polish

**Goal:** Make coolOS look and feel like a real desktop OS. No kernel changes are
required — everything in this phase lives in the compositor, window manager, and
shell layer. The phase is split into three areas: the desktop surface, the window
chrome, and interactive shell widgets.

### 16a — Desktop surface

- [x] **Wallpaper gradient** — the desktop background renders a smooth vertical
      colour gradient (deep blue → teal) using a per-row lerp in `shell/desktop.rs`
      rather than a flat fill. Redrawn only when the resolution changes.
- [x] **Desktop icons** — `DesktopIcon` structs hold a label, a 32×32 pixel glyph
      (drawn with the 8×8 font scaled 2×), an (x, y) grid position, and a selected
      flag. Icons are hit-tested on left click; double-click spawns the associated app.
- [x] **Icon selection highlight** — a 2px white border is drawn around the icon
      bounding box when `selected == true`; cleared when the user clicks the wallpaper.
- [x] **Icon grid snap** — icons are placed on a 64×72 pixel grid anchored to the
      left edge of the desktop; dragging an icon snaps it to the nearest cell on
      mouse-button release.

### 16b — Window chrome

- [x] **Styled title bar** — title bar background uses a two-tone gradient (light
      blue active, mid-gray inactive). Active window is determined by z-order top.
- [x] **Minimise / maximise / restore buttons** — three 16×16 pixel buttons rendered
      in the title bar right section. Minimise hides the window (sets `visible=false`,
      taskbar entry remains). Maximise saves `(x, y, w, h)` and resizes the window to
      fill the desktop area. Restore returns to the saved geometry.
- [x] **Window border** — a 1px solid border (colour matches inactive title bar)
      surrounds each window's content area; drawn by the compositor after blitting the
      window's back-buffer, so apps do not need to draw it themselves.
- [x] **Resize handle** — a 6×6 drag zone in the bottom-right corner of each window;
      dragging it resizes the window and reallocates its back-buffer.
- [x] **Scrollbars** — drawn by the compositor when a window's logical content height
      exceeds its physical height. A `ScrollState { offset, content_h, view_h }` field
      is added to `Window`; apps update `content_h` and the compositor maps scroll-wheel
      events to `offset` changes and redraws the scrollbar track and thumb.

### 16c — Taskbar & start menu

- [x] **Start button** — leftmost taskbar element; click toggles the start menu popup.
      Rendered as a raised 3D-style button (light top/left border, dark bottom/right).
- [x] **Start menu popup** — a 160×220px panel rendered above the taskbar listing
      installed apps. Each entry is a 20px-tall clickable row with hover highlight.
      Clicking a row spawns the app and closes the menu; clicking elsewhere dismisses it.
- [x] **Taskbar window buttons** — one button per non-minimised window, showing a
      truncated title. Clicking the button of the active window minimises it; clicking
      any other window's button raises it to the top of the z-order.
- [x] **Clock area** — rightmost taskbar section displays a static tick counter
      formatted as `HH:MM` (ticks ÷ 18 for approximate seconds). Redrawn each
      compositor frame.
- [x] **Notification area** — reserved slot left of the clock; intentionally left
      empty until Phase 14/15 land hardware (network, volume, battery).

**Exit criteria:** the compositor, shell, and window manager changes compile cleanly
with no regressions to existing apps; desktop icons launch apps on double-click;
windows minimise, maximise, and restore correctly; the start menu opens and closes;
the taskbar reflects the current window list; the wallpaper gradient renders at boot.

**Current status:** complete. All three sub-areas (desktop surface, window chrome,
taskbar/start menu) have shipped. Scrollbars are wired end-to-end: Text Viewer and
File Manager re-render when the thumb is dragged, PS/2 IntelliMouse 4-byte mode is
negotiated at boot, and the scroll wheel dispatches to the topmost window under the
cursor. The notification slot is reserved for Phase 14/15 hardware status icons.

### Phase 16 implementation notes

- All compositor-side changes live in `src/wm/` and `src/shell/`; no `syscall.rs`,
  `scheduler.rs`, or VMM code was touched.
- `Window` gained three new fields: `minimised: bool`, `maximised: bool`, and
  `saved_rect: Option<(i32, i32, i32, i32)>` for min/max/restore bookkeeping.
  The compositor skips blitting a window when `minimised == true`.
- `DesktopIcon` hit-testing uses the same coordinate transform already applied to
  window clicks — subtract the desktop origin, compare against `(x, y, 48, 64)`
  bounding boxes. No new input path was required.
- The wallpaper gradient is computed once into a `Vec<u32>` on first draw and cached;
  the compositor blits it with `copy_nonoverlapping` the same way it blits windows.
  Incremental dirty-rect tracking means the wallpaper is only redrawn in the regions
  uncovered by a moved or closed window.
- Active-window detection for title bar colouring uses `wm.windows.last()` (the top
  of the z-stack), consistent with how click dispatch already determines focus.
- Start menu popup is rendered as a compositor overlay — it is not a `Window` struct
  and does not participate in z-order or drag. It is drawn after all windows in each
  compositor frame when `shell.start_menu_open == true`. This avoids the complexity
  of a zero-title popup window and the risk of the menu being occluded by other windows.
- Taskbar button widths are computed dynamically: total taskbar width minus the start
  button and clock area, divided by the number of open windows, clamped to a minimum
  of 80px. Titles are truncated with a `…` suffix when they exceed the button width.
- 3D-style buttons (start button, taskbar entries) use a four-pixel border trick:
  top and left edges are drawn in `LIGHT_GRAY`, bottom and right edges in `DARK_GRAY`,
  giving a raised appearance without any additional rendering primitives.

---

## ✅ Phase 17 — Browser Foundation

**Goal:** Turn networking into a usable desktop browsing surface without pretending
TLS and JavaScript exist before the crypto/rendering foundations are in place.

- [x] Ship a native Web Browser app with a toolbar, address/search box, status bar,
      clickable links, back/forward navigation, refresh, scrolling, and desktop/launcher
      integration.
- [x] Add browser internal pages: `browser://home`, `browser://history`,
      `browser://bookmarks`, and `browser://search?q=...`.
- [x] Add session history and persistent local bookmarks.
- [x] Upgrade kernel HTTP requests to HTTP/1.1 with a user agent, accept header, and
      connection-close semantics.
- [x] Follow HTTP redirects up to a bounded limit, including relative and absolute
      `Location` values for plain HTTP.
- [x] Decode `Transfer-Encoding: chunked` responses before handing the response to
      apps.
- [x] Raise the kernel HTTP response cap to support larger text pages while keeping
      a hard upper bound.
- [x] Improve HTML text extraction: skip script/style bodies, preserve image alt text,
      wrap long words, handle more entities, and resolve links against the final URL
      after redirects.
- [x] Keep boot clean: the desktop starts with no app windows open, but manual session
      restore remains available from the launcher.

**Current status:** complete. Phase 18 adds real TLS/X.509/crypto support for
HTTPS rather than a fake port-443 passthrough.

### Phase 17 implementation notes

- `src/net.rs` owns HTTP redirect following and chunked transfer decoding so Terminal,
  browser, and syscall callers share one HTTP implementation.
- `src/apps/browser.rs` remains a native no_std GUI app; it does not embed a web
  engine. It renders a useful text view over basic HTML and exposes internal browser
  pages for local state.
- `/bin/wget` now sends an HTTP/1.1 request with a coolOS user agent, keeping it as a
  raw userspace socket demo.
- The next browser phase is richer page rendering and broader web compatibility:
  a larger trust store, stronger hostname matching coverage, CSS/layout, images,
  and eventually JavaScript.

---

## ✅ Phase 18 — HTTPS/TLS Foundation

**Goal:** Give the native browser and terminal a real HTTPS path without
pretending encryption or certificate checks exist.

- [x] Add a no_std TLS 1.3 client over the existing kernel TCP socket layer.
- [x] Use hardware RNG entropy for TLS key material; fail closed when `RDRAND`
      is unavailable instead of falling back to predictable bytes.
- [x] Use the RTC clock for certificate validity checks.
- [x] Validate X.509 certificate chains against built-in DER trust roots.
- [x] Enable server-name verification through the TLS verifier and surface the
      selected trust root in Browser status text and Terminal output.
- [x] Add HTTPS URL parsing and redirect handling across `http://` and
      `https://` locations.
- [x] Add a `https` terminal command and make Browser URL/search defaults prefer
      `https://example.com/`.
- [x] Keep the build reproducible for the custom kernel target by pinning crypto
      crates to portable software backends in `.cargo/config.toml`.
- [x] Add `make smoke-net-https` and `make smoke-net-browser-https`, which boot
      QEMU with virtio-net/USB and verify both terminal and Browser HTTPS paths.

**Current status:** complete for the initial verified HTTPS path. The trust store
is generated from the host certificate bundle and the active root is reported in
the UI. Phase 19 adds the SAN/CN hostname edge-case coverage; HTTPS support here
is still real TLS 1.3 with chain/signature/time checks, not plaintext port-443
fetching.

### Phase 18 implementation notes

- `src/tls.rs` wraps `embedded-tls` blocking TLS over a `KernelTcpStream` adapter
  backed by `net::socket_open/connect/send/recv`.
- `src/entropy.rs` provides RDRAND-backed `rand_core::RngCore` entropy for the
  TLS provider. QEMU smoke runs use `-cpu max` so the instruction is exposed.
- `src/tls_roots.rs` embeds DER trust roots in the kernel image. `net status`
  lists the TLS cipher/group and loaded roots.
- HTTPS requests reuse the same HTTP/1.1 request builder, response cap, redirect
  limit, and chunked decoder as plain HTTP.
- `src/apps/browser.rs` now accepts both HTTP and HTTPS URLs, resolves relative
  links against the final scheme, defaults URL-looking input to HTTPS, and shows
  the TLS trust root in the status line for verified HTTPS responses.
- `src/apps/terminal.rs` now supports both `http` and `https` commands. HTTPS
  output includes the resolved address and trust root before printing the
  response.
- The smoke paths log `[tls] https example.com/ via ... root=AAA Certificate Services`
  to QEMU debugcon so CI can verify the encrypted path; the Browser smoke also
  captures a framebuffer and asserts an application window is visible.

---

## ✅ Phase 19 — Browser Rendering & Trust Store

**Goal:** Move the browser from text-only HTML extraction toward a small but real
native rendering surface while hardening the HTTPS trust layer.

- [x] Add a constrained PNG decoder for 8-bit RGB/RGBA, non-interlaced PNG files.
- [x] Render direct `image/png` responses and local PNG files inline in the
      Browser document pane.
- [x] Keep the decoder bounded by a maximum pixel count and reject unsupported
      PNG formats rather than risking unbounded allocations.
- [x] Add a boot selftest for PNG inflate/filter/decode.
- [x] Add richer HTML layout primitives: tables, block quotes, and better spacing.
- [x] Add image extraction/fetching from HTML pages instead of only direct image URLs.
- [x] Strengthen hostname verification coverage around SAN/CN edge cases.
- [x] Add browser smoke coverage for image responses.

**Current status:** complete. The browser renders direct PNG responses, local
PNG files, local HTML files, and up to four bounded PNG images discovered from
HTML `<img>` elements. The document renderer now formats headings, lists, block
quotes, simple tables, pre/code blocks, and spacing more deliberately. TLS
hostname matching has boot selftest coverage for SAN-over-CN behavior,
case-insensitive wildcard matching, trailing dots, wildcard scope limits, IP
SANs, and IP/CN fallback rejection.

---

## ✅ Phase 20 — Userspace SDK

**Goal:** Stop treating each userspace binary as a one-off syscall experiment.
Create a small no_std SDK that owns program startup, argv parsing, syscall
numbers, and reusable wrappers for the APIs that user programs already depend
on.

- [x] Add `userspace/libcool`, a standalone no_std crate excluded from the
      kernel workspace target inheritance.
- [x] Provide one `entry!` macro for `_start`, initial stack parsing, panic
      aborts, and `Args` access.
- [x] Centralize syscall numbers and raw syscall assembly in the SDK instead of
      duplicating it across every `/bin` program.
- [x] Add SDK modules for process control, file IO, pipes, mmap, shared memory,
      WM event packets, DNS/HTTP helpers, and TCP sockets.
- [x] Add formatting support through `print!` and `println!` over `sys_write`.
- [x] Migrate the existing userspace binaries (`hello`, `exec`, `read`, `pipe`,
      `piperd`, `pipewr`, `keyecho`, `terminal`, `netdemo`, and `wget`) onto
      `libcool`.
- [x] Add `/bin/sdkdemo` as an SDK coverage binary for argv, VFS reads, pipes,
      shared memory, and mmap.
- [x] Embed `/bin/sdkdemo` into `fs.img` and extend the boot filesystem layout
      with standard user folders (`/Documents`, `/Pictures`, `/Desktop`).
- [x] Surface SDK information through the ABI command and add
      `make smoke-userspace-sdk` to exercise the new binary under QEMU.

**Current status:** complete. The userspace tree now has a real SDK boundary:
new programs can opt into `libcool::entry!`, use `Args`, call typed wrappers,
and avoid hand-written syscall boilerplate. The smoke target launches
`exec /bin/sdkdemo alpha` in QEMU and verifies the SDK banner, argv contents,
pipe round-trip, mmap write/read, and clean completion.

---

## ✅ Phase 21 — Userspace GUI Runtime

**Goal:** Let a ring-3 ELF program own a desktop window without becoming a
kernel-mode app module. The kernel still owns window chrome and compositing, but
userspace owns the content pixels and receives compact input/window events.

- [x] Add GUI syscalls: `gui_open`, `gui_present`, `gui_poll_event`, and
      `gui_close`.
- [x] Bump the userspace ABI to version 4 and surface the GUI calls through the
      terminal `abi` command.
- [x] Add `UserGuiApp`, a compositor window variant with an owner pid, window
      handle, pixel surface, and bounded 16-byte event queue.
- [x] Route keyboard, mouse, resize, and close events from the compositor into
      userspace-owned GUI windows.
- [x] Ensure exiting, killed, or faulted tasks have their owned GUI windows
      cleaned up by the scheduler.
- [x] Add `libcool::gui` wrappers for opening windows, presenting `u32`
      framebuffers, polling events, closing windows, and drawing simple
      rectangles/borders/text through a no_std `Canvas`.
- [x] Add `/bin/guidemo`, a ring-3 GUI proof app that draws its own pixel UI,
      presents frames, polls events, and keeps the window alive under QEMU.
- [x] Wire `/bin/guidemo` into the disk image, app metadata, launcher, and
      `make smoke-userspace-gui`.

**Current status:** complete. `exec /bin/guidemo` opens a real desktop window
from ring 3, draws via a userspace pixel buffer, and drives updates through
`libcool::gui`. The launcher also exposes "GUI Demo" and spawns the ELF from
`/bin/guidemo`.

---

## ✅ Phase 22 — Userspace Utility Suite

**Goal:** Stop treating everyday desktop utilities as kernel-mode app modules.
Use the Phase 21 GUI runtime and the SDK to ship real ring-3 GUI applications
for notes, text editing, Trash management, and screenshots.

- [x] Bump the userspace ABI to version 5.
- [x] Add utility syscalls for file writes, directory creation, recursive
      deletion, directory listing, and queued focused-window screenshots.
- [x] Add `libcool::fs` wrappers for file reads/writes, directory listing,
      recursive delete, directory creation, and screenshot requests.
- [x] Add `/bin/notes`, a ring-3 GUI scratchpad backed by
      `/documents/notes.txt`.
- [x] Add `/bin/editor`, a ring-3 GUI text editor backed by
      `/documents/editor.txt`.
- [x] Add `/bin/trash`, a ring-3 GUI Trash utility that lists `/Trash` and can
      permanently empty entries through recursive delete.
- [x] Add `/bin/screenshot`, a ring-3 GUI capture utility that queues a PPM
      screenshot into `/Pictures`.
- [x] Update the launcher to prefer the userspace utility ELFs while retaining
      kernel utility fallbacks if a disk image is missing a binary.
- [x] Embed the new utility binaries into `fs.img` and add
      `make smoke-userspace-utils`.

**Current status:** complete. The launcher and terminal can start the utility
suite as userspace GUI apps (`exec /bin/notes`, `exec /bin/editor`,
`exec /bin/trash`, and `exec /bin/screenshot`). The new smoke target runs each
utility in deterministic smoke mode and verifies file write, directory listing,
recursive delete, and screenshot queue behavior.

---

## ✅ Phase 23 — App Lifecycle + File Open Plumbing

**Goal:** Treat userspace GUI utilities as managed desktop apps instead of
anonymous ELF tasks. File Manager and launcher opens should know which process
owns which window, close requests should cleanly reach the app and fall back to
termination, and ordinary text files should open in the userspace editor.

- [x] Add runtime app lifecycle records with PID, app name, executable path,
      window title/handle, and recent exit status.
- [x] Hook lifecycle start into ELF spawns and `sys_exec`, then refine display
      names when launcher-owned GUI apps open their compositor windows.
- [x] Hook normal exits, kills, and userspace faults into lifecycle completion,
      crash reports, notifications, and user-GUI window cleanup.
- [x] Keep the existing clean close event for userspace GUI windows and add a
      timeout path that terminates apps that ignore close requests.
- [x] Route File Manager text/unknown opens through `/bin/editor <path>` with a
      kernel Text Viewer fallback if the userspace editor cannot spawn.
- [x] Teach `/bin/editor` to accept an absolute path argv while keeping
      `/documents/editor.txt` as the default document.
- [x] Add `make smoke-userspace-file-open` to verify editor argv routing and
      deterministic save behavior for a non-default path.

**Current status:** complete. `apps` and Diagnostics now include running and
recently finished userspace app records, shell close actions send GUI close
events before terminating unresponsive apps, and text-file open requests launch
the ring-3 editor against the selected path.

---

## ✅ Phase 24 — App Platform Polish + File Dialogs

**Goal:** Make the Phase 22/23 utility apps feel like managed desktop
applications: document-oriented editor flows, explicit open-with actions,
desktop-visible lifecycle controls, and less flaky typed QEMU utility smokes.

- [x] Add New, Open, Save, and Save As document flow to the shared userspace
      editor implementation used by Notes and Text Editor.
- [x] Keep `/bin/editor <path>` and `/bin/notes <path>` argv loading while
      allowing newly-created untitled buffers to choose a save path before
      writing.
- [x] Add File Manager context-menu Open With Editor and Open With Viewer
      actions, with compositor routing for direct viewer opens.
- [x] Promote app lifecycle data into System Monitor with selected running app,
      executable path, recent exit status, and Close/Kill/Path controls.
- [x] Route System Monitor close/kill/path requests through the compositor so
      close remains polite for userspace GUI windows and kill uses the scheduler
      termination path.
- [x] Add QEMU `fw_cfg` smoke command injection plus bounded retry support in
      `scripts/qemu_smoke.py` so userspace utility smokes no longer depend on
      HMP keyboard delivery through the emulated USB input path.

**Current status:** complete. Notes and Text Editor now have file-dialog-style
path prompts, File Manager can open files explicitly in either editor or viewer,
System Monitor can manage running userspace GUI apps, and the utility smoke
target now launches commands through deterministic QEMU `fw_cfg` injection.

---

## ✅ Phase 25 — Package Platform

**Goal:** Make `/APPS` more than a mirror of built-in metadata. Apps installed
from package manifests should participate in search, launch, file associations,
permissions, and removal just like built-in apps.

- [x] Add a manifest contract with `id`, `name`, `command`, `version`, `icon`,
      `category`, `permission`, `exec`, `aliases`, and `associations`.
- [x] Teach `pkg install <path.pkg>` to validate UTF-8 package manifests,
      reject built-in collisions, require an existing userspace executable, and
      persist the normalized manifest to `/APPS/<command>/APP.CFG`.
- [x] Load persisted `/APPS` manifests into the package database at boot so
      installed package apps survive across non-snapshot disks.
- [x] Add dynamic launcher entries for manifest-installed apps, including alias
      scoring, category filtering, permissions text, and Open Location routing.
- [x] Let installed package manifests contribute file associations; opening a
      matching file routes through the package app instead of falling back to
      the editor.
- [x] Add `pkg run <id-or-command> [args...]` so package apps can be launched
      directly from Terminal as well as through the shell.
- [x] Make uninstall remove dynamic package manifests and hide removed built-in
      packages from launcher/search/open-with paths.
- [x] Add a deterministic `make smoke-package-app` target using QEMU `fw_cfg`
      command queuing to install `/Packages/guidemo.pkg`, launch it, and remove
      it while verifying the userspace GUI still opens.

**Current status:** complete. The generated disk image includes
`/Packages/guidemo.pkg` and `/Documents/package-demo.p25` as Phase 25 fixtures.
The package app installs into `/APPS/pkgdemo/APP.CFG`, exposes aliases and a
`.P25` association, launches `/bin/guidemo`, and uninstalls without leaving a
stale package entry.

---

## ✅ Phase 26 — CoolFS Root Filesystem

**Goal:** Make CoolFS the operating system filesystem instead of a side mount,
while keeping the VFS as the syscall, path, fd, pipe, and device abstraction.

- [x] Promote CoolFS from `/COOL` to the VFS root backend at `/`.
- [x] Keep FAT32 available only as the compatibility/container mount at `/FAT`.
- [x] Grow CoolFS to 4 KiB blocks, 512 inodes, a 4 MiB root image, and
      direct-plus-indirect block addressing so userspace ELF binaries fit.
- [x] Cache the mounted CoolFS image in memory and persist only the populated
      image span back to `/COOLFS.IMG`.
- [x] Teach the host `fs-image` builder to populate `/COOLFS.IMG` with `/bin`,
      standard OS directories, package fixtures, and document fixtures.
- [x] Route kernel storage services through VFS kernel helpers instead of direct
      FAT32 calls, including config, logs, packages, crash dumps, device nodes,
      notifications, ELF loading, terminal `cd`, and screenshots.
- [x] Update `fsck`, `df`, mount reporting, boot self-tests, and status text so
      CoolFS is reported as the root filesystem and FAT32 is reported as legacy.
- [x] Add smoke coverage for CoolFS root routing and `/FAT` compatibility.

**Current status:** complete. VFS remains in place because it owns file
descriptors, pipes, shared memory, syscalls, and mount routing; CoolFS now owns
the normal persistent namespace under `/`. Phase 27 later moved the backing
store from `/COOLFS.IMG` to the native disk region at LBA 0.

---

## ✅ Phase 27 — Native CoolFS Disk Backend

**Goal:** Remove the FAT-backed `/COOLFS.IMG` transition and make CoolFS the
filesystem stored directly on the OS disk.

- [x] Emit a native CoolFS image at LBA 0 from the host `fs-image` builder.
- [x] Keep a separate FAT32 region at 8 MiB only for optional `/FAT` import and
      compatibility testing.
- [x] Teach the kernel CoolFS mount path to read the superblock, inode table,
      bitmap, and data blocks directly from ATA sectors.
- [x] Replace whole-file `/COOLFS.IMG` persistence with a 64-slot native block
      cache and dirty 4 KiB block writeback.
- [x] Update FAT32 BPB handling so the legacy mount can live at a nonzero disk
      offset while still falling back to older sector-0 FAT images.
- [x] Add Terminal `write` and `rm` commands for deterministic filesystem smoke
      mutation.
- [x] Add `make smoke-coolfs-native` to prove a CoolFS file write survives a
      writable QEMU remount, then prove deletion survives another remount.

**Current status:** complete. CoolFS no longer depends on FAT32 to boot or
persist root filesystem changes. FAT32 remains useful for `/FAT` import coverage,
but the native root path is independent.

---

## ✅ Phase 28 — Users, Permissions, and App Sandboxing

**Goal:** Stop treating filesystem access and package permissions as descriptive
metadata. CoolFS files should have durable ownership and modes, tasks should
carry credentials, and user-facing syscalls should enforce those permissions.

- [x] Store `uid`, `gid`, and Unix-style `rwx` mode bits in the reserved area of
      each CoolFS inode without increasing the inode-table footprint.
- [x] Populate the host-built CoolFS image with root-owned system paths, user-owned
      writable paths (`/TMP`, `/Documents`, `/Pictures`, `/Desktop`, `/Trash`,
      `/Downloads`, `/Packages`), and executable mode bits for `/bin` ELF files.
- [x] Add per-task credentials to the scheduler and include `uid`, `gid`, and
      capability summaries in process status output.
- [x] Enforce CoolFS read/write/execute checks in the VFS path used by terminal,
      GUI apps, and ring-3 filesystem syscalls, while retaining explicit kernel
      VFS helpers for trusted kernel services.
- [x] Enforce execute permission before loading ELF images.
- [x] Convert package manifest permission labels (`desktop`, `filesystem`,
      `network`, `settings`, `diagnostics`, `shell`, etc.) into launch-time task
      capabilities.
- [x] Gate userspace network syscalls behind the `network` capability and GUI /
      screenshot syscalls behind the `desktop` capability.
- [x] Add Terminal `whoami`, `perm`, `chmod`, and `chown` commands.
- [x] Add boot selftests for CoolFS mode enforcement and package grant mapping.
- [x] Add `make smoke-phase28-permissions` for interactive permission inspection,
      chmod denial, restore, hash, and non-executable launch denial.

**Current status:** complete. CoolFS is now the persistent authority for file
ownership and mode bits; VFS/syscalls enforce those bits for user-facing access;
packages launch with bounded capabilities derived from their manifests.

---

## ✅ Phase 29 — Login, Sessions, and Service Supervision

**Goal:** Turn Phase 28's uid/gid/mode enforcement into a real desktop session
model. The OS should know which user is logged in, persist local users, apply
home-directory ownership, and keep service authority separate from desktop
authority.

- [x] Add a CoolFS-backed user database at `/CONFIG/USERS.DB` with hashed
      passwords, roles, homes, uid/gid fields, and enabled/disabled login state.
- [x] Seed default users: disabled `root`, admin `jamie` uid/gid 1000, and
      non-admin `guest` uid 1001.
- [x] Add `/Users`, `/Users/jamie`, and `/Users/guest` to the generated image
      and boot repair path; home directories are owner-only and owned by their
      matching user.
- [x] Replace hardcoded interactive credentials with session-derived
      credentials and make launched packages inherit the active session uid/gid.
- [x] Separate admin authority from package `shell` grants so package manifests
      cannot gain ownership/service control just by asking for shell-like
      capabilities.
- [x] Add Terminal `login`, `su`, `logout`, `passwd`, `id`, `groups`, and
      `umask` commands.
- [x] Apply the active session umask to newly created user files and
      directories.
- [x] Require admin credentials for package install/remove and service
      supervisor mutations.
- [x] Add per-service credentials, service status output, deterministic
      supervisor ticks, and restart accounting for failed services.
- [x] Add boot selftests for session ownership, umask enforcement, package
      grant non-admin behavior, and service supervisor restart behavior.
- [x] Add `make smoke-phase29-sessions` to prove session switching,
      cross-user/admin denial, admin chown recovery, and service restart status.

**Current status:** complete. coolOS now boots into a real local session backed
by a persistent user database, creates user homes with durable CoolFS ownership,
applies session credentials to shell and package-launched tasks, and supervises
kernel services under dedicated service credentials.

---

## Technical notes

### The ordering is non-negotiable

Phase 8 (scheduler) is the hardest gate. Every phase from 9 onwards requires
multiple concurrent execution contexts. Don't skip it or fake it with cooperative
yielding — preemption is what makes the OS real.

### Rust in userspace

Userspace binaries are written in `#![no_std]` Rust and link against
`userspace/libcool`. The SDK owns `_start`, panic aborts, initial argv parsing,
raw syscall assembly, and convenience APIs such as `println!`, `File::open`,
`pipe`, `mmap`, `shmem_create`, `read_event`, `dns_resolve`, TCP sockets, and
filesystem utility calls plus GUI windows through `libcool::fs` and
`libcool::gui`.

### Real hardware vs QEMU

Phase 6 (VBE framebuffer) and Phase 14 (USB) are the two gates to booting on
real machines. Everything in between can be developed entirely in QEMU.

### Versioning

| Tag | Milestone |
| :-- | :-------- |
| v1.14 | Phase 13 complete: pipes, shared memory, IPC, userspace terminal |
| v1.16 | Phase 16 — desktop shell, resize handles, start menu |
| v2.0 | Phase 16 complete: scrollbars wired, IntelliMouse scroll wheel |
| v3.0 | Phase 9 complete — first userspace process |
| v4.0 | Phase 12 complete — ELF binaries load from disk |
| v5.0 | Phase 15 complete: network-capable |
| v5.1 | Phase 17 complete: native plain-HTTP browser foundation |
| v5.2 | Phase 18 complete: verified HTTPS/TLS foundation |
| v5.3 | Phase 19 complete: browser rendering and trust hardening |
| v5.4 | Phase 20 complete: userspace SDK foundation |
| v5.5 | Phase 21 complete: userspace GUI runtime |
| v5.6 | Phase 22 complete: userspace utility suite |
| v5.7 | Phase 23 complete: app lifecycle and file-open plumbing |
| v5.8 | Phase 24 complete: app platform polish and file dialogs |
| v5.9 | Phase 25 complete: package platform |
| v6.0 | Phase 26 complete: CoolFS root filesystem |
| v6.1 | Phase 27 complete: native CoolFS disk backend |
| v6.2 | Phase 28 complete: users, permissions, and app sandboxing |
| v6.3 | Current — Phase 29 complete: login, sessions, and service supervision |
