<img width="1392" height="864" alt="image" src="https://github.com/user-attachments/assets/5824d8c0-d0d8-4b79-b650-9fec05fa0c70" />


# coolOS

A 64-bit operating system kernel written in Rust. Boots bare-metal into a
graphical desktop with draggable and resizable windows, a taskbar, a start
menu, hardware input, kernel and userspace GUI applications, a preemptive
scheduler, ring-3 userspace, per-process virtual memory, process isolation, a
CoolFS root filesystem with VFS/syscalls and uid/gid/mode enforcement, a FAT32
legacy import mount, an ELF loader with `exec`, task-local cwd and fd-mapped
stdio, userspace thread groups, futex wait/wake, per-thread TLS bases,
POSIX pthread/libc userspace shims, a W^X ET_DYN shared-object loader with
bounded dependency/TLS support, read-only file-backed mmap for userspace files
and shared-object text, and IPC with pipes, shared memory, and per-task fd
tables.

---

# Current state — v7.55

The kernel boots into a graphical desktop at **1920×1080, 24bpp** via the
`bootloader 0.11` linear framebuffer on the default BIOS/VBE path and via the
parallel QEMU OVMF UEFI/GPT path, including AHCI/SATA-attached disks and
QEMU xHCI USB mass-storage or NVMe root disks. A terminal window opens on boot.
Right-clicking the desktop opens a context menu to launch additional apps, and
the shell also exposes desktop icons plus a start menu/taskbar flow, global
keyboard shortcuts, `Ctrl+Space` Start-menu search, a task switcher overlay,
edge/keyboard window snapping, taskbar previews/actions, boot-splash-style GUI
login/lock screen with first-run setup, session restore, Accounts settings,
File Manager drag/drop/open-with actions, a shared clipboard, notification
center, userspace app lifecycle tracking with System Monitor close/kill/path
controls, and desktop settings that persist to the CoolFS root.
A preemptive round-robin scheduler is driven by the PIT timer at **288 Hz**;
the kernel boot stack remains the idle/window-manager context, the boot path
performs a synchronous CoolFS read check, and the terminal can also spawn
additional ring-3 ELF tasks from disk with `exec`. The shell also has a real
signed package/app manifest path:
UTF-8 `.PKG` manifests under `/Packages` require detached Ed25519 sidecar
signatures before they can install apps into `/APPS/<command>/APP.CFG`,
contribute launcher aliases and file associations, launch a declared userspace
executable through `exec=`, or be repaired from their source archive. Package
owner records under `/APPS/<command>/OWNER.TXT` pin the source path, signer,
manifest digests, version, and dependencies. Manifest permission labels become
launch-time task capabilities, and the VFS/syscall layer enforces filesystem,
network, desktop, and execute access.
Fresh interactive images now start with a compositor-owned first-boot setup
card when the default `root/cool` handoff is still present. The wizard creates
the real owner admin account, writes `/CONFIG/FIRSTBOOT.CFG`, disables the
default handoff through the same account path as Terminal `setup`, and then
continues into the desktop. After setup, the desktop starts behind the normal
compositor-owned greeter; the active session is
backed by a persistent CoolFS user database, so
Terminal commands, launched ELF tasks, and package apps inherit the logged-in
user's uid/gid and non-admin users cannot mutate protected ownership or service
state without switching back to an admin session. Phase 31 adds a first-run
admin handoff, account create/disable/role/password/delete flows, login
throttling, and persistence smoke coverage for those account records; Phase 80
adds the graphical first-boot installer flow on top of that existing security
model, Phase 81 adds recovery/reset hardening so interrupted or inconsistent
first-boot state can be inspected, reset, or repaired, Phase 82 adds a QEMU
disk installer path, and Phase 83 makes that installed target self-booting by
writing the BIOS bootloader plus a CoolFS root partition to the target disk.
Phase 84 adds safer target selection and graphical progress, Phase 85 adds
a parallel QEMU UEFI/GPT installer path while keeping BIOS/MBR supported,
Phase 86 adds AHCI/SATA storage plus a USB-flashable raw image artifact, and
Phase 87 boots that image as runtime USB mass storage, Phase 88 adds QEMU NVMe
root boot and NVMe installer targets, and Phase 89 adds bare-metal USB boot
readiness diagnostics plus a conservative safe USB image fallback. Phase 90
adds cautious physical-install guardrails for USB-booted systems: the live
`usb0` source is protected, internal `sata*` and `nvme*n1` disks are reported as
candidate targets, and installs require explicit target-name confirmation plus
flush/verify before rebooting the internal disk. Phase 91 hardens bare-metal
readiness diagnostics with USB hub/UASP classification, detailed root-scan
reasons, physical-installer preflight verdicts, and QEMU topology smoke
coverage while keeping BOT USB storage as the supported USB boot path. Phase 92
adds a QEMU Secure Boot firmware path with local development key material,
`uefi-secure.img`, `coolos-usb-secure.img`, and a digest-bound UEFI loader that
verifies the kernel before handoff. Phase 93 turns that into an enforced
QEMU/OVMF local-key chain with enrolled PK/KEK/db variables, a PE/COFF-signed
`BOOTX64.EFI`, artifact verification, and negative smoke coverage for unsigned
loaders, tampered signed loaders, and kernel digest mismatches. Phase 94 makes
that secure USB path usable on real UEFI PCs that allow custom key enrollment:
`target/secure-boot/enroll/` contains public DER certs, ESL/auth files,
fingerprints, checksums, and firmware-enrollment instructions, and
`coolos-usb-secure.img` embeds the same bundle under `/EFI/COOLOS/ENROLL/`
plus `/EFI/COOLOS/SECUREBOOT.TXT`. Microsoft/shim compatibility, production
key management, signed update rollout, and arbitrary real-PC Secure Boot
remain future work.
Phase 32
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
templates. Phases 45-77 add compositor smoothness, evented terminal work, and
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
bounded margins, padding, borders, backgrounds, percentage widths, positioned
and floating boxes, z-index paint ordering, stronger table/list layout, tolerant
HTML parser repair for common implied closes, and smoke fixtures for the browser
engine, CSS layout, forms, DOM events, DOM-backed form interaction, POST form
pages, session state, box-model pages, deeper layout pages, and subresource
cache pages. Phases 59-61 add a bounded JavaScript/DOM runtime with same-origin
script loading, inline handlers, `addEventListener` hooks, bounded timers,
script-driven DOM text/class/form mutations, persistent `localStorage`,
per-document `sessionStorage`, JS `document.cookie`, `location`/`history`
hooks, `querySelectorAll`, `classList`, attribute/style mutations, same-origin
fetch callbacks, robust raw `<head>`/`<script>`/`<style>` suppression for
modern script-heavy pages, main-resource content-type routing so JavaScript and
non-HTML resources show source diagnostics instead of being laid out as HTML,
a Google/Search compatibility shell that keeps the search form usable on
`https://www.google.com/`, and `browser://js`, `browser://storage`, and
`browser://compat` diagnostics; Phase 71 adds `browser://engine` for the WPE
WebKit port ABI/readiness view. Phase 62 shifts back to core OS hardening:
active user tasks are capped, user address spaces and individual `mmap` calls
are bounded, fd allocation preflights before object allocation, shared-memory
and socket quotas are enforced per task and globally, task exit/fault paths
close owned sockets and file tables, and diagnostics/sysreport expose a
resource-limits section. Phase 63 adds memory-pressure recovery: allocator
snapshots classify heap pressure as normal/low/critical, task memory estimates
show user pages, shared memory, kernel stacks, fds, and sockets, the main loop
periodically trims reclaimable CoolFS and Browser caches, large cache/task/fd
allocations are admitted against a reserve, and a critical heap can reclaim the
largest non-current user task through the scheduler cleanup path. Phase 64 makes
the kernel service supervisor durable: service desired state is persisted under
`/CONFIG/SERVICES.CFG`, restart/history snapshots are written to
`/LOGS/SERVICES.TXT`, services carry dependency metadata and restart backoff,
Terminal exposes admin-gated `services start|stop|restart|fail|run` controls
plus readable `services status|history|recovery` diagnostics, and recovery,
sysreport, Diagnostics, and System Monitor surface degraded service state.
Phase 65 adds system update staging and rollback: `/UPDATES/STAGED/UPDATE.MF`
describes staged file payloads, `/UPDATES/SNAPSHOTS/LAST/MANIFEST.TXT` records
pre-apply rollback snapshots, `/LOGS/UPDATE.TXT` journals stage/apply/rollback
events, affected services are stopped and restarted around update writes, and
Terminal plus Recovery expose `update apply`, `update rollback`, and
`recovery rollback`. Phase 66 adds boot health and last-known-good recovery:
`/BOOT/STATE.TXT` tracks boot validation attempts, pending update ids, and the
last healthy desktop boot; `/BOOT/HISTORY.TXT` audits boot-start, validation,
and rollback decisions; update apply marks the next boot as pending validation;
desktop-ready marks it good; and a failed validation attempt triggers automatic
rollback from the Phase 65 snapshot on the next boot. Phase 67 adds staged
update trust verification: staged manifests carry per-payload `sha256=` fields,
`/UPDATES/STAGED/UPDATE.SIG` stores a detached manifest signature,
`/CONFIG/UPDATE-KEYS.TXT` exposes the built-in trusted update key metadata,
Terminal adds `update verify`, `update keys`, and admin-gated `update sign`, and
`update apply` refuses unsigned, tampered, or hash-mismatched payloads before it
captures a snapshot or stops services. Phase 68 upgrades that trust gate to
Ed25519 public-key verification with multiple built-in keys, rotation metadata,
revoked/expired key handling, versioned manifests, and anti-rollback refusal for
older signed updates. Phase 69 applies the same public-key discipline to
installable packages: `/CONFIG/PACKAGE-KEYS.TXT` exposes trusted, rotated,
revoked, and expired package signing keys; `pkg verify` refuses unsigned,
tampered, revoked, expired, unknown-key, dependency-missing, or downgrade
package archives; installed package owner records make `pkg repair`,
Recovery, Diagnostics, and Sysreport report package trust state. Phase 70 turns
those archives into real payload installers: manifests can declare
`payload=<target>|<source>|<sha256>|<mode>` entries, installs copy payload files
transactionally, `/LOGS/PACKAGE-TXN.TXT` records clean or rolled-back package
transactions, owner records pin payload hashes, and verify/repair/remove cover
the installed payloads as well as the launcher manifest. Phase 71 starts the
real modern-browser track: WPE WebKit is now the selected engine target,
`src/browser_engine.rs` defines a versioned browser engine port ABI/readiness
surface, `/CONFIG/BROWSER-ENGINE.CFG` records the desired engine/fallback path,
`/SDK/BROWSER_ENGINE_PORT.TXT` documents the host contract and blockers, and
Terminal, Browser, Recovery, Diagnostics, and Sysreport expose the engine-port
state while the native browser remains the fallback/debug renderer. Phase 72
adds the first browser-runtime threading substrate: ring-3 tasks can spawn
same-address-space userspace threads with private user/kernel stacks, shared
PML4 lifetime is reaped only after the last sibling exits, ABI v11 adds
`thread_spawn`, `futex_wait`, and `futex_wake`, `libcool::thread` wraps the
ABI, `/bin/threaddemo` verifies two worker threads plus futex wake/join, and
Diagnostics/Sysreport expose futex and thread-stack telemetry. Phase 73 moves
that substrate closer to a real runtime ABI: ABI v12 adds `thread_tls_set`,
`thread_tls_get`, and `thread_spawn_tls`; the scheduler reloads each task's
FS-base on every context switch; `libcool::thread` adds `TlsBlock`, TLS keys,
spawn-with-TLS, and pthread-style mutex/condvar/once helpers backed by futexes;
`/bin/tlsdemo` verifies independent per-thread TLS plus condition-variable
wakeups; and browser-engine readiness now marks `threads-futex` ready while
moving hosted libc/POSIX pthread wiring to the runtime layer. Phase 74 adds
that first POSIX-shaped runtime layer inside `libcool::posix`/`libcool::libc`:
`pthread_create`, `pthread_join`, `pthread_exit`, `pthread_self`, mutexes,
condition variables, once, pthread keys, per-thread `errno`, `gettid`,
`sched_yield`, and `nanosleep` now sit on top of the TLS/futex/thread
primitives, and `/bin/pthreaddemo` verifies the compatibility path. Phase 75
adds ABI v13 `mprotect`, W^X executable mmap transitions, `/lib` shared-object
image placement, `libcool::dynlink` for ET_DYN `PT_DYNAMIC` parsing, RELA
relocations, dynsym lookup, init-array execution, and `/bin/lddemo` calling an
exported function from `/lib/libphase75.so`.
Phase 76 extends that loader to bounded `/lib` dependency graphs:
`dynlink::load_with_deps` follows `DT_NEEDED` sonames, resolves dynsym exports
across loaded objects, copies ELF TLS templates, applies TLS relocations,
runs dependency init arrays before dependents, and `/bin/lddemo` verifies
`/lib/libphase76main.so` calling into `/lib/libphase76dep.so`.
Phase 77 adds ABI v14 `mmap_file(desc)`, read-at VFS support for regular file
descriptors, VMM accounting for file-backed pages, `libcool::memory::mmap_file`,
POSIX-shaped `io::open_flags`, read-only file-backed mappings for ET_DYN
read-only segments, and `/bin/mmapdemo` covering `/TMP`, `/bin/motd.txt`,
write-map denial, executable file maps, and diagnostics.

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
session history, persistent local bookmarks, persistent browser cookies, and a
compatibility mode for Google/Search pages that would otherwise expose raw
Closure script text.
HTTPS uses a
no_std TLS 1.3 client over the kernel TCP stack with hardware RNG entropy,
RTC-backed certificate validity checks, X.509 chain validation against the
built-in trust roots, and SAN-first hostname validation coverage.

### What's working

| Subsystem | Details |
| :-------- | :------ |
| **Framebuffer** | `bootloader 0.11` linear framebuffer at 1920×1080 on the normal QEMU `-vga std` BIOS/VBE path and the QEMU OVMF UEFI/GPT path, including AHCI-backed, USB-storage-backed, and NVMe-backed UEFI boots. The safe USB image requests 1024×768 as a physical-machine fallback. 3bpp and 4bpp both handled. Shadow-buffer compositor with dirty row-span blits, adaptive 36/144 Hz frame pacing, and a hardware cursor overlay fast path that restores/draws only cursor rectangles for mouse-only motion. No tearing. |
| **PS/2 mouse** | Full hardware init (CCB, 0xF6/0xF4), 9-bit signed X/Y deltas, IRQ12 packet collection via atomics. |
| **Window manager** | Z-ordered windows, focus-on-click, title-bar drag, edge snapping, keyboard snapping, task switcher overlay, minimise/maximise/restore, resize grip, close button, taskbar previews/right-click actions, per-window pixel back-buffer. |
| **Desktop shell** | Wallpaper, desktop icons, right-click context menu, searchable start menu, taskbar window buttons, configurable shortcuts, login/lock greeter, Accounts settings, notification center, File Manager drag/drop/open-with routing, shared clipboard plumbing, userspace app lifecycle tracking with System Monitor controls, persistent settings, session restore, clock, and adaptive compositor smoothness telemetry. |
| **Heap** | `LockedHeap` allocator — `String`, `Vec`, `Box` all work. 96 MiB heap to accommodate 1080p shadow and window buffers, with high-water snapshots, low/critical pressure states, allocation admission reserves, and memory-pressure diagnostics. |
| **Paging / VMM** | 4-level `OffsetPageTable` + global `BootInfoFrameAllocator`. Per-process PML4 cloned from kernel upper half; private user-space mappings in lower half. `vmm::` module exposes `new_process_pml4`, `map_page_in`, `map_region`, `map_file_frame_in`, `protect_region`, and `switch_to`, with resource stats distinguishing anonymous owned pages from file-backed pages. |
| **IDT** | Breakpoint, Double Fault, Page Fault with user-task termination/crashdump handling for invalid ring-3 accesses, General Protection Fault, Invalid Opcode, Timer (IRQ0), Keyboard (IRQ1), Mouse (IRQ12). |
| **Scheduler** | Preemptive round-robin at 288 Hz. Each task carries `pml4: Option<PhysFrame>` plus `uid`, `gid`, capability credentials, current working directory, process group, controlling TTY, pending signal state, thread-group id, optional user-stack slot, per-thread FS/TLS base, and a private 64 KiB kernel stack. The scheduler switches CR3 when needed, reloads the current task's TLS base, and updates TSS RSP0 to the selected task's stack so ring-3 IRQ frames never share one global stack. Task lifecycle now distinguishes ready/running/blocked/stopped/exited/reaped states, records parents and exit codes, supports blocking `waitpid`/reaping, supports same-address-space userspace threads without freeing a shared PML4 until the last sibling is reaped, supports kernel-side task termination and OOM reclaim of the largest non-current user task, and backs blocking pipe/TTY/futex waits with `block_current` / `unblock(id)`. |
| **TTY sessions** | Each Terminal owns a kernel TTY with canonical/raw input modes, echo and signal flags, cell geometry, a dedicated output queue, and a foreground process group. Userspace `read(0)` plus stdout/stderr route through the task's controlling TTY by default, fd mappings can override 0/1/2 for pipes and redirection, `exec` blocks the prompt as a foreground job, background jobs keep their terminal output route, and Ctrl+C/Ctrl+Z signal the foreground group when signal mode is enabled. |
| **Process isolation** | Two user processes share the same user-stack virtual address (`0x7FFF_0010_0000`) but map it to different physical frames. Guard pages (kernel-only) sit below each stack. |
| **GDT + TSS** | Four segments (kernel code/data ring 0, user code/data ring 3) + TSS. RSP0 starts on a fallback ISR stack and is updated on every context switch to the selected task's private kernel stack for ring-3 IRQ entry. |
| **SYSCALL/SYSRET** | EFER.SCE enabled. STAR/LSTAR/SFMASK MSRs configured. Naked `syscall_entry` saves context, switches to the currently scheduled task's private kernel stack top, dispatches on rax, restores context, and executes `sysretq`. |
| **Syscall table** | `0 exit`, `1 write`, `2 yield`, `3 getpid`, `4 mmap(addr, len, flags)`, `5 open(path, len)`, `6 read(fd, buf, len)`, `7 close(fd)`, `8 exec(path, len)`, `9 pipe(fds_ptr)`, `10 dup(fd)`, `11 shmem_create(len)`, `12 shmem_map(id)`, `13 waitpid(pid, status_ptr)`, `14 spawn(path, len)`, `15 sleep_ms(ms)`, `16 abi_version()`, `17 dns_resolve(host, len)`, `18 http_get(host, len)`, `19 socket(domain, type, proto)`, `20 connect(socket, ipv4, port)`, `21 send(socket, buf, len)`, `22 recv(socket, buf, len)`, `23 gui_open(title, len, dims)`, `24 gui_present(handle, pixels, len)`, `25 gui_poll_event(handle, packet, len)`, `26 gui_close(handle)`, `27 fs_write_file(desc)`, `28 fs_create_dir(path, len)`, `29 fs_delete_tree(path, len)`, `30 fs_list_dir(desc)`, `31 screenshot(path, len, flags)`, `32 signal(pid, signal)`, `33 setpgid(pid, pgid)`, `34 getpgid(pid)`, `35 signal_group(pgid, signal)`, `36 spawn_args(desc)`, `37 chdir(path, len)`, `38 getcwd(buf, len)`, `39 stat(desc)`, `40 rename(desc)`, `41 open_write(path, len)`, `42 spawn_fds_args(desc)`, `43 sync()`, `44 time()`, `45 poll(desc, count, timeout_ms)`, `46 tty_control(op, arg1, arg2)`, `47 thread_spawn(entry, arg, flags)`, `48 futex_wait(addr, expected, timeout_ms)`, `49 futex_wake(addr, count, flags)`, `50 thread_tls_set(base, flags)`, `51 thread_tls_get()`, `52 thread_spawn_tls(desc_ptr)`, `53 mprotect(addr, len, flags)`, and `54 mmap_file(desc_ptr)` where the descriptor is `[fd, addr, len, file_offset, flags]`. `mmap`/`mprotect`/`mmap_file` use bit 0 for writable and bit 1 for executable, reject writable+executable mappings, and keep the dynamic-loader path W^X; Phase 77 `mmap_file` intentionally allows only read-only or executable private file mappings. `sys_read(0)` reads from the current task's controlling TTY when assigned unless fd 0 is mapped; `sys_write` writes stdout/stderr to that TTY, falls back to the compositor ring for orphaned output, or writes pipe/file descriptors through the VFS fd table. |
| **Userspace** | Ring-3 code can run either as the original isolation stubs or as real ELF64 binaries loaded from `/bin`. The `libcool` SDK crate now provides no_std entry/argv setup plus process, signal/process-group, file/open_flags, pipe, thread/futex/TLS, pthread-style mutex/condvar/once/key helpers, POSIX-shaped `pthread_*`/`errno`/`nanosleep` wrappers, ET_DYN `dynlink` single-object and dependency-graph loading, evented poll, TTY mode/size, mmap/mmap_file/mprotect, shared-memory, event, DNS/HTTP, TCP socket, filesystem utility, screenshot, time, and userspace GUI wrappers. `/bin/sh` reads stdin from the TTY, tracks the kernel cwd, parses quoting and escapes, runs builtins, resolves bare commands under `/bin`, supports `<`/`>` redirection and one-stage `|` pipelines, and can launch argv/fd-capable children with `spawn_args` or `spawn_fds_args`. `/bin/ls`, `/bin/cat`, `/bin/echo`, `/bin/pwd`, `/bin/mkdir`, `/bin/touch`, `/bin/rm`, `/bin/writefile`, `/bin/cp`, `/bin/mv`, `/bin/grep`, `/bin/head`, `/bin/tail`, `/bin/date`, `/bin/uname`, `/bin/clear`, `/bin/stat`, `/bin/sync`, `/bin/devkit`, `/bin/polldemo`, `/bin/tuidemo`, `/bin/threaddemo`, `/bin/tlsdemo`, `/bin/pthreaddemo`, `/bin/mmapdemo`, and `/bin/lddemo` cover practical command-line, evented, terminal-mode, thread/futex, TLS, POSIX pthread, file-backed mapping, and shared-object loading workflows. `sys_exec` replaces the current userspace image in-place by swapping CR3 and rewriting the saved syscall return frame. Shared memory (`sys_shmem_create`/`sys_shmem_map`) maps a region of physical frames into the caller's address space at a fixed VA. |
| **ELF loader** | Kernel `exec` validates ELF64 headers, maps static `PT_LOAD` segments into a fresh address space, allocates a private user stack, builds an initial `argc/argv/envp` stack frame, and can either spawn a new task or prepare an image for `sys_exec`. Phase 77 userspace `libcool::dynlink` handles ET_DYN objects under `/lib`, with `PT_DYNAMIC`, RELA, dynsym, init-array, W^X `mprotect`, bounded `DT_NEEDED` dependency loading, global cross-object symbol resolution, ELF TLS template/relocation coverage, and read-only file-backed `PT_LOAD` segments through `mmap_file`. |
| **Storage layer** | Generic block-device ids cover IDE (`ide0-master`, `ide0-slave`, `ide1-master`, `ide1-slave`), QEMU AHCI/SATA (`sata0`, `sata1`, `sata2`, ...), QEMU xHCI USB mass storage (`usb0`, `usb1`, ...), and QEMU NVMe (`nvme0n1`, `nvme1n1`, ...). IDE uses LBA28 PIO with bounded retries; AHCI v1 uses polled command slots for identify/read/write/flush on QEMU; USB MSC uses Bulk-Only Transport with SCSI inquiry/capacity/read/write/flush commands; NVMe uses polled admin/I/O queues for namespace-1 read/write/flush. Root discovery resolves legacy raw CoolFS disks, BIOS/MBR `0xc0` CoolFS partitions, and GPT CoolFS partitions across IDE, SATA, USB, and NVMe. |
| **CoolFS layer** | Native coolOS root filesystem mounted at `/`, stored directly at LBA 0 for live/dev images, inside a private MBR partition on BIOS-installed disks, or inside a coolOS GPT partition on UEFI-installed disks. It has a CoolFS superblock, fixed inode table with durable `uid`/`gid`/mode metadata, block bitmap, 4 KiB blocks, direct plus indirect data blocks, directory records, a 64-slot block cache with dirty 4 KiB writeback, pressure-triggered clean-cache trimming, VFS read/write/create/rename/delete/copy routing, stats, and boot self-tests. |
| **FAT32 layer** | Optional legacy import mount at `/FAT`, formatted in a separate 8 MiB-offset region relative to the resolved root disk/partition. BPB parsing, FAT chain walking, short-name and long-filename lookup, directory traversal, cluster→sector mapping, mutation helpers, free-space stats, and `fsck` remain available without being required for CoolFS boot. |
| **VFS** | CoolFS-root path routing, `/FAT` legacy routing, CoolFS read/write/execute permission enforcement, task-local cwd resolution, and task-local fd tables (16 slots, with explicit 0/1/2 mappings for child stdio) backed by shared file/pipe/shmem objects. `vfs_open` reads whole files into heap buffers after access checks and drops them if pressure admission fails; `vfs_open_write` buffers writable file descriptors and commits through safe CoolFS writes on close/exit; `vfs_read_fd_at` gives `mmap_file` stable read-at access to regular read-only fds without advancing offsets; `vfs_pipe` allocates a 512-byte kernel ring buffer only when the heap reserve allows it; `vfs_read_blocking` blocks tasks on empty pipes and wakes them on write/EOF; `ipc` and `spawn_fds_args` selectively inherit pipe/file fds into child processes; `vfs_shmem_create`/`vfs_shmem_map` manage a shared memory region pool indexed by ID. |
| **Networking** | Legacy PCI virtio-net driver for QEMU user networking, polling RX/TX virtqueues, Ethernet framing, ARP cache, IPv4, ICMP echo, UDP DNS queries, minimal TCP client sockets, userspace socket syscalls, HTTP/1.1, and verified TLS 1.3 HTTPS for the native browser/terminal path. |
| **Kernel services** | Persistent kernel log buffer flushed to `/LOGS/KERNEL.TXT`, crash-screen log tail, sysreport generation to `/LOGS/SYSREPORT.TXT`, central device registry for PCI/USB/system devices, signed installable package/app manifests with payload ownership and file associations, networking status, ACPI power-control status foundation, a credentialed durable service supervisor with dependency metadata, persisted `/CONFIG/SERVICES.CFG` desired state, `/LOGS/SERVICES.TXT` restart history, backoff, recovery diagnostics under service uid/gid 200, browser engine port readiness under `/CONFIG/BROWSER-ENGINE.CFG`, and boot-health state under `/BOOT` for last-known-good validation. |
| **Updates / rollback** | Ed25519-signed staged system update manifests under `/UPDATES/STAGED`, per-payload SHA-256 verification, public-key trust checks against `/CONFIG/UPDATE-KEYS.TXT`, multiple trusted/revoked/expired key states, anti-rollback version checks, payload snapshots under `/UPDATES/SNAPSHOTS/LAST`, update journals in `/LOGS/UPDATE.TXT`, service-aware apply/rollback operations, recovery rollback integration, and automatic rollback when a pending update fails boot validation. |
| **Packages** | Ed25519-signed package archives under `/Packages` with detached `<package>.sig` files, public package trust keys under `/CONFIG/PACKAGE-KEYS.TXT`, version/dependency checks, payload tables with SHA-256 and mode metadata, per-install owner records under `/APPS/<command>/OWNER.TXT`, package history under `/LOGS/PACKAGES.TXT`, transaction state under `/LOGS/PACKAGE-TXN.TXT`, verified `pkg install\|run\|repair`, and recovery/sysreport package trust diagnostics. |
| **Browser engine port** | Phase 71 selects WPE WebKit as the modern-browser target while keeping the native browser as fallback. Phase 77 marks threading/TLS/pthread support ready and moves dynamic linking/JIT execmem/file-mmap from missing to partial with ABI v14 `mmap_file` and W^X `mprotect`, `/lib` shared-object placement, read-only file-backed text, `DT_NEEDED` dependency loading, cross-object dynsym resolution, and ELF TLS records. `src/browser_engine.rs` defines port ABI v1, runtime requirement diagnostics, backend readiness probing through `/SYSTEM/BROWSER-ENGINE/WPE.READY`, Terminal `engine` commands, `browser://engine`, Recovery/Diagnostics/Sysreport lines, futex/TLS/dynamic-link/file-mmap telemetry, and SDK docs for the host contract. |
| **Applications** | Terminal, System Monitor, Text Viewer, Color Picker, File Manager, Web Browser, ring-3 Notes, Text Editor, Trash Bin, Screenshot, Process Demo, and GUI Demo. Text-file opens route into `/bin/editor <path>` with kernel viewer fallback, while File Manager exposes explicit Open With Editor/Viewer actions. |
| **Disk image** | `disk-image/src/fs_image.rs` builds `fs.img` as a 64 MiB raw OS disk: native CoolFS starts at LBA 0 with root-owned system paths, `/lib` shared objects including `/lib/libphase75.so`, `/lib/libphase76dep.so`, and `/lib/libphase76main.so`, user-owned writable paths including `/TMP`, executable `/bin` ELFs including `/bin/threaddemo`, `/bin/tlsdemo`, `/bin/pthreaddemo`, `/bin/mmapdemo`, and `/bin/lddemo`, `/RECOVERY` boot/repair docs, `/SDK` devkit docs/templates including `/SDK/BROWSER_ENGINE_PORT.TXT`, `/CONFIG/BROWSER-ENGINE.CFG`, `/SYSTEM/BROWSER-ENGINE`, `/Packages/guidemo.pkg`, `/Packages/guidemo.elf`, and `/Documents/package-demo.p25`; an optional FAT32 `/FAT` import region starts at 8 MiB. The Makefile can attach this live image as IDE, AHCI/SATA, NVMe, or USB mass storage, the installer can copy it into BIOS/MBR or UEFI/GPT targets, and `build-usb-image` produces a single raw UEFI/GPT image suitable for flashing plus QEMU USB-storage or NVMe boot testing. |

### Applications

| App | How to open | Description |
| :-- | :---------- | :---------- |
| **Terminal** | Start search / right-click | Interactive shell. Type commands, press Enter. |
| **System Monitor** | Right-click | Live CPU vendor, heap usage and pressure state, uptime, service health, scheduler counts, USB/input status, and userspace app lifecycle controls for close, kill, and app path. |
| **Text Viewer** | Right-click | Scrollable "About" doc; `j`/`k` to scroll. |
| **Color Picker** | Right-click | Clickable 16-colour EGA palette grid. |
| **File Manager** | Right-click / desktop icon | Browse and mutate the CoolFS root with breadcrumbs, recursive search, sorting, multi-select, clipboard copy/cut/paste, Trash-backed delete, properties, inline text editing, Open With Editor/Viewer, and ELF launch routing. |
| **Web Browser** | Start search / desktop icon | Native HTTP/HTTPS/local-file browser with address/search bar, redirects, decoded chunked responses, headings/lists/quotes/tables, CSS2-style cascade, box-model, positioning, float, z-index, table/list, parser-repair, external stylesheet/script loading, inline-image cache, subresource metadata hints, a bounded JavaScript/DOM runtime for text/class/value/checked mutations plus event handlers/timers, web-app APIs for storage, cookies, location/history, attributes/styles/classes, and same-origin fetch callbacks, robust raw script/style/head suppression, main-resource content-type routing, a Google/Search compatibility shell, styled text blocks, direct and HTML-sourced inline PNG previews, image metadata/placeholders for JPEG/GIF/WebP, clickable links/forms, session history, visible TLS trust-root status, persistent bookmarks, persistent cookies/storage, `browser://session`, `browser://cache`, `browser://js`, `browser://storage`, `browser://compat`, and `browser://engine`. |
| **Accounts** | Start search / Display Settings Users tab | Admin account management for first-run setup, account creation, role changes, enable/disable, password reset, and deletion. |
| **Trash Bin** | Start search / desktop icon / `exec /bin/trash` | Ring-3 GUI utility that lists deleted items staged in `/Trash` and can permanently empty them. |
| **Screenshot** | Start search / desktop icon / `exec /bin/screenshot` | Ring-3 GUI utility that queues a focused-window PPM capture to `/Pictures`. |
| **Process Demo** | Start search / `exec /bin/procdemo` | Ring-3 process-control proof for spawn, process groups, USR1, STOP/CONT, group TERM, and `waitpid`. |
| **Notes** | Start search / desktop icon / `exec /bin/notes [path]` | Ring-3 scratchpad backed by `/documents/notes.txt` by default, with New, Open, Save, and Save As document flow. |
| **Text Editor** | Start search / desktop icon / `exec /bin/editor [path]` | Ring-3 text editor backed by `/documents/editor.txt` by default, or any absolute file path passed as argv, with New, Open, Save, Save As, and cursor controls. |
| **GUI Demo** | Start search / `exec /bin/guidemo` | First ring-3 windowed app. It opens a compositor window, presents its own pixel buffer, and polls keyboard/mouse/close events through `libcool::gui`. |

### Desktop shortcuts

| Shortcut | Action |
| :------- | :----- |
| **Ctrl+Space** | Open Start-menu search for apps and files. |
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
| `install [status\|reset\|repair\|disks\|plan <device>\|disk <device>\|verify <device>]` | Report, reset, or repair first-boot state, list block devices, preflight an installer target, install to a target disk, or verify an installed target; mutating operations require admin, recovery, or installer context |
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
| `hardware [status]` | Print boot hardware readiness, framebuffer, memory-map, storage, USB, AHCI, and NVMe diagnostics |
| `net` | Print network adapter/protocol status |
| `netproto` | Print ARP/IPv4/ICMP/UDP/TCP protocol status |
| `dns <host>` | Resolve a host through the network DNS API |
| `ping <host>` | Send an ICMP echo request |
| `http <host-or-url> [path]` | Fetch an HTTP response through the kernel network API |
| `https <host-or-url> [path]` | Fetch an HTTPS response through the verified kernel TLS client |
| `browser [url]` | Open the native Web Browser to a URL or `browser://home` |
| `engine [status\|abi\|requirements\|config\|log\|recovery]` | Inspect the WPE WebKit browser-engine port ABI and readiness blockers |
| `power [reboot\|shutdown\|sleep]` | Print or request power-control actions |
| `log` | Flush and print the kernel log tail |
| `logs` | Open the in-terminal log view |
| `profiler` | Print boot/session profiler events |
| `boot [status\|history\|mark-good\|fail-validation <id> <reason>]` | Inspect boot health, mark the current boot as last-known-good, or record a failed validation attempt for recovery testing |
| `services [list\|status <name>\|history\|recovery\|run\|start <name>\|restart <name>\|stop <name>\|fail <name>]` | Inspect and control the durable service supervisor; mutating operations require an admin session |
| `update [status\|verify\|keys\|history\|sign\|sign-as <key>\|stage <path> <text>\|stage-version <path> <version> <text>\|apply\|rollback]` | Stage or sign a trusted system file update, verify manifest/payload integrity and monotonic version, apply it with a pre-update snapshot, inspect the update journal, or roll back |
| `memory` | Print heap pressure, reclaim counters, OOM state, and per-task memory estimates |
| `diagnostics` | Print kernel, profiler, boot-health, hardware, service, update, browser-engine, compositor, heap, memory-pressure, resource-limit, filesystem, VFS, and crash diagnostics |
| `sysreport [write]` | Print the generated system report or write it to `/LOGS/SYSREPORT.TXT` |
| `devkit` | Print SDK paths, ABI version, and userspace template locations |
| `compositor` | Print FPS, frame pacing, frame budget, damage, and cursor overlay telemetry |
| `smoothness` | Alias for compositor pacing/latency telemetry |
| `fsck` | Print CoolFS-root consistency plus optional legacy FAT32 import summary |
| `recovery [repair\|rollback\|firstboot status\|firstboot reset\|firstboot repair\|fsck-on-boot on\|fsck-on-boot off]` | Show recovery status, repair boot/first-boot state, roll back the last update, or toggle boot fsck |
| `coolfs` | Print CoolFS root mount and inode/block usage |
| `df` | Print CoolFS `/` and optional FAT32 `/FAT` used/free/total space |
| `pkg [list\|keys\|history\|transaction\|info <id\|path>\|verify <id\|path>\|install <id\|path>\|remove <id>\|repair <id>\|run <id> [args...]]` | Inspect package trust keys, verify signed package archives or installed owner/payload records, inspect transaction state, install/remove/repair packages, and launch package apps. |
| `pkg [sign <path>\|sign-as <path> <key>\|unsign <path>\|tamper <path>\|tamper-payload <path>\|deps <path> [ids...]\|break <id>\|break-payload <id>\|install-fail <path>]` | Admin/test package trust helpers used by the smoke suite for rotated, unsigned, tampered, dependency, repair, payload-integrity, and rollback flows. |
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

Interactive QEMU windows use Cocoa `zoom-to-fit` by default so the true
1920×1080 guest framebuffer fits below the macOS menu/title bars. They also use
QEMU USB tablet input by default for smoother absolute pointer tracking.
Override with `QEMU_POINTER=mouse make run` to test the relative USB mouse path,
`QEMU_DISPLAY=cocoa make run` for 1:1 pixels, or
`QEMU_DISPLAY=cocoa,full-screen=on make run` for fullscreen.

The phase 46 profile uses the same smooth tablet pointer path with the
adaptive-refresh defaults:

```bash
make run-smooth
```

For virtio networking in QEMU:

```bash
make run-net
```

To boot the QEMU disk installer with a blank writable target attached as
`ide1-master`:

```bash
make run-installer
```

After installing, boot the produced target disk by itself:

```bash
make run-installed
```

The UEFI/GPT path is available in parallel under QEMU OVMF:

```bash
make run-uefi
make run-uefi-installer
make run-uefi-installed
```

The AHCI/SATA UEFI path exercises the same boot and installer flow without IDE
storage:

```bash
make run-uefi-ahci
make run-uefi-ahci-installer
make run-uefi-ahci-installed
```

The NVMe UEFI path boots the same GPT image through QEMU PCIe storage and can
use a writable NVMe installer target:

```bash
make run-uefi-nvme
make run-uefi-nvme-installer
make run-uefi-nvme-installed
```

To build a single USB-flashable raw UEFI/GPT image:

```bash
make build-usb-image
make build-usb-safe-image
```

To boot that image as an actual QEMU xHCI USB mass-storage disk:

```bash
make run-uefi-usb-storage
make run-uefi-usb-storage-safe
```

`coolos-usb.img` keeps the normal 1920×1080 framebuffer request. The safe
variant `coolos-usb-safe.img` requests 1024×768 and enters safe mode, which
prints `[boot] safe mode`, avoids nonessential boot-time work after the root
mount, and is intended for physical PCs that fail to display the normal image.

The UEFI helpers use Homebrew QEMU's EDK2 firmware by default and can be
overridden with `QEMU_EFI_CODE=/path/to/edk2-x86_64-code.fd`.

To try physical USB boot, flash one of the raw images to a USB drive with
Secure Boot disabled. On macOS, replace `/dev/rdiskN` with the removable disk:

```bash
diskutil list
diskutil unmountDisk /dev/diskN
sudo dd if=target/x86_64-unknown-none/release/coolos-usb.img of=/dev/rdiskN bs=4m status=progress
diskutil eject /dev/diskN
```

On Linux, replace `/dev/sdX` with the removable disk:

```bash
sudo dd if=target/x86_64-unknown-none/release/coolos-usb.img of=/dev/sdX bs=4M status=progress conv=fsync
sync
```

If the normal image does not display or reach first boot, flash
`coolos-usb-safe.img` instead. Expected useful boot lines include `FB ...`,
`MSC usb0`, `[storage] root device=usb0 layout=gpt-coolfs`, and `[boot] first
boot ready`. From the Terminal, `hardware`, `devices`, `diagnostics`, and
`sysreport` show framebuffer, PCI storage/input, USB, root-discovery, and
probe-failure details for troubleshooting.

The build process compiles the kernel ELF, compiles the userspace ELF binaries
in `userspace/hello/`, wraps the kernel into BIOS-bootable `bios.img` and
UEFI-bootable `uefi.img` images, and builds `fs.img` with the userspace
binaries embedded into the native CoolFS `/bin`.

If using `QEMU_POINTER=mouse`, click inside the QEMU window to capture mouse
input. Press **Ctrl+Alt+G** to release it.

---

## Architecture

```
disk-image/
  src/main.rs      Host tool — wraps kernel ELF into bios.img via bootloader 0.11
  src/fs_image.rs  Host tool — builds fs.img (64 MiB raw OS disk) with native
                    CoolFS at LBA 0 plus an optional /FAT import region;
                    installed targets store that image in an MBR partition:
                    /bin, /CONFIG, /APPS, /Documents, /Packages, /Pictures,
                    /Desktop, /Downloads, /Trash, /LOGS, /SDK, /SYSTEM,
                    and process-control demos
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
  allocator.rs     Heap allocator (linked_list_allocator, 96 MiB)
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
  boot_health.rs   Boot validation, last-known-good state, and auto rollback
  services.rs      Durable service supervisor with dependency/backoff policy and /CONFIG + /LOGS state
  browser_engine.rs WPE WebKit port ABI/readiness model and diagnostics
  update_crypto.rs SHA-256 and Ed25519 helpers for update/package trust checks
  updates.rs       Signed staged system updates, snapshots, journals, and rollback
  packages.rs      Built-in package registry plus signed payload archive transactions
  notifications.rs Desktop notification queue used by USB/task/filesystem events
  app_lifecycle.rs Persistent app recents/settings plus runtime userspace app ownership
  clipboard.rs     Shared text/path clipboard service
  device_registry.rs Central PCI/USB/system device table
  hardware.rs      Boot hardware readiness report for framebuffer, memory, storage, and USB/PCI probes
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
    terminal/      TerminalApp — keyboard input, shell commands, text render
    file_manager/  FileManagerApp — Explorer-style file browsing and operations
    system_monitor.rs SysMonApp — live CPU/heap/uptime/scheduler/app lifecycle controls
    display_settings.rs DisplaySettingsApp — display/personalization/security settings,
                   including the Users/Accounts panel
    text_viewer.rs TextViewerApp — scrollable static text
    color_picker.rs ColorPickerApp — clickable EGA palette swatches
    user_gui.rs    UserGuiApp — compositor-owned window/surface/event queue for ring-3 apps
    utilities.rs   UtilityApp — Trash Bin, Screenshot, Notes, and Text Editor
userspace/
  libcool/         no_std userspace SDK — entry, argv, syscalls, files, pipes,
                   signals, process groups, threads/futexes/TLS, pthread-style primitives,
                   POSIX pthread/libc shims, dynlink, evented poll, TTY control, mmap/mprotect,
                   shmem, events, networking, filesystem utilities, time, GUI, print!/println!
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
    src/bin/threaddemo.rs `/bin/threaddemo` — userspace thread/futex demo
    src/bin/tlsdemo.rs `/bin/tlsdemo` — FS-base TLS + pthread-style sync demo
    src/bin/pthreaddemo.rs `/bin/pthreaddemo` — POSIX pthread/libc shim demo
    src/bin/lddemo.rs `/bin/lddemo` — ET_DYN /lib loader and function-call demo
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
requesting 1920×1080. The bootloader negotiates a VBE mode with QEMU's SeaBIOS
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
at compile time by a host-side `fs-image` tool and attached to QEMU as either
the IDE primary-bus slave (`if=ide,index=1`) or an AHCI/SATA disk for live/dev
boots. CoolFS starts directly at LBA 0 in that image and owns the `/`
namespace; self-booting installed disks resolve the same root from an MBR or
GPT partition. The legacy FAT32 import region
starts at 8 MiB and remains available at `/FAT` without being part of the boot
path. The IDE driver targets ports
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
state through `services`. Phase 64 extends that supervisor with dependency
metadata, persisted desired state in `/CONFIG/SERVICES.CFG`, restart snapshots
in `/LOGS/SERVICES.TXT`, restart backoff, and recovery/sysreport health lines.
Phase 65 layers update safety on top of that: staged updates can declare
affected services, apply stops/restarts them around safe file writes, and
rollback uses the captured snapshot manifest from `/UPDATES/SNAPSHOTS/LAST`.
Phase 66 adds `/BOOT/STATE.TXT`, `/BOOT/HISTORY.TXT`, and
`/BOOT/LAST-GOOD.TXT` so an applied update is only accepted after the next boot
reaches the desktop-ready checkpoint; otherwise the following boot restores the
last update snapshot automatically. Phase 67 verifies staged update manifests
and payload hashes before any snapshot/service mutation, records the trusted key
and algorithm in the applied manifest, and exposes update trust state in
Recovery, Diagnostics, and Sysreport. Phase 68 moves that trust model to
Ed25519 public keys, records update OS/version metadata, accepts rotated trusted
keys, and rejects revoked, expired, unknown, or non-monotonic signed updates.
Phase 69 adds package-level Ed25519 trust: package archives are verified through
detached `.sig` sidecars, installs record owner/source/signature metadata under
`/APPS`, dependencies and package downgrades are refused, and recovery/sysreport
surface installed package trust. Phase 70 adds real package payloads and
transactions: archive manifests list payload source/target/hash/mode entries,
install copies those files into protected targets such as `/bin/pkgdemo`, owner
records pin installed payload hashes, `pkg verify` detects payload tampering,
`pkg repair` restores files from the trusted source archive, `pkg remove`
deletes owned payloads, and rollback restores pre-transaction file state.
Phase 71 adds the browser engine port surface for the WPE WebKit track:
`engine` reports ABI/readiness, `browser://engine` exposes the GUI view, and
Recovery/Sysreport include the same target/fallback state.
Phase 72 adds the thread/futex OS substrate that a hosted WebKit-class runtime
expects: `thread_spawn` creates ring-3 tasks sharing the caller's PML4 with
separate user/kernel stacks, `futex_wait` and `futex_wake` provide blocking
word-address waits, shared address spaces are freed only after the last sibling
is reaped, `libcool::thread` exposes the ABI, `/bin/threaddemo` covers two
workers plus futex wake/join, and diagnostics/sysreport include futex counters
and thread-stack capacity. Phase 73 adds FS-base TLS as scheduler task state,
ABI v12 `thread_tls_set`, `thread_tls_get`, and `thread_spawn_tls`, libcool
TLS blocks/keys plus pthread-style mutex/condvar/once helpers, `/bin/tlsdemo`,
and TLS thread counts in diagnostics/resource reporting. Phase 74 adds
`libcool::posix` and `libcool::libc` pthread/libc shims over that substrate,
including `pthread_create`/join/exit/self, mutex/condvar/once/key APIs,
per-thread `errno`, `gettid`, `sched_yield`, `nanosleep`, `/bin/pthreaddemo`,
and `make smoke-phase74-pthread-libc`. Phase 75 adds ABI v13 `mprotect`,
`libcool::dynlink`, `/lib/libphase75.so`, `/bin/lddemo`, W^X executable
shared-object mappings, RELA relocations, dynsym lookup, init arrays, and
`make smoke-phase75-dynlink`. Phase 76 adds `dynlink::load_with_deps`,
`/lib/libphase76dep.so`, `/lib/libphase76main.so`, bounded `DT_NEEDED`
loading, soname lookup, cross-object symbol resolution, ELF TLS template/TLS
relocation handling, dependency-order init arrays, and
`make smoke-phase76-dynlink-deps`. Phase 77 adds ABI v14
`mmap_file(desc_ptr)`, `libcool::memory::mmap_file`, `io::open_flags`,
read-at VFS mapping support, read-only file-backed `/lib` text mappings,
`/bin/mmapdemo`, and `make smoke-phase77-file-mmap`.

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

**Installer and first boot (Phase 80).** Interactive boots now show a modern
first-boot setup card instead of asking users to sign in with the default
handoff. The setup flow collects an owner name, password confirmation, and
optional device name, calls the same `complete_first_run_admin` path as
Terminal `setup`, persists `/CONFIG/FIRSTBOOT.CFG`, and unlocks directly into
the new owner session. `install status` reports the installer state, while
existing automated smokes use a QEMU fw_cfg smoke marker to keep legacy
regression paths deterministic; `make smoke-phase80-firstboot` exercises the
real wizard and persistence flow.

**First-boot recovery and reset (Phase 81).** Admin sessions can use
`install reset` to restore the default development handoff and make the next
boot show the setup wizard again, or `install repair` to reconcile
`/CONFIG/FIRSTBOOT.CFG` with the user database. The recovery surface mirrors
that as `recovery firstboot status|reset|repair`, so a broken owner-account
state can be recovered without hand-editing the disk image. Normal boots also
repair stale completed/required state before the greeter is built.

**QEMU disk installer (Phase 82).** Installer mode is triggered with
`opt/coolos/installer=1` and opens a graphical installer card instead of the
first-boot wizard. `install disks` reports `ide0-master`, `ide0-slave`,
`ide1-master`, and `ide1-slave`; `install disk ide1-master` copies the live
CoolFS root image sector-by-sector to a blank secondary IDE target, flushes it,
and verifies the copy.

**Self-booting installed disk (Phase 83).** The installer now writes a complete
BIOS/MBR target: it copies the existing bootloader/stage/FAT boot area, adds a
private `0xc0` CoolFS root partition aligned after that boot area, copies the
live root image into the partition, and verifies both boot and root contents.
The resulting target boots by itself as `ide0-master` and enters the normal
first-boot owner setup flow.

**Installer v2 safety flow (Phase 84).** Installer mode now opens an
interactive target-selection flow instead of a static command card. The
installer lists all block devices with role, protection, layout state, and
installability reason; `install plan <device>` performs the same non-mutating
preflight from Terminal or recovery. The graphical flow requires reviewing the
target and typing the target device name before writing, then shows copy,
flush, and verify progress before the installed disk is rebooted on its own.

**UEFI/GPT installer foundation (Phase 85).** coolOS now builds `uefi.img`
alongside `bios.img` and can boot the same kernel/root image under QEMU OVMF.
Root discovery understands raw CoolFS, BIOS/MBR `0xc0` CoolFS partitions, and
the coolOS GPT CoolFS partition type
`B9C7C6F0-9D2E-4F6A-9A3D-434F4F4C4653`. When the installer is launched from a
UEFI source boot disk, it writes an ESP plus CoolFS GPT target, verifies both
partitions, and the installed target boots alone under OVMF into the normal
first-boot owner setup flow. BIOS/MBR remains the default developer path.

**AHCI/SATA and USB image foundation (Phase 86).** The storage layer now
enumerates QEMU AHCI disks as `sata0`, `sata1`, `sata2`, and so on. The UEFI
installer can boot from AHCI, refuse protected source boot/root disks, install
to a SATA target, and boot the installed target alone under OVMF. `make
build-usb-image` builds `coolos-usb.img`, a raw UEFI/GPT image with ESP plus
CoolFS root.

**Runtime USB mass-storage root boot (Phase 87).** The xHCI stack now detects
USB Mass Storage Class devices, configures bulk endpoints, speaks Bulk-Only
Transport with the SCSI commands needed for block I/O, and exposes USB disks as
`usb0`, `usb1`, and so on. The generic storage layer can discover the GPT
CoolFS root on `usb0`, so `make run-uefi-usb-storage` boots `coolos-usb.img`
through QEMU `usb-storage` without IDE or AHCI root disks.

**NVMe/PCIe storage root boot (Phase 88).** The storage layer now detects QEMU
NVMe controllers, exposes namespace 1 as `nvme0n1`, `nvme1n1`, and so on, and
uses the same raw/MBR/GPT CoolFS discovery path as IDE, AHCI, and USB. `make
run-uefi-nvme` boots `coolos-usb.img` as a QEMU NVMe disk, and installer
commands can plan, install, and verify writable NVMe targets. Secure Boot,
UASP boot support and broad real-hardware USB/NVMe variance remain future work; guarded
physical-machine installation from a USB source is covered by Phase 90.

**Bare-metal USB boot readiness (Phase 89).** The boot path now records a
compact hardware readiness report covering framebuffer/GOP details, memory-map
summary, storage root layout, block devices, and AHCI/NVMe/USB probe status.
The report appears in boot logs and through Terminal `hardware`, `devices`,
`diagnostics`, and `sysreport`. `make build-usb-safe-image` produces
`coolos-usb-safe.img`, a conservative 1024×768 UEFI/GPT USB image that enters
safe mode for physical machines that cannot handle the normal 1080p path.
Secure Boot, UASP, wider hardware-driver coverage, and physical installs beyond
the guarded Phase 90 UEFI/GPT path remain future work.

**Physical disk installer guardrails (Phase 90).** A system booted from the
UEFI/GPT USB image can now plan and run a guarded internal install to AHCI/SATA
or NVMe storage. `install disks` and `install plan <device>` show bus, size,
layout state, source/root/protected role, and destructive refusal reason;
`install physical <device>` is an explicit alias for installing from the live
USB root to an internal UEFI/GPT target. The graphical installer defaults to a
safe internal disk when available, shows the USB source and GPT layout on the
review screen, requires typing the exact target device name, copies ESP and
CoolFS partitions, flushes, and verifies before completion. Secure Boot, UASP
boot support, MBR physical installs, and physical disk partitioning beyond this
UEFI/GPT writer remain future work.

**Bare-metal hardware readiness (Phase 91).** USB diagnostics now classify hub
and UASP devices instead of hiding them as unknown devices. UASP is reported as
unsupported with BOT fallback required, while BOT USB mass storage remains the
supported USB root path. `hardware`, `devices`, and `sysreport` include
per-device root-scan reasons plus a physical-installer preflight verdict
covering the live USB source, ESP/root availability, and internal SATA/NVMe
candidate targets. `make smoke-phase91-hardware-readiness` exercises USB
topology diagnostics and installer preflight under QEMU.

**Secure Boot test-key foundation (Phase 92).** The repo now builds Secure
Boot foundation artifacts under `target/secure-boot`, plus `uefi-secure.img`
and `coolos-usb-secure.img`. Secure images rebuild the vendored UEFI loader
with the current kernel SHA-256 embedded, and the loader verifies
`kernel-x86_64` before handing off. `make run-uefi-secure`,
`make run-uefi-usb-storage-secure`, and `make smoke-phase92-secure-boot` use
QEMU secure OVMF firmware and report `secure_boot ... kernel=verified` through
boot logs, `hardware`, and `sysreport`.

**Enforced Secure Boot test-key chain (Phase 93).** Secure Boot artifacts now
include local PK/KEK/db certs, ESL/auth enrollment material, and an enrolled
writable OVMF vars image with Secure Boot enabled. `make build-uefi-secure`
signs the vendored UEFI loader as `EFI/BOOT/BOOTX64.EFI` using `sbsign`/`sbverify`
when available, or `osslsigncode` on macOS. `make verify-secure-boot-artifacts`
checks the PE/COFF signature plus enrolled vars, and
`make smoke-phase93-secure-boot` verifies the signed USB boot path reports
`loader=signed-pe kernel=verified vars=enrolled enforcement=on`. The same smoke
also confirms OVMF rejects unsigned and tampered loaders, while the signed
loader rejects a tampered kernel digest before handoff. Generated keys stay
under `target/secure-boot`; Microsoft/shim compatibility, production keys, and
signed update rollout remain later work.

**Real-PC Secure Boot enrollment diagnostics (Phase 94).** `make
build-secure-boot-enrollment` creates a public enrollment bundle under
`target/secure-boot/enroll/` with DER certs, ESL/auth files, fingerprints,
`SHA256SUMS`, and `README.TXT`; verification fails if private key material is
copied there. `make build-usb-secure-image` embeds that bundle into the secure
USB ESP at `/EFI/COOLOS/ENROLL/` and writes `/EFI/COOLOS/SECUREBOOT.TXT` with
the signed-loader fingerprint, db cert fingerprint, kernel SHA-256, and build
mode. The signed UEFI loader reports firmware `SecureBoot`/`SetupMode` state to
the kernel, so `hardware` and `sysreport` can distinguish QEMU-enforced,
firmware-secureboot-on, firmware-setup-mode, signed-loader, kernel-verified,
and unknown/unsigned boots. `make smoke-phase94-secure-boot-enrollment`
validates the embedded ESP bundle and diagnostics under QEMU OVMF.

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
`/SDK/APP_TEMPLATE.RS`, `/SDK/PACKAGE_TEMPLATE.PKG`, and the Phase 71
`/SDK/BROWSER_ENGINE_PORT.TXT` WebKit port contract.

**Compositor latency and smoothness (Phase 45).** The timer IRQ now requests a
passive frame tick instead of forcing a full repaint every interrupt; normal
events still request explicit full repaint. Mouse packets distinguish plain
motion from button/scroll/drag work, so simple cursor movement can restore the
old cursor rectangle from the clean shadow scene and draw the new cursor
directly to the hardware framebuffer without recomposing windows or boosting
full-frame pacing. The idle loop polls USB input before service/deferred/network
maintenance and limits deferred work to a smaller budget so input-to-pixel
latency wins. `compositor` and `smoothness` show full-frame count,
cursor-fast count, passive frame cadence, damage rows/pixels, cursor overlay
pixel counts, and the active USB pointer kind.

**Adaptive high refresh (Phase 46).** The desktop keeps the Phase 45 idle cadence
at 36 Hz, but clicks, drags, scrolls, app updates, and other explicit repaint
work extend a 750 ms active boost window that paces full frames at 144 Hz. Pure
cursor movement stays on the cursor-overlay path so a 1920×1080 desktop does not
pay for unnecessary full recomposes. Explicit repaints mark the pacing clock so
the compositor avoids immediate duplicate passive frames after an input-driven
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

**Browser rendering phases (49-61).** The native Browser now has a more explicit
HTML/CSS rendering path: style blocks and inline styles are parsed into bounded
rules for tag, class, id, and simple compound selectors; computed style drives
hidden content, alignment, indentation, text color, backgrounds, preformatted
text, image width/height hints, and CSS2 box-model fields for margins, padding,
borders, fixed/percentage widths, bounded heights, relative/absolute/fixed
positioning, sticky-style offsets, top/right/bottom/left offsets, floats,
z-index, and list-style hints. The Browser layout pass now keeps content boxes
separate from painted border/background boxes, wraps text inside percentage-width boxes,
reserves flow width beside bounded left/right floats, offsets positioned boxes
without losing normal-flow sizing, paints and hit-tests by z order, and uses the
laid-out box rects for link/control hit testing. HTML parsing now repairs common
implied closes for paragraphs, list items, rows, and cells so malformed desktop
pages do not bleed styles or table state. Table rows use bounded content-aware
column sizing, and CSS list markers can render square/circle/decimal/none
variants. External `<link rel="stylesheet">` resources now load through a
bounded local/HTTP/HTTPS subresource path and feed the existing cascade before
rendering. Image handling keeps the PNG decoder bounded while recognizing
PNG/JPEG/GIF/WebP dimensions for previews/placeholders; HTML-sourced images now
share a small in-memory cache with stylesheet resources, and `browser://cache`
shows URL, type, size, age, last-use, and hit metadata. Forms render text,
search, and email fields, checkboxes, radios, selects, textareas, and buttons,
preserve checked/default values, and now bind rendered controls to a bounded
DOM/form-state model. Clicks and keyboard focus edit live control values,
checkbox/radio/select/reset controls update state across reflow, GET forms
submit encoded live values, and POST forms now send
`application/x-www-form-urlencoded` request bodies through the same HTTP/TLS
response path used by normal page loads. Browser GET/POST page loads now share
a persistent cookie jar in `/CONFIG/BROWSER.COOKIES`; `Set-Cookie` handling
covers Domain, Path, Secure, and Max-Age deletion, and `browser://session`
shows redacted session state. Normal reloads reuse cached subresources while
uppercase `R` performs a hard subresource reload. The Browser now executes a
bounded JavaScript subset after DOM construction: same-origin external scripts
and inline scripts can mutate `textContent`, `className`, form `value`,
`checked`, and `disabled` state, register `click`/`change`/`submit` handlers
through inline attributes or `addEventListener`, and run bounded timer callbacks.
Script mutations serialize back through the DOM state before reflow. Phase 60
adds a bounded web-app API layer: persistent per-origin `localStorage` in
`/CONFIG/BROWSER.STORAGE`, per-document `sessionStorage`, JS `document.cookie`
read/write through the existing cookie jar, `location.href` / `location.search`
and `history.pushState` / `replaceState` hooks, `querySelectorAll` indexing,
`classList.add/remove/toggle`, `setAttribute` / `getAttribute`,
`removeAttribute`, `style.<property>` mutations, simple script variables, and
same-origin `fetch()` text callbacks for local/HTTP/HTTPS resources.
`browser://js` reports script, handler, timer, mutation, storage, cookie,
fetch, navigation, and error counts for the last loaded page, while
`browser://storage` lists persisted localStorage keys. Phase 61 hardens modern
page handling around the actual Google failure mode: raw text elements such as
`<head>`, `<script>`, `<style>`, `<template>`, SVG/canvas/media/embed blocks,
and no-script fallbacks are skipped by jumping to their matching closing tag,
so JavaScript comparisons and HTML-looking strings no longer leak into rendered
page text or DOM serialization. Main responses are routed by content type:
HTML enters the renderer, images enter the preview path, and JavaScript/CSS/JSON
or other non-HTML resources show a source/resource diagnostics page instead of
being interpreted as markup. Google home/search pages now activate a bounded
compatibility shell with a native search form that submits real Google GET
queries, and `browser://compat` explains whether the last main resource used
native rendering, source diagnostics, or the Google compatibility profile. This
is a compatibility foundation, not a full Chromium/WebKit-class engine.
The Browser can also be
launched from Terminal with
`browser [url]`. The fixture targets from `make smoke-phase49-browser-engine`
through `make smoke-phase61-browser-compat` boot
pages or internal Browser diagnostics; the Phase 54 target submits the fixture
form over HTTPS, Phase 55 opens the session-state surface, Phase 56 renders the
CSS box-model fixture, and Phase 57 renders the positioning/floats/parser
fixture, Phase 58 renders the external CSS/image/cache fixture, and Phase 59
renders the script/DOM mutation fixture, and Phase 60 renders the storage,
cookie, history/location, attribute/style/class, and fetch fixture. Phase 61
renders the Google/Search compatibility fixture and verifies that Closure script
source does not appear as page text. Phase 62 adds scheduler, VMM, VFS, shared
memory, and socket quota diagnostics, folds the resource-limit checks into the
kernel selftest path, and adds `make smoke-phase62-resource-limits` to boot the
diagnostics surface and verify the resource-limit report. Phase 63 adds the
`memory` command, memory-pressure diagnostics, per-task memory estimates,
clean-cache/browser-cache trimming, allocation admission checks, and
`make smoke-phase63-memory-pressure`. Phase 64 adds durable service supervisor
coverage with `make smoke-phase64-services`, including admin-gated mutations,
persisted service config, restart history, recovery lines, and sysreport output.
Phase 65 adds `update status|stage|apply|history|rollback`, Recovery
`rollback`, update diagnostics in sysreport, and
`make smoke-phase65-update-rollback`. Phase 66 adds `boot status|history`,
last-known-good health state in Recovery/Diagnostics/Sysreport, automatic
rollback after a failed pending-update validation, and
`make smoke-phase66-boot-health` with a writable two-boot recovery image.
Phase 67 adds `update verify|keys|sign`, signed manifest and payload-hash
checks, trust diagnostics in Recovery/Sysreport, and
`make smoke-phase67-update-trust` for valid, tampered, unsigned, and rollback
flows. Phase 68 adds Ed25519 public-key verification, key rotation/revocation
metadata, anti-rollback version checks, `update sign-as` and
`update stage-version` diagnostics, and `make smoke-phase68-update-keys` for
active, rotated, revoked, expired, unknown, and downgrade cases. Phase 69 adds
`pkg keys|verify|info|repair|history`, package owner records, package trust
diagnostics, and `make smoke-phase69-package-trust` for valid install, rotated
signing keys, unsigned/tampered archives, revoked/expired/unknown keys,
dependency refusal, repair, recovery, and sysreport coverage.
Phase 70 adds `pkg transaction`, `pkg tamper-payload`, `pkg break-payload`, and
`pkg install-fail`, plus `make smoke-phase70-package-payloads` for real payload
install/run/remove, installed payload tamper repair, source payload hash refusal,
and injected install rollback. Phase 71 adds `engine`, `engine abi`,
`engine requirements`, `engine config`, `engine log`, `browser://engine`,
browser-engine Recovery/Diagnostics/Sysreport lines, and
`make smoke-phase71-browser-engine-port` to verify the WPE WebKit port ABI,
SDK documentation, backend probe, and native fallback state. Phase 72 adds
`/bin/threaddemo`, ABI v11 `thread_spawn`/`futex_wait`/`futex_wake`,
futex diagnostics, and `make smoke-phase72-threads-futex`. Phase 73 adds
`/bin/tlsdemo`, ABI v12 `thread_tls_set`/`thread_tls_get`/`thread_spawn_tls`,
libcool TLS/pthread-style helpers, `req.threads-futex=ready`, and
`make smoke-phase73-tls-pthread`. Phase 74 adds `/bin/pthreaddemo`,
POSIX-shaped `pthread_*`, per-thread `errno`, `gettid`, `sched_yield`, and
`nanosleep` wrappers in `libcool::posix`/`libcool::libc`, plus
`make smoke-phase74-pthread-libc`. Phase 75 adds ABI v13 `mprotect`,
`libcool::dynlink`, `/lib/libphase75.so`, `/bin/lddemo`, partial
`req.dynamic-linker` readiness, and `make smoke-phase75-dynlink`. Phase 76
adds `dynlink::load_with_deps`, `DT_NEEDED` dependency graphs,
cross-object dynsym resolution, ELF TLS records, `/lib/libphase76*.so`, and
`make smoke-phase76-dynlink-deps`. Phase 77 adds `mmap_file`, POSIX-shaped
`open_flags`, read-only file-backed ET_DYN segments, file-backed page
diagnostics, `/bin/mmapdemo`, and `make smoke-phase77-file-mmap`. Phase 80
adds the graphical first-boot installer, `/CONFIG/FIRSTBOOT.CFG`,
`install status`, and `make smoke-phase80-firstboot` for wizard, completion,
and persistence coverage. Phase 81 adds `install reset|repair`, recovery
first-boot reset/repair commands, boot-time state reconciliation, and
`make smoke-phase81-firstboot-recovery`. Phase 82 adds QEMU installer mode,
named IDE disk discovery, sector-copy install/verify commands, `make
run-installer`, and `make smoke-phase82-installer`. Phase 83 makes the
installed target self-booting, adds standalone `--boot-disk` smoke support,
`make run-installed`, and `make smoke-phase83-self-booting-installer`. Phase
84 adds installer target preflight, protected-disk refusal reporting, graphical
review/confirmation/progress states, and `make smoke-phase84-installer-v2`.
Phase 85 adds `uefi.img`, QEMU OVMF helpers, GPT CoolFS partition discovery,
UEFI/GPT install/verify support, and `make smoke-phase85-uefi-gpt`. Phase 86
adds AHCI/SATA block devices, AHCI installer targets, `make
smoke-phase86-ahci-storage`, and `make build-usb-image`. Phase 87 adds USB MSC
Bulk-Only Transport, `usb*` block devices, `make run-uefi-usb-storage`, and
`make smoke-phase87-usb-storage-root`. Phase 88 adds QEMU NVMe root boot,
`nvme*n1` block devices, NVMe installer targets, `make run-uefi-nvme`, and
`make smoke-phase88-nvme-storage`. Phase 89 adds boot hardware diagnostics,
`hardware [status]`, `make build-usb-safe-image`,
`make run-uefi-usb-storage-safe`, and `make smoke-phase89-baremetal-readiness`.
Phase 90 adds guarded physical-install simulation with USB source protection,
internal AHCI/NVMe candidate reporting, `install physical <device>`, `make
run-physical-installer-sim`, and `make smoke-phase90-physical-installer`.
Phase 91 adds USB hub/UASP diagnostics, detailed storage root-scan output,
installer hardware preflight reporting, and `make smoke-phase91-hardware-readiness`.
Phase 92 adds Secure Boot foundation artifacts, `make build-uefi-secure`,
`make build-usb-secure-image`, `make run-uefi-secure`,
`make run-uefi-usb-storage-secure`, and `make smoke-phase92-secure-boot`.
Phase 93 adds enforced local-key OVMF variables, PE/COFF loader signing,
`make verify-secure-boot-artifacts`, `make tamper-secure-boot-artifacts`, and
`make smoke-phase93-secure-boot`. Phase 94 adds
`make build-secure-boot-enrollment`, embeds public enrollment material into
`coolos-usb-secure.img`, passes firmware Secure Boot state from the loader to
the kernel, and adds `make smoke-phase94-secure-boot-enrollment`.

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
| 6 | High-resolution framebuffer via `bootloader 0.11` (1920×1080) | **Done** |
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
| 57 | Browser layout and parser fidelity — positioning, floats, z-index, list/table layout, and implied-close repair | **Done** |
| 58 | Browser subresources and cache — external CSS, cached images, metadata placeholders, and `browser://cache` | **Done** |
| 59 | Browser JavaScript and DOM runtime — bounded script loading, handlers, timers, mutations, and `browser://js` | **Done** |
| 60 | Browser web-app APIs — storage, cookies, location/history, class/attribute/style DOM APIs, fetch, and `browser://storage` | **Done** |
| 61 | Browser modern-page compatibility — raw script suppression, content-type routing, Google/Search shell, and `browser://compat` | **Done** |
| 62 | Kernel resource limits and cleanup — task/address-space/fd/shmem/socket caps plus diagnostics and smoke coverage | **Done** |
| 63 | Memory pressure and OOM recovery — heap pressure states, cache trimming, per-task estimates, admission checks, and OOM reclaim | **Done** |
| 64 | Persistent service supervision and recovery — durable desired state, dependency/backoff policy, restart history, and degraded diagnostics | **Done** |
| 65 | System update, snapshot, and rollback — staged manifests, rollback snapshots, service-aware apply, update journal, and recovery rollback | **Done** |
| 66 | Boot health and last-known-good rollback — pending-update validation, boot history, and automatic snapshot restore | **Done** |
| 67 | Signed updates and integrity verification — per-payload hashes, signed manifest trust diagnostics, and apply refusal for tampered updates | **Done** |
| 68 | Update key rotation and anti-rollback — Ed25519 public-key signatures, trusted/revoked/expired key states, and monotonic update versions | **Done** |
| 69 | Package trust and repair — Ed25519-signed package archives, owner records, dependencies, and recovery/sysreport diagnostics | **Done** |
| 70 | Package payloads and transactional installs — real file payload copying, owner hash verification, repair/remove payload handling, and rollback journal coverage | **Done** |
| 71 | Browser engine port ABI — WPE WebKit target selection, host contract diagnostics, SDK docs, and native fallback readiness | **Done** |
| 72 | Userspace threads and futex ABI — same-address-space thread tasks, futex wait/wake, libcool wrappers, `/bin/threaddemo`, and diagnostics | **Done** |
| 73 | Thread-local storage and pthread runtime groundwork — FS-base TLS, spawn-with-TLS, libcool pthread-style primitives, `/bin/tlsdemo`, and diagnostics | **Done** |
| 74 | POSIX pthread/libc shim — `pthread_create`/join/exit/self, mutex/condvar/once/key wrappers, per-thread `errno`, timing helpers, `/bin/pthreaddemo`, and docs | **Done** |
| 75 | Dynamic loader foundation — ABI v13 `mprotect`, `/lib` ET_DYN loading, RELA relocations, dynsym/init arrays, `/bin/lddemo`, and docs | **Done** |
| 76 | Dynamic linker dependencies and ELF TLS — `DT_NEEDED`, soname lookup, cross-object symbols, TLS records/relocations, dependency init order, and docs | **Done** |
| 77 | File-backed mmap and POSIX file runtime — ABI v14 `mmap_file`, `open_flags`, read-only ET_DYN file maps, `/bin/mmapdemo`, and docs | **Done** |
| 80 | Installer and first boot — graphical owner-account setup, `/CONFIG/FIRSTBOOT.CFG`, setup persistence, and Phase 80 smoke coverage | **Done** |
| 81 | First-boot recovery and reset — admin/recovery reset, state repair, boot hardening, and Phase 81 smoke coverage | **Done** |
| 82 | QEMU disk installer v1 — installer mode, IDE disk discovery, sector-copy install/verify, and installed-target first-boot smoke coverage | **Done** |
| 83 | Self-booting QEMU installed disk — BIOS/MBR target layout, CoolFS root partition discovery, standalone target boot smoke | **Done** |
| 84 | Installer v2 — target preflight, review/confirmation, graphical progress, and safer disk refusal coverage | **Done** |
| 85 | UEFI/GPT installer foundation — `uefi.img`, OVMF run helpers, GPT root discovery, and standalone UEFI installed-target smoke | **Done** |
| 86 | AHCI/SATA storage and USB image foundation — `sata*` block devices, AHCI installer smoke, and raw UEFI/GPT USB image build | **Done** |
| 87 | Runtime USB mass-storage root boot — USB MSC BOT, `usb*` block devices, and QEMU USB-storage first-boot smoke | **Done** |
| 88 | NVMe/PCIe storage root boot — `nvme*n1` block devices, QEMU NVMe root boot, and NVMe installer targets | **Done** |
| 89 | Bare-metal USB boot readiness — hardware diagnostics, safe USB image fallback, and physical-boot troubleshooting docs | **Done** |
| 90 | Physical disk installer guardrails — USB-source protection, internal SATA/NVMe install targets, and guarded install verification | **Done** |
| 91 | Bare-metal hardware readiness — USB hub/UASP diagnostics, root-scan reasons, installer preflight, and topology smoke coverage | **Done** |
| 92 | Secure Boot test-key foundation — secure OVMF helpers, digest-bound UEFI loader, secure USB image, and Phase 92 smoke | **Done** |
| 93 | Enforced Secure Boot test-key chain — enrolled OVMF vars, signed `BOOTX64.EFI`, artifact verification, and tamper/rejection smoke coverage | **Done** |
| 94 | Real-PC Secure Boot enrollment and diagnostics — public enrollment bundle, secure USB ESP manifest, loader firmware-status handoff, and Phase 94 smoke | **Done** |

Full task checklists and technical notes in [ROADMAP.md](ROADMAP.md).
