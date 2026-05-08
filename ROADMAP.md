# coolOS Roadmap

The goal is to evolve coolOS from a kernel-mode GUI demo into a real desktop
operating system — one that can load and run user programs, manage storage, and
support multiple processes without any one of them being able to crash the machine.

Phases 1–74 are complete. The current milestone gives coolOS a much more
normal command-line and platform layer: cwd-aware userspace syscalls, shell
quoting/redirection/pipelines, writable file descriptors with durable close
commit, metadata and rename APIs, persistent sysreports under `/LOGS`, an
in-image `/SDK` with devkit templates, ABI v9 evented readiness waits, and ABI
v10 TTY control for raw terminal-mode programs. The native browser now has a
bounded HTML/CSS rendering foundation with CSS selector/cascade support,
CSS-styled line boxes, better image metadata/sizing, form submit URL handling,
DOM-event hit-box fixture coverage, and DOM-backed form controls with live
editing, reset handling, real URL-encoded POST request bodies, persistent
cookie/session state, CSS2 box-model layout, positioned/floating boxes, z-index
paint order, improved table/list layout, and parser repair for common implied
HTML closes plus external stylesheet/image/script subresource loading with
bounded cache metadata, a small JavaScript/DOM mutation runtime, and bounded
web-app APIs for storage, cookies, location/history, attributes/classes/styles,
same-origin fetch callbacks, plus a modern-page compatibility foundation that
keeps raw script bundles out of rendered text and provides a bounded
Google/Search compatibility shell. Phase 62 adds kernel resource limits for
task creation, user address spaces, mmap calls, file descriptors, shared
memory, and sockets, with cleanup on task exit/fault and resource-limit
diagnostics in Terminal, Sysreport, and the Diagnostics viewer. Phase 63 adds
memory-pressure recovery: heap pressure states, allocation admission reserves,
reclaimable CoolFS/Browser cache trimming, per-task memory estimates, and an
OOM path that reclaims the largest non-current user task when pressure remains
critical. Phase 64 makes service supervision durable with persisted desired
state under `/CONFIG`, restart history under `/LOGS`, dependency metadata,
restart backoff, admin-gated controls, and degraded-service diagnostics in
Terminal, recovery, sysreport, Diagnostics, and System Monitor.
Phase 65 adds a service-aware staged update and rollback path: update manifests
live under `/UPDATES/STAGED`, pre-apply file snapshots live under
`/UPDATES/SNAPSHOTS/LAST`, `/LOGS/UPDATE.TXT` records stage/apply/rollback
events, and `update rollback` plus `recovery rollback` can restore the previous
file state. Phase 66 adds boot-health validation on top of that update path:
`/BOOT/STATE.TXT` tracks pending update ids and validation attempts,
`/BOOT/LAST-GOOD.TXT` records the last desktop-ready boot, and a pending update
that fails to reach the healthy checkpoint is automatically rolled back from the
Phase 65 snapshot on the next boot. Phase 67 adds signed staged-update trust:
staged manifests now include per-payload SHA-256 hashes, `/UPDATES/STAGED`
carries a keyed `UPDATE.SIG`, `/CONFIG/UPDATE-KEYS.TXT` exposes trusted update
key metadata, `update verify|keys|sign` make the trust path inspectable, and
apply refuses unsigned or tampered payloads before taking snapshots or stopping
services. Phase 68 replaces the local HMAC trust foundation with Ed25519
public-key verification, multiple built-in trust keys, rotation metadata,
revoked/expired key refusal, versioned manifests, and anti-rollback checks for
older signed updates. Phase 69 extends public-key trust to installable packages:
package archives now have detached Ed25519 sidecar signatures, package trust
metadata under `/CONFIG/PACKAGE-KEYS.TXT`, source/owner records under `/APPS`,
dependency and downgrade checks, package repair, and package trust diagnostics
in Terminal, Recovery, Diagnostics, and Sysreport. Phase 70 makes package
installs copy real payload files transactionally: manifests can declare
payload target/source/hash/mode entries, installs verify and write those
payloads, owner records pin the installed payload table, repair restores payload
files, remove deletes owned payloads, and `/LOGS/PACKAGE-TXN.TXT` records clean
or rolled-back install/repair/remove state. Phase 71 pivots modern browser work
from incremental native-parser growth to a real engine port track: WPE WebKit is
the selected target, the native browser remains the fallback/debug renderer,
`src/browser_engine.rs` exposes a browser engine port ABI/readiness model,
`/CONFIG/BROWSER-ENGINE.CFG` records preferred/fallback engine policy,
`/SDK/BROWSER_ENGINE_PORT.TXT` documents the host contract and blockers, and
Terminal, Browser, Recovery, Diagnostics, and Sysreport expose the WPE readiness
surface.
Phase 72 adds the first userspace threading and futex substrate for hosted
browser runtimes: ABI v11 exposes `thread_spawn`, `futex_wait`, and
`futex_wake`; the scheduler can create same-address-space ring-3 threads with
private user/kernel stacks; shared PML4s are freed only after the last sibling
is reaped; `libcool::thread` wraps spawn/join/futex operations; `/bin/threaddemo`
tests two worker threads plus futex wake/join; and diagnostics/sysreport expose
futex counters plus thread-stack capacity. Phase 73 adds per-thread FS-base TLS
state, ABI v12 `thread_tls_set`, `thread_tls_get`, and `thread_spawn_tls`,
libcool TLS blocks/keys plus pthread-style mutex/condvar/once helpers,
`/bin/tlsdemo`, and browser-engine readiness that marks `threads-futex` ready
while identifying hosted libc/POSIX pthread wiring as the next runtime layer.
Phase 74 adds the first POSIX-shaped pthread/libc shim layer in
`libcool::posix` and `libcool::libc`: pthread create/join/exit/self,
mutex/condvar/once/key APIs, per-thread `errno`, `gettid`, `sched_yield`,
`nanosleep`, `/bin/pthreaddemo`, and smoke coverage over the ABI v12
thread/TLS/futex substrate. Phase 75 adds ABI v13 `mprotect`, `/lib`
shared-object image placement, `libcool::dynlink`, `/bin/lddemo`, ET_DYN
`PT_DYNAMIC` parsing, RELA relocations, dynsym export lookup, init-array
execution, and W^X executable text transitions. Phases 45-75 focus on responsiveness,
interactive terminal behavior, and
desktop-browser compatibility:
cursor-only framebuffer updates,
input-first idle-loop ordering, adaptive 36/144 Hz frame pacing, compositor
telemetry, and `poll`-driven userspace waits for pipes, TTY stdin, sockets,
GUI events, and child exits, plus raw TTY input, ANSI-rendered TUI output,
keyboard-editable Browser controls, and a richer native Browser rendering
surface with GET/POST form submission, persistent cookie/storage state, and
bounded margin/padding/border/position/float layout plus a small Browser
subresource cache, script runtime, web-app API layer, main-response
content-type routing, compatibility diagnostics, resource accounting for the
scheduler, VMM, VFS, shared memory, and sockets, low-memory recovery, durable
service recovery, update rollback, boot-health rollback, signed update
verification, update key rotation, downgrade refusal, signed package
install/repair trust, package payload transactions, and the first explicit
WPE WebKit port ABI plus thread/futex/TLS/POSIX pthread runtime prerequisites
for a future full browser engine.

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
| 8 | Preemptive scheduler — naked timer ISR, round-robin context switching, 288 Hz PIT |
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
- PIT reprogrammed during Phase 8 and now runs at 288 Hz
  (`TIMER_HZ` in `interrupts.rs`) so the scheduler and compositor get frequent
  ticks without relying on the BIOS-era ~18 Hz default.

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
- `src/apps/browser.rs` remains a native no_std GUI app. Phases 49-61 give it a
  bounded browser-engine layer for HTML/CSS line boxes, images, forms, and
  DOM-backed document controls, persistent cookie/session state, CSS2 box-model
  layout, positioning/floats, z-index paint order, table/list improvements, and
  implied-close parser repair plus external stylesheet/image subresources and
  cache metadata plus a small bounded JavaScript/DOM runtime and web-app API
  layer, content-type-aware main-resource handling, and a Google/Search
  compatibility profile.
- `/bin/wget` now sends an HTTP/1.1 request with a coolOS user agent, keeping it as a
  raw userspace socket demo.
- Phases 59-61 add the small JavaScript/DOM runtime planned after the
  subresource layer: same-origin script loading, event dispatch hooks, bounded
  timers, DOM mutation, storage/cookie APIs, location/history hooks, and
  same-origin fetch enough for simple interactive pages.

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
- [x] Populate the host-built CoolFS image with root-owned system paths, a
      shared-writable `/TMP`, user-facing paths (`/Documents`, `/Pictures`,
      `/Desktop`, `/Trash`, `/Downloads`, `/Packages`), and executable mode
      bits for `/bin` ELF files.
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
- [x] Seed default users: admin `root` uid/gid 1000 and non-admin `guest`
      uid 1001.
- [x] Add `/Users`, `/Users/root`, and `/Users/guest` to the generated image
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

## ✅ Phase 30 — GUI Login and Lock Screen

**Goal:** Put a real graphical gate in front of the desktop session. The shell
should boot to a greeter, authenticate with the existing user database, block
desktop input while locked, and expose lock/logout flows through both GUI shell
chrome and the Terminal.

- [x] Boot the compositor in a locked state and render a boot-splash-style
      coolOS greeter with account list, username/password fields, masked
      password entry, status messages, and splash lockup treatment.
- [x] Route keyboard and mouse input to the greeter while locked; normal window,
      taskbar, launcher, desktop-icon, scroll, and shortcut input is suppressed
      until authentication succeeds.
- [x] Authenticate the greeter through `security::login`, so GUI sign-in uses
      the same `/CONFIG/USERS.DB` records and password hashes as Terminal
      `login`.
- [x] Add shell lock/logout actions: Start menu/session launcher entries call
      the lock gate directly, Terminal `lock` requests the greeter, and Terminal
      `logout` returns the session to guest before locking.
- [x] Keep QEMU smoke automation deterministic by auto-signing into the greeter
      for desktop-interaction smokes while still providing `make
      smoke-login-screen` and `make smoke-lock-screen` coverage for the locked
      login UI.

**Current status:** complete. coolOS now has a real GUI login path instead of
only Terminal-driven session switching; the desktop is hidden behind the
splash-style greeter at boot and after lock/logout, while existing session
permissions remain the source of truth.

---

## ✅ Phase 31 — First-Run Setup and Account Management

**Goal:** Finish the local-account story so the shipped default admin is only a
handoff state. The OS should be able to create the first real admin account,
manage users from both GUI and Terminal surfaces, prevent admin lockout, and
prove that those records survive CoolFS remounts.

- [x] Detect first-run state when the only enabled admin is the default
      `root/cool` handoff.
- [x] Add Terminal `setup <user> <pass>` to create or convert the first real
      admin account and disable the default handoff when a replacement admin is
      created.
- [x] Add Terminal `account` subcommands for `list`, `add`, `enable`,
      `disable`, `role`, `pass`, and `delete`.
- [x] Enforce account safety rules: stronger new-password minimums, protected
      built-in names, no deletion/disable of the active session, and no action
      that would remove the last enabled admin.
- [x] Add repeated-login-failure throttling shared by Terminal and GUI login.
- [x] Add an Accounts settings panel, launcher metadata, start-menu pinning,
      and disk-image app manifest entries for account administration.
- [x] Extend boot selftests with an account-management roundtrip covering
      create, disable/enable, role change, password reset, login, and delete.
- [x] Add `make smoke-phase31-accounts` with account lifecycle and writable
      CoolFS persistence coverage for first-run setup.

**Current status:** complete. coolOS now treats `root/cool` as a first-run
handoff, exposes persistent account management through both Terminal and GUI,
and verifies account lifecycle plus first-run persistence in QEMU smoke tests.

---

## ✅ Phase 32 — User/Kernel Isolation Hardening

**Goal:** Close the biggest remaining userspace isolation hole. Ring-3 code
should run from user-owned ELF mappings, kernel pages should stay supervisor-only,
and malicious userspace pointers should fail without crashing the kernel.

- [x] Stop marking the boot page table user-accessible; recursively clear U/S on
      boot mappings before userspace starts.
- [x] Build process PML4s with kernel mappings present but supervisor-only, while
      reserving explicit user roots for ELF/stack/mmap and shared memory.
- [x] Replace kernel-text boot sentinels with `/bin/sentinel`, a normal userspace
      ELF that preserves the existing `sentinel ok` smoke output.
- [x] Remove broad lazy lower-half page-fault allocation. User faults now terminate
      the task and produce crashdump context instead of mapping arbitrary pages.
- [x] Restrict `mmap` to a dedicated userspace arena and mark stacks, mmap pages,
      shared memory, and non-executable ELF segments NX.
- [x] Track process-owned page-table and leaf frames, reclaim them on `waitpid`,
      zombie reap, and successful `exec`, and release shared-memory references
      when tasks exit or exec.
- [x] Add `/bin/badptr`, `/bin/badwrite`, `/bin/badmmap`, `/bin/badexec`, and
      `/bin/baduserread` probes plus `make smoke-phase32-isolation`.
- [x] Add boot selftests proving boot and process PML4s do not expose kernel text
      or heap as user-accessible pages.

**Current status:** complete. coolOS still has a lower-half kernel, but ring-3
tasks can no longer execute or dereference kernel mappings through the copied
process page tables, and QEMU smoke coverage now exercises both denied syscalls
and a real user page fault.

---

## ✅ Phase 33 — Process Control and Jobs

**Goal:** Make process control a real kernel/userspace contract instead of a
set of shell-only diagnostics. Ring-3 programs should be able to spawn children,
manage process groups, deliver bounded signals, and observe wait/reap behavior,
while the desktop job model should be able to control long-running processes.

- [x] Add scheduler STOP/CONT state handling alongside TERM/INT/USR1 delivery.
- [x] Enforce permission-aware process control: callers can control themselves,
      their children, same-uid tasks, or any task when running with admin caps.
- [x] Add process-control syscalls: `signal`, `setpgid`, `getpgid`, and
      `signal_group`, and bump the userspace ABI to version 6.
- [x] Extend `libcool` with typed signal and process-group wrappers.
- [x] Add `/bin/procdemo`, a ring-3 proof binary that spawns `/bin/procsleep`,
      moves it into its own process group, sends USR1/STOP/CONT/group TERM, and
      waits for exit code 143.
- [x] Bind Terminal jobs to real processes via `job run`, `job pause`,
      `job resume`, and `job cancel`; process jobs now display pid/state and
      use STOP/CONT/TERM under the hood.
- [x] Fix ATA PIO reads to wait for post-transfer settle, preventing immediate
      nested ELF loads from racing the disk device.
- [x] Add `make smoke-phase33-process-control` with separate QEMU runs for the
      ring-3 process-control ABI and process-bound job controls.

**Current status:** complete. coolOS now exposes process groups and signals
through both Terminal and `libcool`, and the scheduler has a real stopped state
that removes tasks from the run queue until CONT resumes them.

---

## ✅ Phase 34 — TTY Sessions and Foreground Job Control

**Goal:** Make Terminal-launched programs behave like session-owned foreground
jobs instead of detached output producers. Each terminal should own a TTY,
route stdout/stderr from its tasks back to the right window, and control the
foreground process group with familiar shell actions.

- [x] Add a kernel TTY registry with per-terminal output buffers and foreground
      process-group tracking.
- [x] Add a controlling TTY field to scheduler tasks, inherit it across spawn,
      expose attach helpers, and report TTY ownership in process diagnostics.
- [x] Route `write(1|2, ...)` to the current task's controlling TTY, with the
      existing global syscall-output ring retained as the fallback path.
- [x] Make `exec <path>` a foreground terminal job: the shell assigns a process
      group, attaches the TTY before unblocking the task, and delays the prompt
      until the group exits or stops.
- [x] Keep `job run` as background execution while still binding the task to
      the launching terminal's TTY.
- [x] Add `tty`, `fg`, and `bg` terminal commands plus Ctrl+C/Ctrl+Z delivery to
      the foreground process group.
- [x] Add a `tty-routing` boot selftest and `make smoke-phase34-tty-jobs` with
      foreground and background QEMU runs.

**Current status:** complete. coolOS now has enough TTY/session semantics for a
foreground shell prompt, process-bound background jobs, and keyboard-generated
INT/STOP delivery to coexist on the desktop.

---

## ✅ Phase 35 — TTY Stdin and Line Discipline

**Goal:** Make foreground userspace programs able to read real terminal input
instead of relying on kernel-side command buffers or pipe demos.

- [x] Add a canonical TTY input queue with echo, backspace, enter, and EOF
      handling.
- [x] Route `read(0, ...)` through the current task's controlling TTY when one
      is assigned, while keeping VFS reads for files and pipes.
- [x] Forward Terminal keyboard input to the foreground process group instead
      of dropping it while a foreground job is active.
- [x] Add `/bin/ttyread` as a ring-3 proof program and
      `make smoke-phase35-tty-input`.

**Current status:** complete. A foreground ELF process can block in `read(0)`,
receive a typed line from its Terminal, echo through the same TTY, and exit
cleanly.

---

## ✅ Phase 36 — Userspace Shell Foundation

**Goal:** Move interactive command interpretation into a normal ring-3 process
that uses the same stdin/stdout contract as other programs.

- [x] Add ABI v7 `spawn_args(desc)` so userspace can launch child programs with
      argv rather than path-only spawn.
- [x] Add `libcool::process::spawn_args` and expose it through the userspace
      prelude.
- [x] Add `/bin/sh`, a no_std userspace shell that reads stdin, runs builtins,
      launches child processes, and waits for them.
- [x] Add a Terminal `sh` command that starts `/bin/sh` as the foreground job.
- [x] Add `make smoke-phase36-userspace-shell`.

**Current status:** complete. `/bin/sh` is a real foreground userspace process
on the controlling TTY and can run filesystem commands and child processes.

---

## ✅ Phase 37 — Coreutils Command Set

**Goal:** Provide basic external userspace tools so the shell is not just a
collection of builtins.

- [x] Add `/bin/ls`, `/bin/cat`, `/bin/echo`, `/bin/pwd`, `/bin/mkdir`,
      `/bin/touch`, `/bin/rm`, and `/bin/writefile`.
- [x] Teach the disk-image builder to install extra `/bin` ELFs by filename so
      future userspace tools do not need one-off host-side wiring.
- [x] Make `waitpid` block in the kernel, wake parents on child exit/fault/kill,
      and update TSS RSP0 per scheduled task so ring-3 IRQ frames land on the
      owning task's private kernel stack.
- [x] Add `make smoke-phase37-coreutils` to exercise external commands, argv,
      file creation, reading, touching, and removal through `/bin/sh`.

**Current status:** complete. The system now has a useful ring-3 command-line
workflow: a userspace shell invoking external userspace tools over the kernel
VFS and TTY contracts.

---

## ✅ Phase 38 — Utility App Dependability

**Goal:** Make the existing ring-3 utility apps prove their durable side
effects instead of only proving that windows open.

- [x] Make Text Editor smoke mode verify its saved document by reading back from
      CoolFS or confirming the resulting directory entry.
- [x] Make Trash smoke mode verify that the Trash listing is empty after
      permanent deletion.
- [x] Make Screenshot smoke mode verify that `/Pictures/SMOKE.PPM` exists and
      has PPM magic after queuing capture.
- [x] Add `make smoke-phase38-apps` with separate editor, Trash, and Screenshot
      QEMU runs.

**Current status:** complete. Utility app smokes now cover persisted outcomes,
not just GUI launch paths.

---

## ✅ Phase 39 — Recovery and Repair Path

**Goal:** Give coolOS a discoverable in-OS recovery surface that can explain the
boot target, recreate standard directories, and leave a repair report on disk.

- [x] Add `/RECOVERY/README.TXT` and `/RECOVERY/BOOT.CFG` to the generated
      CoolFS image.
- [x] Add a `recovery` kernel module and Terminal command for status,
      `recovery repair`, and `recovery fsck-on-boot on|off`.
- [x] Make `recovery repair` call the filesystem hardening repair path and write
      `/RECOVERY/LAST-REPAIR.TXT`.
- [x] Add `make smoke-phase39-recovery`.

**Current status:** complete. The OS now exposes a recovery report path from the
running desktop and can persist an fsck-on-boot setting for the next boot.

---

## ✅ Phase 40 — Shell, Terminal, and Filesystem Semantics

**Goal:** Make the userspace shell behave less like a demo REPL and more like a
small Unix-style command interpreter.

- [x] Add ABI v8 `chdir(path)`, `getcwd(buf)`, and task-local cwd inheritance.
- [x] Resolve relative filesystem, screenshot, exec, spawn, and directory-list
      syscall paths against the current task's cwd.
- [x] Replace `/bin/sh` parsing with tokenization for quotes, backslash escapes,
      `<`, `>`, and `|`.
- [x] Add shell cwd builtins, `/bin/<command>` lookup for bare command names,
      output/input redirection, and one-stage pipelines.
- [x] Add fd-mapped child launch through `spawn_fds_args(desc)` so shell pipes
      and redirection use kernel file descriptors instead of kernel-side tricks.
- [x] Add `make smoke-phase40-shell-semantics`.

**Current status:** complete. `/bin/sh` now supports cwd-aware file workflows,
quoted arguments, redirected files, and simple command pipelines running as
normal ring-3 child processes.

---

## ✅ Phase 41 — Filesystem Durability and Metadata

**Goal:** Give userspace practical file mutation primitives and prove their
results survive a remount.

- [x] Add ABI v8 `stat(desc)`, `rename(desc)`, `open_write(path)`, `sync()`, and
      RTC `time()`.
- [x] Commit writable file descriptors through CoolFS safe-write on close and
      when a task exits with dirty open files.
- [x] Add `libcool` wrappers for metadata, rename, cwd, sync, time, and file
      creation.
- [x] Add `/bin/cp`, `/bin/mv`, `/bin/stat`, `/bin/sync`, `/bin/date`,
      `/bin/uname`, and `/bin/clear`.
- [x] Add a writable-image smoke that creates a redirected file, syncs it,
      reboots from the same copied disk image, and verifies file contents plus
      metadata.

**Current status:** complete. Shell-created files and renamed/copied files now
flow through real userspace fds and survive across a second QEMU boot.

---

## ✅ Phase 42 — App Consistency and In-OS Help

**Goal:** Keep built-in app text, terminal diagnostics, and shell-facing helper
surfaces aligned with the current runtime instead of stale milestone labels.

- [x] Update Text Viewer welcome/about content to describe the current ring-3,
      shell, recovery, sysreport, and devkit capabilities.
- [x] Add Terminal `devkit` output that reports the active ABI and SDK paths.
- [x] Keep Terminal help/diagnostics surfaces aware of the new sysreport/devkit
      commands.
- [x] Add `make smoke-phase42-app-consistency`.

**Current status:** complete. In-OS help and diagnostics now describe the same
ABI, shell, and devkit surfaces that the boot image actually exposes.

---

## ✅ Phase 43 — Observability and Sysreport

**Goal:** Provide one command that gathers useful system state into a persistent
report file for debugging and support.

- [x] Add `src/sysreport.rs` to gather kernel log, profiler, services, process
      table, wait queues, VFS, writeback, and crash-report sections.
- [x] Add Terminal `sysreport` to print the generated report and
      `sysreport write` to persist `/LOGS/SYSREPORT.TXT`.
- [x] Flush the report through the writeback barrier so it is durable after the
      command returns.
- [x] Add `make smoke-phase43-observability`.

**Current status:** complete. `sysreport write` creates a readable
`/LOGS/SYSREPORT.TXT` that can be inspected immediately from the running OS.

---

## ✅ Phase 44 — Developer Platform and SDK Devkit

**Goal:** Make the generated OS image self-describing enough for future
userspace app development.

- [x] Generate `/SDK/README.TXT`, `/SDK/APP_TEMPLATE.RS`, and
      `/SDK/PACKAGE_TEMPLATE.PKG` in the CoolFS image.
- [x] Add `/bin/devkit` as a userspace ABI/devkit path helper.
- [x] Add Terminal `devkit` as the kernel-side companion view.
- [x] Wire the new devkit binary into the Makefile and disk-image builder.
- [x] Add `make smoke-phase44-devkit`.

**Current status:** complete. The boot image now carries current ABI SDK notes,
starter app/package templates, and both kernel and userspace commands to find
them.

---

## ✅ Phase 45 — Compositor Latency and Smoothness

**Goal:** Make mouse movement and normal desktop frames feel less like a toy by
separating cursor latency from full-scene composition and by keeping background
work behind the input/render path.

- [x] Split repaint requests into explicit full repaint, cursor-only repaint,
      and passive timer frame tick paths.
- [x] Move the timer IRQ from unconditional full repaint requests to passive
      frame ticks paced at 36 Hz for clocks/animations/idle visual updates.
- [x] Keep mouse-only movement out of the full compositor path when no button,
      scroll, drag, modal, menu, or taskbar hover work needs to change the base
      scene.
- [x] Draw the cursor as a hardware overlay after the clean shadow scene is
      blitted, and restore/draw only old/new cursor rectangles on fast-path
      mouse motion.
- [x] Reorder the idle loop to poll USB input and render before service,
      deferred, and network maintenance; reduce the deferred-work budget per
      loop to bound latency spikes.
- [x] Add `compositor`/`smoothness` telemetry for full frames, cursor-fast
      frames, passive frame cadence, damage rows/pixels, and cursor overlay
      pixels.
- [x] Add `make run-smooth` for QEMU tablet input and
      `make smoke-phase45-smoothness`.

**Current status:** complete. Plain pointer movement can now update through a
cursor overlay path instead of forcing a full window recomposition, while normal
events still request full frames when the base scene changes.

---

## ✅ Phase 46 — Adaptive High Refresh

**Goal:** Make coolOS feel closer to a 144 Hz desktop without burning full-frame
work while the machine is idle.

- [x] Keep idle passive full-frame pacing at 36 Hz for clocks, animations, and
      background UI updates.
- [x] Add a 750 ms active boost window that raises full-frame pacing to 144 Hz
      after explicit repaint work or mouse movement.
- [x] Mark the pacing clock when an explicit full frame is composed so the next
      passive timer tick does not immediately duplicate it.
- [x] Preserve the Phase 45 cursor overlay fast path for plain pointer motion,
      while allowing active full frames to refresh hover, menu, drag, resize, and
      app animation state at high refresh during interaction.
- [x] Check delayed startup commands on due/paced frames instead of forcing a
      full compositor pass every idle loop while startup commands are waiting.
- [x] Extend `compositor`/`smoothness` telemetry with pacing mode, target Hz,
      idle/active Hz, boost duration/remaining time, target frame-budget ticks,
      and budget misses.
- [x] Add `make smoke-phase46-adaptive-refresh`.

**Current status:** complete. A 144 Hz monitor can now get high-refresh full
desktop frames during active work, while the idle desktop falls back to the
lower Phase 45 cadence.

---

## ✅ Phase 47 — Evented Userspace Runtime

**Goal:** Replace spin/yield readiness loops with a kernel-backed event wait
surface that lets ring-3 tasks block on the resources they actually care about.

- [x] Add ABI v9 `poll(desc_ptr, count, timeout_ms)` with bounded descriptor
      validation, timeout handling, and lost-wakeup-safe waiter registration.
- [x] Support fd readiness for regular files and pipes, including pipe
      read/write wakeups and HANGUP on closed peers.
- [x] Support current TTY stdin readiness for canonical `read(0)` consumers.
- [x] Support TCP socket read/write/HANGUP readiness from the network stack.
- [x] Support userspace GUI event readiness from compositor event queues.
- [x] Support child-exit readiness so shells and process demos can wait without
      yield loops before calling `waitpid`.
- [x] Add `libcool::evented::{PollDesc, poll, wait_fd_read, wait_socket_read,
      wait_gui_event, wait_child}` and migrate representative userspace apps.
- [x] Add `/bin/polldemo` plus `make smoke-phase47-evented-userspace`.
- [x] Update `/SDK/README.TXT`, in-OS devkit/about text, README, and roadmap
      docs for ABI v9.

**Current status:** complete. `/bin/sh`, the event terminal, GUI utilities,
`/bin/guidemo`, `/bin/wget`, and process demos now use event waits where the
kernel can report readiness. `/bin/polldemo` verifies poll timeout, pipe
readiness, and child-exit readiness under QEMU.

---

## ✅ Phase 48 — Terminal/TUI Platform

**Goal:** Build on evented waits with a real terminal-mode userspace surface:
foreground apps should be able to query terminal geometry, disable canonical
line input, receive single-key raw input, and draw practical ANSI/VT output.

- [x] Add ABI v10 `tty_control(op, arg1, arg2)` for foreground TTY mode and
      size queries.
- [x] Track per-TTY canonical/raw mode, echo, signal delivery, and cell
      geometry in the kernel TTY registry.
- [x] Make TerminalApp update TTY geometry when resized and reset foreground
      jobs back to canonical mode when they stop or exit.
- [x] Forward raw control bytes plus cursor/home/end/page escape sequences to
      foreground apps when canonical mode is disabled.
- [x] Add a practical ANSI/VT renderer subset for SGR colors, cursor
      positioning/movement, screen clearing, line clearing, and cursor
      save/restore.
- [x] Add `libcool::tty::{mode,set_mode,enter_raw_mode,restore_mode,size}`.
- [x] Add `/bin/tuidemo` plus `make smoke-phase48-terminal-tui` to verify raw
      single-key input without Enter and ANSI-rendered status output.
- [x] Update `/SDK/README.TXT`, in-OS devkit/about text, README, and roadmap
      docs for ABI v10.

**Current status:** complete. `/bin/tuidemo` enters raw no-echo/no-signal mode,
uses `poll` on stdin, exits on a single `q`, restores the previous TTY mode, and
prints ANSI-colored/status output through TerminalApp's VT parser.

---

## ✅ Phase 49 — Browser Engine Foundation

**Goal:** Move the Browser from linear HTML extraction to a bounded native
rendering engine that can compute document style before emitting line boxes.

- [x] Parse `<style>` blocks and inline `style=` declarations into bounded CSS
      rules.
- [x] Match tag, class, id, simple compound selectors, grouped selectors, and
      last-part descendant selectors with source-order/specificity handling.
- [x] Carry computed style through the renderer as line-box hints instead of
      losing CSS before layout.
- [x] Add `/TMP/PHASE49.HTML` and `make smoke-phase49-browser-engine`.

**Current status:** complete. The native Browser computes CSS-derived hidden
state, alignment, indentation, colors, backgrounds, whitespace, and image
sizing hints before laying out document items.

---

## ✅ Phase 50 — CSS Layout Pass

**Goal:** Cover the practical CSS2-style layout properties needed for readable
desktop browsing without adding an unsafe or unbounded web engine.

- [x] Apply `display:none`, `visibility:hidden`, `text-align`, `margin-left`,
      `padding-left`, `text-indent`, `color`, `background(-color)`, `width`,
      `height`, and `white-space`.
- [x] Draw styled text/control/image backgrounds inside the Browser document
      pane.
- [x] Preserve CSS alignment in the existing line layout model and keep image
      scaling bounded by document width and maximum preview height.
- [x] Add `/TMP/PHASE50.CSS.HTML` and `make smoke-phase50-css-layout`.

**Current status:** complete. CSS now affects visible layout, not just filtering:
styled fixture pages render centered/right-aligned text, indented blocks,
background colors, hidden content, and CSS-sized images.

---

## ✅ Phase 51 — Browser Forms

**Goal:** Make HTML forms useful in the native Browser surface.

- [x] Render text/search/email-style inputs, checkboxes, radio buttons, select
      boxes, textareas, image buttons, and submit/reset/button controls.
- [x] Preserve checked/default values and hidden fields in form state.
- [x] Build clickable GET submit URLs from form action, field names, values, and
      submit-button values; POST remains visibly non-clickable until the browser
      has request-body submission.
- [x] Add `/TMP/PHASE51.FORM.HTML` and `make smoke-phase51-browser-forms`.

**Current status:** complete. Forms render as native controls in the Browser
document pane, and submit controls route through the existing hit-box/navigation
path with encoded query strings.

---

## ✅ Phase 52 — DOM/Event Foundation

**Goal:** Establish the event routing foundation needed before scripting: links,
forms, and button-like controls must be represented as clickable document
objects with stable hit boxes.

- [x] Keep link/form/button controls in the same layout item stream used for
      hit-box generation.
- [x] Preserve image/control box dimensions so click targets match rendered
      output after scrolling.
- [x] Add Browser debug rendering coverage for CSS style and form URL behavior.
- [x] Add `/TMP/PHASE52.DOM.HTML` and `make smoke-phase52-dom-events`.

**Current status:** complete. The native document model established here now
feeds the bounded script event hooks added in Phase 59.

---

## ✅ Phase 53 — DOM-Backed Browser Forms

**Goal:** Move Browser form interaction from static rendered labels to live
document state that can survive reflow and feed real submissions.

- [x] Build a bounded DOM tree alongside the rendered document, preserving
      element/text nodes, parent/child links, and a compact attribute set for
      later scripting/runtime work.
- [x] Track forms and controls as live document state: text/search/email-style
      inputs, checkboxes, radios, selects, textareas, submit/image buttons,
      resets, hidden values, disabled controls, default values, selected options,
      and form method/action metadata.
- [x] Bind rendered controls and table-flattened form controls back to stable
      control ids so click hit boxes activate live state instead of static URLs.
- [x] Add keyboard focus traversal/editing: Tab cycles controls, text fields and
      textareas edit values, checkboxes/radios/selects update in place, Escape
      clears focus, and Enter submits the focused form through its default
      submitter when available.
- [x] Rebuild GET submit URLs from current live control values and show staged
      POST request targets/bodies until the network layer supports request-body
      submission.
- [x] Add `/TMP/PHASE53.DOM.HTML`, DOM/form debug selftest coverage, and
      `make smoke-phase53-dom-forms`.

**Current status:** complete. Forms now have the state and event path that later
phases use for real POST dispatch and Phase 59 script-driven interaction.

---

## ✅ Phase 54 — Browser POST Submission

**Goal:** Turn DOM-backed POST forms from staged request previews into real
network submissions through the shared Browser loader.

- [x] Add a bounded HTTP request builder that can emit both GET and POST
      requests with `Content-Type`, byte-accurate `Content-Length`, and
      `application/x-www-form-urlencoded` bodies.
- [x] Route POST requests through the existing HTTP and HTTPS exchange paths so
      response normalization, TLS verification, image/HTML handling, status
      reporting, history, and downloads stay consistent with normal page loads.
- [x] Preserve browser redirect behavior: 307/308 keep the original method and
      body, while 301/302/303 convert submitted POSTs to GET requests for the
      redirected location.
- [x] Replace the Phase 53 staged POST page with real Browser submission logic
      while keeping unsupported non-web POST targets explicit in the document
      pane.
- [x] Add `/TMP/PHASE54.POST.HTML`, HTTP request-construction selftest coverage,
      Terminal `browser [url]` launch coverage, and `make
      smoke-phase54-browser-post` with a keyboard-submitted HTTPS POST.

**Current status:** complete. Browser forms can now send live DOM values as real
URL-encoded POST request bodies; later phases add richer cache state and a
bounded JavaScript runtime.

---

## ✅ Phase 55 — Browser Session State

**Goal:** Give the native Browser a persistent session foundation so page loads,
redirects, images, and form submissions can share cookie state like a desktop
browser.

- [x] Add a bounded persistent cookie jar at `/CONFIG/BROWSER.COOKIES` with
      safe serialization, corruption recovery through the config store, and
      redacted display output.
- [x] Parse `Set-Cookie` headers for name/value, Domain, Path, Secure, and
      Max-Age deletion while rejecting oversized, malformed, cross-domain, and
      insecure Secure-cookie inputs.
- [x] Split Browser network entry points from the Terminal/web API path so
      Browser GET and POST requests recompute matching cookies per request and
      redirect, while terminal/network diagnostics remain stateless.
- [x] Route HTML-sourced inline PNG fetches through the Browser session path so
      authenticated image loads can share cookie state with their page.
- [x] Add `browser://session` plus a home-page link so users can inspect
      cookie counts, storage location, scope, path, and Secure flags without
      exposing cookie values.
- [x] Extend kernel selftests with deterministic cookie matching/deletion and
      Cookie-header request construction checks, add `/TMP/PHASE55.SESSION.HTML`,
      and add `make smoke-phase55-browser-session`.

**Current status:** complete. The Browser now persists cookies across page
loads, applies scope/path/secure matching when building requests, and exposes a
redacted session-state page. Later phases add HTTP cache behavior, deeper
layout/runtime fidelity, and bounded JavaScript execution.

---

## ✅ Phase 56 — CSS2 Box Model and Reflow

**Goal:** Move Browser layout beyond styled text rows by adding bounded CSS2 box
metrics that affect wrapping, painting, and hit testing.

- [x] Extend the CSS cascade with `width`, `max-width`, `height`,
      margin/padding shorthands and edges, border width/color/style, and
      percentage width parsing.
- [x] Carry box metrics through `BrowserLineStyle` so rendered text, controls,
      and images keep their content box separate from painted border/background
      boxes.
- [x] Wrap text inside fixed and percentage-width content boxes instead of only
      the viewport width.
- [x] Update the Browser layout pass to compute content rects, painted box
      rects, margins, padding, borders, and bounded heights, then reuse those
      box rects for link/control hit testing.
- [x] Paint CSS backgrounds and borders from layout boxes while preserving
      existing text/image/control rendering and resize-triggered reflow.
- [x] Add `/TMP/PHASE56.BOX.HTML`, box-style/layout selftest checks, and
      `make smoke-phase56-css-box-model`.

**Current status:** complete. The Browser now handles the practical CSS2 box
model needed for readable desktop pages: bounded margin/padding/border boxes,
background painting, percentage-width reflow, and box-based hit testing. Deeper
positioning, floats, parser recovery, and table/list fidelity moved into Phase
57.

---

## ✅ Phase 57 — Browser Layout and Parser Fidelity

**Goal:** Make the Browser layout engine handle the next set of common desktop
page constructs: positioned boxes, floats, z-ordering, stronger tables/lists,
and malformed-but-common HTML structure.

- [x] Extend the CSS cascade with `position`, `top/right/bottom/left`, `float`,
      `z-index`, and `list-style` / `list-style-type` parsing.
- [x] Carry positioning, float, z-index, and list marker metadata through
      `BrowserLineStyle`, style debug output, and box layout debug output.
- [x] Update layout placement so relative/sticky boxes offset visually while
      staying in normal flow, absolute/fixed boxes can anchor by offsets, and
      z-index/source order controls paint and hit-test priority.
- [x] Add bounded left/right float handling that reserves horizontal flow space
      for following text while keeping painted float boxes in the document.
- [x] Improve table/list fidelity with CSS square/circle/decimal/none list
      markers and content-aware table column sizing.
- [x] Repair common HTML implied closes for paragraphs, list items, table rows,
      and table cells in both renderer flow and DOM/control scanning.
- [x] Add `/TMP/PHASE57.LAYOUT.HTML`, parser/layout selftest checks, and
      `make smoke-phase57-browser-layout`.

**Current status:** complete. The Browser now has a more resilient layout/parser
layer for ordinary desktop web pages: positioned and floating boxes, z-ordered
painting and hit testing, better list/table output, and parser recovery for
common omitted closing tags. External CSS/image subresources and cache semantics
moved into Phase 58, followed by bounded JavaScript execution in Phase 59.

---

## ✅ Phase 58 — Browser Subresources and Cache

**Goal:** Let the Browser render common page dependencies instead of only the
main HTML document: external stylesheets, HTML image resources, and visible
cache state.

- [x] Discover bounded `<link rel="stylesheet">` resources, resolve them
      against `<base>` / page URLs, and load local, HTTP, or HTTPS CSS through
      the shared Browser loader path.
- [x] Feed loaded external CSS into the existing selector/cascade engine before
      rendering while preserving inline style and inline `<style>` behavior.
- [x] Route HTML-sourced images through a bounded subresource path that decodes
      PNG previews and keeps JPEG/GIF/WebP as dimension-aware placeholders.
- [x] Add an in-memory subresource cache with URL, kind, content type, size,
      age, last-use, and hit-count metadata.
- [x] Add `browser://cache`, cache hit/miss status text, normal reload reuse,
      and uppercase `R` hard reload behavior for subresources.
- [x] Add `/TMP/PHASE58.SUBRESOURCES.HTML`, `/TMP/PHASE58.CSS`, subresource
      debug selftest coverage, and `make smoke-phase58-browser-subresources`.

**Current status:** complete. The Browser now has the first practical
subresource layer for desktop pages: external CSS affects layout, inline PNGs
reuse cached bytes across reflow/reload, non-PNG images keep useful metadata
placeholders, and the cache can be inspected from inside the OS. The script
loading policy and DOM mutation runtime are now covered by Phase 59.

---

## ✅ Phase 59 — Browser JavaScript and DOM Runtime

**Goal:** Let simple interactive desktop pages run bounded scripts without
pretending coolOS has a full modern JS engine yet.

- [x] Discover bounded inline `<script>` blocks and same-origin local/HTTP/HTTPS
      external scripts, load external JavaScript through the Browser subresource
      cache, and reject unsupported cross-origin or oversized scripts.
- [x] Add a small statement runtime for practical DOM operations:
      `document.getElementById(...)`, `document.querySelector(...)`,
      `textContent` / `innerText`, `className`, form `value`, `checked`, and
      `disabled` mutations.
- [x] Bind script mutations back into the existing DOM/form state, serialize the
      mutated DOM before reflow, and keep live form submissions using script-set
      values.
- [x] Support inline `onclick` / `onchange` / `onsubmit` handlers plus
      `addEventListener("click" | "change" | "submit", function(){...})` on
      bounded DOM targets.
- [x] Add bounded `setTimeout(function(){...}, ms)` execution for simple delayed
      page initialization while keeping recursion and statement limits explicit.
- [x] Add `browser://js`, JS status text, script debug selftest coverage,
      `/TMP/PHASE59.JS.HTML`, `/TMP/PHASE59.JS`, and
      `make smoke-phase59-browser-js`.

**Current status:** complete. The Browser now has the first useful script layer:
simple pages can initialize text, classes, and form values, respond to button
clicks, and inspect script runtime counts from inside coolOS. Phase 60 builds
on this bounded compatibility layer with storage, cookie, location/history, and
same-origin fetch APIs; broader ECMAScript semantics and full async networking
remain future browser engine work.

---

## ✅ Phase 60 — Browser Web-App APIs

**Goal:** Let small HTML5-style web apps persist state, inspect browser context,
and fetch same-origin text resources inside the native Browser.

- [x] Add persistent per-origin `localStorage` backed by
      `/CONFIG/BROWSER.STORAGE`, with bounded entry/key/value counts and
      corruption recovery through the config-store path.
- [x] Add per-document `sessionStorage` for script state that survives reflow
      within the loaded document but does not persist across pages.
- [x] Expose JS `document.cookie` reads/writes through the existing persistent
      Browser cookie jar while preserving the same Domain/Path/Secure policy
      used by Browser network requests.
- [x] Add bounded `location.href`, `location.search`, `location.assign`,
      `location.replace`, `history.pushState`, and `history.replaceState`
      hooks so scripts can inspect or request navigation state.
- [x] Expand practical DOM APIs with `querySelectorAll(...)[index]`,
      `classList.add/remove/toggle`, `setAttribute`, `getAttribute`,
      `removeAttribute`, and `style.<property>` mutations that serialize back
      into the DOM before reflow.
- [x] Add simple script variables plus same-origin `fetch()` text callbacks for
      local, HTTP, and HTTPS resources, including bounded POST body support for
      web targets.
- [x] Add `browser://storage`, expanded `browser://js` API counters,
      `/TMP/PHASE60.WEBAPP.HTML`, `/TMP/PHASE60.DATA`, selftest coverage, and
      `make smoke-phase60-browser-webapi`.

**Current status:** complete. The Browser can now run small stateful pages:
scripts can persist localStorage, use sessionStorage, read/write cookies, adjust
classes/attributes/styles, inspect `location.search`, update history state, and
consume same-origin text resources through a bounded fetch callback. This is
still intentionally not a full browser VM: Promise scheduling, service workers,
WebSockets, IndexedDB, canvas/media, and complete HTML5 API coverage remain
future work.

---

## ✅ Phase 61 — Browser Modern-Page Compatibility

**Goal:** Stop modern script-heavy pages from rendering JavaScript as page text,
make main-resource type handling explicit, and provide a usable Google/Search
compatibility surface while the native engine continues to grow.

- [x] Harden raw-element parsing for `<head>`, `<script>`, `<style>`,
      `<noscript>`, `<template>`, SVG/canvas/media/embed/object blocks, and
      iframe content by jumping directly to the matching closing tag before
      layout or DOM text serialization.
- [x] Add main-response content-type routing: HTML enters the renderer, images
      stay on the preview/metadata path, and JavaScript/CSS/JSON/other non-HTML
      resources show bounded source/resource diagnostics instead of being
      interpreted as markup.
- [x] Add a Google/Search compatibility profile for `google.*` home/search
      pages. It detects the script-heavy Google shell, replaces it with a small
      native search form, and submits real GET searches to
      `https://www.google.com/search`.
- [x] Add `browser://compat` to explain whether the last page used native
      rendering, source diagnostics, or the Google compatibility shell.
- [x] Add `/TMP/PHASE61.GOOGLE.HTML`, selftest coverage for Closure-script
      suppression and the Google compatibility shell, plus
      `make smoke-phase61-browser-compat`.

**Current status:** complete. `https://www.google.com/` no longer exposes raw
Closure JavaScript as the rendered page body. It renders a bounded native
Google search shell in coolOS and keeps the status bar honest with
`compat=google-search`. This is not a full modern browser engine yet: complete
CSSOM/layout, full ECMAScript, Promise/event-loop semantics, canvas/media, web
components, accessibility tree parity, and multi-process site isolation remain
future work.

---

## ✅ Phase 62 — Kernel Resource Limits and Cleanup

**Goal:** Keep core OS resources bounded so task floods, runaway mappings,
descriptor churn, shared-memory growth, and socket leaks fail cleanly instead
of exhausting the kernel heap.

- [x] Add central resource-limit constants and diagnostics for active tasks,
      user address-space pages, per-call mmap bytes, per-task fd slots,
      shared-memory region/task bytes, and socket ownership/global counts.
- [x] Make task creation allocation-aware: user task spawn checks the active
      task cap, preflights scheduler/task-stack allocation, and reports the cap
      through `info`, System Monitor, Sysreport, Terminal diagnostics, and the
      Diagnostics viewer.
- [x] Bound user memory growth by rejecting over-large `mmap` calls and refusing
      mappings that would exceed the per-task owned-page ceiling.
- [x] Preflight VFS fd allocation before opening files, duplicating fds, or
      creating pipes; enforce shared-memory region and per-task shared-memory
      quotas before allocating or mapping frames.
- [x] Enforce per-task and global socket limits, and close sockets owned by a
      task when it exits, is killed, or faults.
- [x] Fold resource-limit invariants into the kernel selftest path and add
      `make smoke-phase62-resource-limits` to boot Diagnostics, mirror the
      resource-limit report, and verify the bounded-resource telemetry.

**Current status:** complete. coolOS now has explicit kernel-side ceilings for
the resources most likely to turn a desktop OS into an unbounded heap consumer:
tasks, address-space pages, mmap calls, descriptors, shared memory, and sockets.
The limits are intentionally simple and conservative until the kernel grows
per-user accounting and paging; denial paths fail the syscall/spawn request
without tearing down the machine.

---

## ✅ Phase 63 — Memory Pressure and OOM Recovery

**Goal:** Convert the Phase 62 ceilings into runtime pressure behavior: report
when the heap is getting tight, trim reclaimable caches, admit large allocations
against a reserve, and reclaim a user task if pressure stays critical.

- [x] Add allocator heap snapshots and a memory-pressure module with
      normal/low/critical states, low/critical thresholds, admission reserve
      checks, reclaim counters, OOM kill counters, and selftest coverage.
- [x] Add per-task memory estimates covering owned user pages, shared-memory
      pages, kernel stack bytes, fd count, socket count, and total estimated
      bytes; expose them through the `memory` command, Diagnostics, and
      Sysreport.
- [x] Add pressure-triggered reclaim hooks for clean CoolFS cache blocks and
      Browser page/subresource/image caches, with reclaimed-byte counters.
- [x] Gate large allocations for task stacks, Browser subresource cache writes,
      VFS file opens, and VFS pipes through memory-pressure admission checks.
- [x] Add an OOM reclaim path that chooses the largest non-current user task,
      closes its VFS/socket/GUI resources, frees its kernel stack and address
      space immediately, records crash/profile/app-lifecycle state, and wakes
      waiters.
- [x] Surface heap pressure and OOM count in System Monitor, add
      `make smoke-phase63-memory-pressure`, and document v7.27.

**Current status:** complete. Memory pressure is now a first-class kernel
condition instead of only a post-failure heap number. The current policy is
deliberately conservative: trim cheap caches first, preserve a heap reserve for
large allocations, and only reclaim a user task when the heap remains critical.

---

## ✅ Phase 64 — Persistent Service Supervision and Recovery

**Goal:** Turn the kernel service supervisor from a volatile restart loop into a
durable operational subsystem with persisted desired state, dependency-aware
recovery, restart history, and degraded-state diagnostics.

- [x] Persist supervised service desired state to `/CONFIG/SERVICES.CFG` and
      reload it during boot after the root filesystem is online.
- [x] Write restart/history snapshots to `/LOGS/SERVICES.TXT`, including
      restart policy, dependencies, failure counters, backoff, and last action.
- [x] Add static dependency metadata for core services and gate supervisor
      loops/restarts on dependency readiness.
- [x] Add restart backoff for repeated failures while keeping first-failure
      recovery immediate for deterministic smoke coverage and manual repair.
- [x] Expand Terminal `services` with readable
      `list`, `status`, `history`, and `recovery` diagnostics plus admin-gated
      `run`, `start`, `restart`, `stop`, and `fail` operations.
- [x] Surface service recovery in `/RECOVERY` status/repair reports, sysreport,
      Diagnostics/Log Viewer, and System Monitor service health.
- [x] Add `make smoke-phase64-services` and document v7.28.

**Current status:** complete. coolOS now records service policy and recovery
state across boots, exposes failed/degraded services in the same operational
surfaces as filesystem and memory pressure, and keeps service mutations behind
admin credentials.

---

## ✅ Phase 65 — System Update, Snapshot, and Rollback

**Goal:** Give coolOS a safe local update workflow so system files can be staged,
applied, audited, and rolled back without relying on ad hoc manual file edits.

- [x] Add `src/updates.rs` with `/UPDATES`, `/UPDATES/STAGED`,
      `/UPDATES/SNAPSHOTS/LAST`, `/UPDATES/APPLIED.MF`, and
      `/LOGS/UPDATE.TXT` layout management.
- [x] Add staged update manifests at `/UPDATES/STAGED/UPDATE.MF` with payload
      paths, target paths, version/id metadata, and affected service lists.
- [x] Capture rollback snapshots before applying updates and write
      `/UPDATES/SNAPSHOTS/LAST/MANIFEST.TXT` with target/snapshot/missing-file
      state.
- [x] Apply updates through kernel safe writes, stop affected services before
      file replacement, restart them after apply or rollback, and flush through
      the writeback barrier.
- [x] Add Terminal `update status`, `update stage`, `update apply`,
      `update history`, and `update rollback`; mutating operations require an
      admin session.
- [x] Add `recovery rollback` plus update status in Recovery, Diagnostics,
      Sysreport, and the in-OS log/diagnostics viewer.
- [x] Add `make smoke-phase65-update-rollback` and document v7.29.

**Current status:** complete. coolOS can now stage a system file update, snapshot
the previous state, apply the staged payload with service coordination, audit the
operation through `/LOGS/UPDATE.TXT`, and restore the snapshot through either the
normal update command or the recovery surface.

---

## ✅ Phase 66 — Boot Health and Last-Known-Good Rollback

**Goal:** Turn Phase 65 rollback into a boot-time safety net: updates are only
accepted after the next boot reaches a known-good desktop checkpoint, and a
failed validation boot restores the previous snapshot automatically.

- [x] Add `src/boot_health.rs` with `/BOOT/STATE.TXT`,
      `/BOOT/HISTORY.TXT`, and `/BOOT/LAST-GOOD.TXT` layout management.
- [x] Record boot-start attempts durably during early boot, mark the desktop
      ready checkpoint as last-known-good, and audit state transitions.
- [x] Mark successful `update apply` operations as pending validation instead
      of immediately accepting them as healthy.
- [x] Detect a prior failed pending-update validation on the next boot and
      invoke the Phase 65 snapshot rollback path automatically before the
      desktop is marked healthy.
- [x] Add Terminal `boot status`, `boot history`, `boot mark-good`, and
      admin-gated `boot fail-validation <id> <reason>` for diagnostics and
      deterministic recovery smoke coverage.
- [x] Surface boot health in Recovery, Sysreport, Diagnostics, Log Viewer, and
      the in-OS About/help text.
- [x] Add `make smoke-phase66-boot-health` with a writable two-boot image and
      document v7.30.

**Current status:** complete. coolOS now tracks whether a staged update has
survived a full desktop boot, keeps a persistent boot-health history, and can
restore the last-known-good snapshot automatically when the recorded validation
attempt failed before the healthy checkpoint.

---

## ✅ Phase 67 — Signed Updates and Integrity Verification

**Goal:** Add a trust gate to the Phase 65/66 update path so staged updates are
verified before they can modify files, stop services, or become pending boot
validation candidates.

- [x] Add `src/update_crypto.rs` with no_std SHA-256, HMAC-SHA256, hex encoding,
      and constant-time digest comparison helpers for kernel update checks.
- [x] Extend staged update manifests with per-file `sha256=` payload hashes and
      write `/UPDATES/STAGED/UPDATE.SIG` as a keyed `hmac-sha256` signature over
      the manifest.
- [x] Add built-in trust metadata at `/CONFIG/UPDATE-KEYS.TXT` and expose it
      through Terminal `update keys`.
- [x] Verify the trusted key id, algorithm, manifest digest, signature, and each
      payload hash before snapshot creation, service stops, or file writes.
- [x] Add Terminal `update verify` and admin-gated `update sign`, plus
      deterministic admin diagnostics for tamper/unsigned smoke coverage.
- [x] Record successful verification in `/UPDATES/APPLIED.MF`, update history,
      Recovery, Diagnostics, Log Viewer, and Sysreport update sections.
- [x] Add `make smoke-phase67-update-trust` with valid, tampered, unsigned, and
      rollback flows, and document v7.31.

**Current status:** complete. coolOS now refuses unsigned staged updates and
payloads whose bytes no longer match the signed manifest. This phase established
the local HMAC trust gate that Phase 68 later replaced with public-key signing,
multi-key rotation metadata, and rollback protection.

---

## ✅ Phase 68 — Update Key Rotation and Anti-Rollback

**Goal:** Move the update trust path from a single local keyed signature to a
public-key model that can survive key rotation and reject old signed updates.

- [x] Add direct no_std `ed25519-dalek` verification/signing support and expose
      Ed25519 helpers from `src/update_crypto.rs`.
- [x] Replace staged update signatures with Ed25519 signatures over the staged
      manifest while keeping SHA-256 payload and manifest hashes.
- [x] Extend staged manifests with monotonic `version=`, `min_os_version=`, and
      `target_os_version=` metadata.
- [x] Expand `/CONFIG/UPDATE-KEYS.TXT` to public-key metadata with multiple
      built-in keys, generation numbers, validity ranges, trusted, revoked, and
      expired states.
- [x] Enforce trusted key id, algorithm, signature version, key validity window,
      revoked/expired key refusal, unknown-key refusal, and anti-rollback before
      snapshot creation or service stops.
- [x] Add Terminal diagnostics for rotated signing keys and explicit
      `update stage-version` downgrade testing.
- [x] Add `make smoke-phase68-update-keys` covering active-key success,
      downgrade refusal, rotated-key success, revoked key, expired key, unknown
      key, update history, recovery, sysreport, and docs v7.32.

**Current status:** complete. coolOS now verifies staged updates with Ed25519
public keys, accepts both current and rotated trusted keys, and refuses revoked,
expired, unknown, unsigned, tampered, or non-monotonic update candidates before
they can mutate system files.

---

## ✅ Phase 69 — Package Trust and Repair

**Goal:** Bring the package/app platform up to the same trust bar as system
updates: signed package archives, inspectable trust keys, dependency metadata,
owner records, and repair diagnostics before the OS grows more desktop
package-manager surface.

- [x] Add Ed25519 package signing and verification over a normalized package
      trust manifest while keeping `/Packages/*.pkg` readable as UTF-8 app
      manifests.
- [x] Add detached `<package>.sig` sidecars with signer id, algorithm, package
      id/version, issue epoch, manifest SHA-256, and Ed25519 signature.
- [x] Add package trust metadata under `/CONFIG/PACKAGE-KEYS.TXT` with current,
      rotated, revoked, expired, and unknown-key smoke coverage.
- [x] Require valid package signatures before archive install, run-time repair,
      or source verification; refuse unsigned, tampered, revoked, expired,
      unknown-key, incompatible, and downgrade candidates.
- [x] Extend package manifests with `depends=` and `min_os_version=` metadata,
      and refuse installs/repairs when dependencies are missing.
- [x] Write `/APPS/<command>/OWNER.TXT` owner records containing source archive,
      signer, algorithm, installed manifest digest, package trust digest,
      version, and dependencies.
- [x] Extend Terminal `pkg` with `keys`, `info`, `verify`, `repair`, `history`,
      `sign`, `sign-as`, `unsign`, `tamper`, `deps`, and `break` diagnostics.
- [x] Surface package trust status in Recovery, Diagnostics, and Sysreport, with
      package events journaled to `/LOGS/PACKAGES.TXT`.
- [x] Add `make smoke-phase69-package-trust` covering valid install, rotated
      signing key, unsigned/tampered archives, revoked/expired/unknown keys,
      dependency refusal, owner verification, repair, removal, recovery, and
      sysreport output, and document v7.33.

**Current status:** complete. coolOS now refuses untrusted package archives
before they can create launcher entries or run apps, records signed package
ownership under `/APPS`, and can verify or repair installed packages from their
trusted source archive.

---

## ✅ Phase 70 — Package Payloads and Transactional Installs

**Goal:** Move package installation from signed launcher metadata to signed,
hash-verified file payloads with rollback, ownership, and repair semantics close
enough to serve as the foundation for a real desktop package manager.

- [x] Extend `.pkg` manifests with `payload=<target>|<source>|<sha256>|<mode>`
      entries and include normalized payload records in the signed package trust
      manifest.
- [x] Verify package payload source files and SHA-256 hashes before signature
      acceptance, install, or repair.
- [x] Copy payload files into protected targets such as `/bin/pkgdemo` with the
      declared executable mode, while refusing collisions with unowned files or
      payloads owned by another package.
- [x] Extend `/APPS/<command>/OWNER.TXT` with the installed payload table so
      owner records pin source, target, digest, and mode metadata.
- [x] Add `/LOGS/PACKAGE-TXN.TXT` package transaction state for clean, running,
      and rolled-back install/repair/remove operations.
- [x] Roll back partially written payloads, APP.CFG, and OWNER.TXT on injected or
      real install/repair/remove failures.
- [x] Make `pkg verify` check installed payload existence and hashes, make
      `pkg repair` restore tampered payloads from the trusted source archive, and
      make `pkg remove` delete owned payload targets.
- [x] Extend Terminal `pkg` with `transaction`, `install-fail`,
      `tamper-payload`, and `break-payload` diagnostics for payload and rollback
      coverage.
- [x] Update `/SDK/PACKAGE_TEMPLATE.PKG` and `/SDK/README.TXT` for payload
      manifests.
- [x] Add `make smoke-phase70-package-payloads` covering real payload
      install/run/remove, payload tamper repair, source-payload hash refusal,
      transaction rollback, recovery/sysreport diagnostics, and document v7.34.

**Current status:** complete. coolOS packages now install real signed payload
files, verify those files after install, repair them from their trusted source,
remove owned payloads cleanly, and leave an inspectable transaction journal when
an install is rolled back.

---

## ✅ Phase 71 — Browser Engine Port ABI

**Goal:** Stop treating the native HTML/CSS renderer as the path to Chrome-class
compatibility and create the first real host boundary for a mature embeddable
engine, with WPE WebKit selected as the primary target and the existing native
browser kept as a small fallback/debug renderer.

- [x] Add `src/browser_engine.rs` with browser engine port ABI v1, WPE WebKit
      target metadata, native fallback selection, backend readiness probing, and
      a requirement table for process isolation, surfaces, input, networking,
      filesystem, mmap, timers, fonts, threads/futexes, dynamic linking, JIT
      policy, and GPU acceleration.
- [x] Persist default engine policy under `/CONFIG/BROWSER-ENGINE.CFG` and a
      boot-readable engine port log under `/LOGS/BROWSER-ENGINE.TXT`.
- [x] Reserve `/SYSTEM/BROWSER-ENGINE` and define
      `/SYSTEM/BROWSER-ENGINE/WPE.READY` as the future backend-ready probe.
- [x] Add Terminal `engine`, `engine abi`, `engine requirements`,
      `engine config`, `engine log`, and `engine recovery` diagnostics.
- [x] Add `browser://engine` and link it from the Browser home/compatibility
      pages so the GUI exposes the same WPE readiness state.
- [x] Include browser-engine readiness in Recovery, Diagnostics, Sysreport,
      ABI output, config-store diagnostics, and in-OS About/Diagnostics text.
- [x] Generate `/SDK/BROWSER_ENGINE_PORT.TXT` and extend `/SDK/README.TXT`,
      Terminal `devkit`, and `/bin/devkit` so engine-port work has a discoverable
      host-contract document.
- [x] Add `make smoke-phase71-browser-engine-port` covering Terminal engine
      diagnostics, ABI manifest output, requirement blockers, browser launch
      routing to `browser://engine`, sysreport/recovery lines, and SDK docs.
- [x] Update README, Roadmap, and in-OS text for v7.35.

**Current status:** complete. coolOS now has an explicit WPE WebKit port target
and an inspectable browser engine host contract. A real WebKit backend is not
booting yet; the readiness surface deliberately identifies the remaining OS
work after Phase 75: full dynamic-link dependency/TLS/C runtime support,
larger/file-backed mappings, JavaScriptCore JIT/interpreter policy, richer
POSIX socket/file semantics, scalable fonts/text shaping, remaining pthread
edge cases, and eventually graphics acceleration.

---

## ✅ Phase 72 — Userspace Threads and Futex ABI

**Goal:** Add the first same-address-space userspace threading primitive and a
word-address futex wait/wake ABI so a hosted WebKit-class runtime has a real
blocking synchronization substrate instead of processes only.

- [x] Extend scheduler tasks with thread-group ids and per-thread user-stack
      slot metadata.
- [x] Add reserved userspace thread-stack slots below the mmap arena and map a
      fresh 64 KiB stack for each spawned thread.
- [x] Add `thread_spawn(entry, arg, flags)` as ABI v11 syscall 47. The spawned
      task shares the caller's PML4, inherits fd references, starts in ring 3
      with `rdi=arg`, and has its own kernel syscall/IRQ stack.
- [x] Rework wait/reap/OOM cleanup so shared address spaces are not freed until
      the last sibling using that PML4 has been reaped.
- [x] Add kernel futex waiter tracking plus `futex_wait(addr, expected,
      timeout_ms)` and `futex_wake(addr, count, flags)` as ABI v11 syscalls 48
      and 49.
- [x] Add futex cleanup on task exit/fault/kill and futex counters in
      Diagnostics and Sysreport.
- [x] Add `libcool::thread::{spawn,join,futex_wait,futex_wake}` wrappers and
      `/bin/threaddemo` to exercise two worker threads, shared atomics, futex
      wake, and `waitpid`-based joins.
- [x] Update the browser-engine requirement table so `threads-futex` is partial
      instead of missing, with TLS/pthread libc integration called out as the
      next layer.
- [x] Add `make smoke-phase72-threads-futex` and update SDK/README/Roadmap docs
      for ABI v11.

**Current status:** complete. coolOS now has the kernel primitive needed for
threaded userspace runtimes: same-address-space ring-3 tasks, private stacks,
futex wait/wake, and join/reap semantics. This is not a complete POSIX pthread
stack yet; TLS segment setup, libc pthread shims, robust futex variants, and
shared process-wide signal/exit policy remain future work.

---

## ✅ Phase 73 — Thread-Local Storage and Pthread Runtime Groundwork

**Goal:** Turn the raw thread/futex ABI into the next runtime layer a hosted
browser engine expects: per-thread TLS state, spawn-with-TLS, and pthread-style
synchronization helpers that can later back a libc/POSIX pthread ABI.

- [x] Add per-task FS-base TLS state to the scheduler and reload it on every
      context switch alongside CR3 and TSS RSP0.
- [x] Add ABI v12 syscalls `thread_tls_set(base, flags)`,
      `thread_tls_get()`, and `thread_spawn_tls(desc_ptr)` where the descriptor
      carries entry, argument, initial TLS base, and flags.
- [x] Reset the current task's TLS base on `exec` so replaced images do not
      inherit stale thread-control-block pointers.
- [x] Extend scheduler/resource diagnostics with TLS-base and TLS-thread
      counts for task memory/sysreport output.
- [x] Add `libcool::thread::TlsBlock`, TLS keys, FS read/write helpers,
      `spawn_tls`, `install_tls_block`, and `bind_current_tls_os_tid`.
- [x] Add futex-backed `PThreadMutex`, `PThreadCondvar`, and `PThreadOnce`
      helpers to establish the pthread-shaped synchronization surface.
- [x] Add `/bin/tlsdemo` to verify independent per-thread FS-base TLS,
      TLS-key slots, condition-variable wakeups, once initialization, and
      join/reap results.
- [x] Update the browser-engine requirement table so `threads-futex` is ready
      and hosted libc/POSIX pthread wiring is staged as the next runtime layer.
- [x] Add `make smoke-phase73-tls-pthread` and update SDK/README/Roadmap docs
      for ABI v12.

**Current status:** complete. coolOS now saves and restores a real user TLS
base per scheduled task, can launch a userspace thread with its TLS base already
installed before first instruction, and has SDK-level pthread-style blocking
primitives on top of futexes. A fuller upstream libc pthread ABI still needs
dynamic linking, ELF TLS relocations, symbol/export integration, destructors,
and process-wide multi-thread signal/exit policy in later phases.

---

## ✅ Phase 74 — POSIX Pthread and Libc Shim

**Goal:** Put a hosted-runtime shaped API on top of the Phase 73 thread/TLS/futex
substrate so browser-engine ports and C runtime experiments can call familiar
`pthread_*`, `errno`, and timing symbols before a full dynamic libc exists.

- [x] Reserve TLS slot 0 for per-thread `errno` and keep application-created
      TLS keys in the remaining slots.
- [x] Add `libcool::posix` plus `libcool::libc` re-exports for
      `pthread_create`, `pthread_join`, `pthread_exit`, `pthread_self`, and
      `pthread_equal`.
- [x] Back `pthread_create` with `thread_spawn_tls`, fixed runtime-owned TLS
      blocks, and a trampoline that records POSIX return pointers as join
      statuses.
- [x] Add POSIX-shaped wrappers for pthread mutexes, condition variables,
      `pthread_once`, pthread keys/specific values, and no-op attr/destroy
      helpers.
- [x] Add libc-adjacent helpers for `errno`, `__errno_location`,
      `init_main_thread`, `gettid`, `sched_yield`, `nanosleep`, and `usleep`.
- [x] Add `/bin/pthreaddemo` to verify pthread creation/join, condition wakeups,
      once initialization, pthread-specific values, per-thread `errno`, yield,
      and sleep behavior through the compatibility layer.
- [x] Add `make smoke-phase74-pthread-libc` and update SDK, browser-engine,
      README, Roadmap, and in-OS text/docs.

**Current status:** complete. coolOS now has a bounded no-alloc POSIX pthread
shim suitable for early hosted-engine integration work. It is not a complete
libc or upstream pthread implementation yet: dynamic linking, ELF TLS
relocations, pthread destructors, robust/recursive mutex variants, cancellation,
signal semantics across multi-threaded process groups, and broader POSIX
file/socket compatibility remain future runtime work.

---

## ✅ Phase 75 — Dynamic Loader Foundation

**Goal:** Stop blocking hosted-runtime work on static-only userspace images by
adding the first real shared-object loader path: W^X executable mappings,
ET_DYN dynamic metadata parsing, relocations, export lookup, init arrays, and a
ring-3 proof that code loaded from `/lib` can be invoked safely.

- [x] Add ABI v13 `mprotect(addr, len, flags)` for existing mmap-arena pages,
      with bit 0 as writable, bit 1 as executable, and writable+executable
      mappings rejected by both `mmap` and `mprotect`.
- [x] Add VMM support for changing leaf PTE flags on already-mapped user pages
      while keeping the operation constrained to existing user mappings.
- [x] Extend `libcool::memory` with `mmap_flags` and `mprotect`.
- [x] Add no-alloc `libcool::dynlink` support for ELF64 ET_DYN objects:
      `PT_LOAD` mapping, `PT_DYNAMIC` parsing, SysV hash symbol counts,
      dynsym/dynstr export lookup, RELA relocation handling for
      `RELATIVE`, `64`, `GLOB_DAT`, and `JUMP_SLOT`, init-array execution,
      and final W^X segment sealing.
- [x] Generate a real test shared object at build time as `/lib/libphase75.so`,
      containing executable x86_64 text, writable data, RELA slots, dynsym
      exports, and an init-array function.
- [x] Add `/bin/lddemo` to load `/lib/libphase75.so`, resolve
      `phase75_add` and `phase75_increment`, call the loaded function, and
      verify the init array updates the exported data before invocation.
- [x] Add `/lib` to the generated FAT/CoolFS image and route `.so` build
      artifacts there while keeping normal extra ELFs under `/bin`.
- [x] Update browser-engine readiness so dynamic linking and executable memory
      policy move from missing to partial, with the remaining `DT_NEEDED`, ELF
      TLS, libc `ld.so`, C++ runtime, and JIT policy work called out.
- [x] Add `make smoke-phase75-dynlink` and update README, Roadmap, SDK, in-OS
      About text, and browser-engine docs/logs for v7.39.

**Current status:** complete. coolOS can now load a bounded shared object from
`/lib`, apply dynamic relocations, seal text pages executable/non-writable, run
an init array, resolve exports, and call loaded code from ring 3. This is not a
complete ELF dynamic linker yet: dependency graphs (`DT_NEEDED`), soname/cache
resolution, ELF TLS records, PLT/lazy binding, symbol versioning, libc `ld.so`,
C++ runtime constructors/destructors, file-backed mappings, and browser-engine
JIT policy remain future work.

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
`File::create`, `pipe`, `mmap`, `shmem_create`, `spawn_args`,
`spawn_fds_args`, `thread::spawn`, `thread::spawn_tls`,
`thread::set_tls_base`, `thread::futex_wait`, `thread::PThreadMutex`,
`thread::PThreadCondvar`, `thread::PThreadOnce`, `evented::poll`,
`tty::enter_raw_mode`, `dynlink::load`, `memory::mprotect`, `read_event`,
`dns_resolve`, TCP sockets, time, and filesystem utility calls plus GUI windows through `libcool::fs` and
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
| v6.3 | Phase 29 complete: login, sessions, and service supervision |
| v6.4 | Phase 30 complete: GUI login and lock screen |
| v6.5 | Phase 31 complete: first-run setup and account management |
| v6.6 | Phase 32 complete: user/kernel isolation hardening |
| v6.7 | Phase 33 complete: process control and jobs |
| v6.8 | Phase 34 complete: TTY sessions and foreground job control |
| v6.9 | Phase 35 complete: real TTY stdin and line discipline |
| v7.0 | Phase 36 complete: userspace shell foundation |
| v7.1 | Phase 37 complete: coreutils command set |
| v7.2 | Phase 38 complete: utility app dependability |
| v7.3 | Phase 39 complete: recovery and repair path |
| v7.4 | Phase 40 complete: shell semantics, cwd, redirection, and pipelines |
| v7.5 | Phase 41 complete: filesystem durability and metadata |
| v7.6 | Phase 42 complete: app consistency and in-OS help |
| v7.7 | Phase 43 complete: observability and sysreport |
| v7.8 | Phase 44 complete: developer platform and SDK devkit |
| v7.9 | Phase 45 complete: compositor latency and smoothness |
| v7.10 | Phase 46 complete: adaptive high refresh |
| v7.11 | Phase 47 complete: evented userspace runtime |
| v7.12 | Phase 48 complete: terminal/TUI platform |
| v7.13 | Phase 49 complete: browser engine foundation |
| v7.14 | Phase 50 complete: CSS layout pass |
| v7.15 | Phase 51 complete: browser forms |
| v7.16 | Phase 52 complete: DOM/event foundation |
| v7.17 | Phase 53 complete: DOM-backed browser forms |
| v7.18 | Phase 54 complete: Browser POST submission |
| v7.19 | Phase 55 complete: Browser session state |
| v7.20 | Phase 56 complete: CSS2 box model and reflow |
| v7.21 | Phase 57 complete: Browser layout and parser fidelity |
| v7.22 | Phase 58 complete: Browser subresources and cache |
| v7.23 | Phase 59 complete: Browser JavaScript and DOM runtime |
| v7.24 | Phase 60 complete: Browser web-app APIs |
| v7.25 | Phase 61 complete: Browser modern-page compatibility |
| v7.26 | Phase 62 complete: Kernel resource limits and cleanup |
| v7.27 | Phase 63 complete: Memory pressure and OOM recovery |
| v7.28 | Phase 64 complete: Persistent service supervision and recovery |
| v7.29 | Phase 65 complete: System update, snapshot, and rollback |
| v7.30 | Phase 66 complete: Boot health and last-known-good rollback |
| v7.31 | Phase 67 complete: Signed updates and integrity verification |
| v7.32 | Phase 68 complete: Update key rotation and anti-rollback |
| v7.33 | Phase 69 complete: Package trust and repair |
| v7.34 | Phase 70 complete: Package payloads and transactional installs |
| v7.35 | Phase 71 complete: Browser engine port ABI |
| v7.36 | Phase 72 complete: Userspace threads and futex ABI |
| v7.37 | Phase 73 complete: Thread-local storage and pthread runtime groundwork |
| v7.38 | Phase 74 complete: POSIX pthread and libc shim |
| v7.39 | Current — Phase 75 complete: dynamic loader foundation |
