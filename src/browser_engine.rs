extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, Ordering};

pub const PORT_ABI_VERSION: u64 = 1;
pub const TARGET_ENGINE: &str = "wpe-webkit";
pub const FALLBACK_ENGINE: &str = "coolos-native";
pub const CONFIG_PATH: &str = "/CONFIG/BROWSER-ENGINE.CFG";
pub const LOG_PATH: &str = "/LOGS/BROWSER-ENGINE.TXT";
pub const HOST_LOG_PATH: &str = "/LOGS/BROWSER-ENGINE-HOST.TXT";
pub const SDK_DOC_PATH: &str = "/SDK/BROWSER_ENGINE_PORT.TXT";
pub const SYSTEM_DIR: &str = "/SYSTEM/BROWSER-ENGINE";
pub const HOST_HELPER_PATH: &str = "/bin/browserhost";
pub const HOST_READY_PATH: &str = "/SYSTEM/BROWSER-ENGINE/HOST.READY";
pub const HOST_REQUEST_PATH: &str = "/SYSTEM/BROWSER-ENGINE/HOST.REQUEST";
const WPE_READY_PATH: &str = "/SYSTEM/BROWSER-ENGINE/WPE.READY";

const DEFAULT_CONFIG: &[u8] = b"preferred=wpe-webkit\nfallback=coolos-native\nmode=host-bootstrap\nengine_abi=1\nsurface=rgba-shmem\ninput=gui-events\nnetwork=kernel-http-tls\nhost=/bin/browserhost\n";

const INITIAL_LOG: &[u8] = b"coolOS browser engine port log\nphase=97\npreferred=wpe-webkit\nactive=coolos-native\nstatus=host-bootstrap\nthreads_futex=ready\ntls_pthread=ready\nposix_libc=partial-open-flags\ndynamic_linker=partial-file-mmap\nwx_mprotect=ready\nfile_mmap=partial-readonly\nhost_bridge=file-artifacts\n";
const INITIAL_HOST_LOG: &[u8] =
    b"coolOS browser engine host log\nphase=97\nstatus=not-started\nbackend=test\n";
static INIT_DONE: AtomicBool = AtomicBool::new(false);
static HOST_STATE: spin::Mutex<HostState> = spin::Mutex::new(HostState::new());

#[derive(Clone, Copy)]
struct HostState {
    pid: Option<usize>,
    launches: usize,
}

impl HostState {
    const fn new() -> Self {
        Self {
            pid: None,
            launches: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequirementStatus {
    Ready,
    Partial,
    Missing,
}

impl RequirementStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Partial => "partial",
            Self::Missing => "missing",
        }
    }
}

#[derive(Clone, Copy)]
struct Requirement {
    key: &'static str,
    status: RequirementStatus,
    detail: &'static str,
    next: &'static str,
}

const REQUIREMENTS: &[Requirement] = &[
    Requirement {
        key: "process-isolation",
        status: RequirementStatus::Ready,
        detail: "ring-3 ELF tasks, per-process address spaces, signals, waitpid",
        next: "split browser shell, web process, and network/storage helpers",
    },
    Requirement {
        key: "surface-output",
        status: RequirementStatus::Ready,
        detail: "userspace GUI windows can present RGBA buffers",
        next: "map WebKit/WPE view buffers onto gui_present surfaces",
    },
    Requirement {
        key: "input-events",
        status: RequirementStatus::Ready,
        detail: "GUI event polling exposes keyboard and pointer input",
        next: "translate coolOS events to WPE view input events",
    },
    Requirement {
        key: "networking",
        status: RequirementStatus::Partial,
        detail: "DNS, TCP sockets, HTTP/1.1, and TLS 1.3 are available",
        next: "add POSIX-style socket flags, nonblocking connect, compression, HTTP/2",
    },
    Requirement {
        key: "filesystem",
        status: RequirementStatus::Partial,
        detail: "CoolFS/VFS provides files, dirs, metadata, rename, fd tables, /TMP, and read-only mmap_file",
        next: "add broader POSIX errno/open flags, unlink-on-close, and writable mmap sync",
    },
    Requirement {
        key: "memory-map",
        status: RequirementStatus::Partial,
        detail: "mmap, mprotect, shared memory, and read-only file-backed mappings exist with bounded per-task limits",
        next: "raise per-process address-space caps and support shared/writable file mappings",
    },
    Requirement {
        key: "timers-poll",
        status: RequirementStatus::Partial,
        detail: "sleep_ms and evented poll cover pipes, TTY, sockets, GUI, child exit",
        next: "add high-resolution timers and WebKit-compatible run-loop wakeups",
    },
    Requirement {
        key: "fonts-text",
        status: RequirementStatus::Partial,
        detail: "kernel bitmap font path and /FONTS directory exist",
        next: "ship scalable font files, fontconfig-like lookup, shaping, Unicode breaks",
    },
    Requirement {
        key: "threads-futex",
        status: RequirementStatus::Ready,
        detail: "thread groups, futex wait/wake, FS-base TLS, and POSIX pthread/libc shims exist",
        next: "wire hosted engine builds onto the shim and fill remaining pthread edge cases",
    },
    Requirement {
        key: "dynamic-linker",
        status: RequirementStatus::Partial,
        detail:
            "ET_DYN loader maps /lib objects, file-backs read-only segments, follows DT_NEEDED, resolves global symbols, handles TLS records, and runs init arrays",
        next:
            "add libc ld.so entry points, symbol versioning, lazy PLT binding, and C++ runtime support",
    },
    Requirement {
        key: "jit-execmem",
        status: RequirementStatus::Partial,
        detail: "ABI 14 mmap_file and mprotect support W^X executable mappings for loaded text",
        next: "define signed-engine JIT policy or force JavaScriptCore interpreter mode",
    },
    Requirement {
        key: "gpu-accel",
        status: RequirementStatus::Missing,
        detail: "framebuffer compositor has no EGL/OpenGL/WebGL backend",
        next: "start with software rendering, then add GPU acceleration later",
    },
];

pub fn init() {
    let _ = crate::vfs::vfs_kernel_create_dir("/SYSTEM");
    let _ = crate::vfs::vfs_kernel_create_dir(SYSTEM_DIR);
    ensure_file(CONFIG_PATH, DEFAULT_CONFIG);
    ensure_file(LOG_PATH, INITIAL_LOG);
    ensure_file(HOST_LOG_PATH, INITIAL_HOST_LOG);
    if INIT_DONE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let _ = crate::vfs::vfs_kernel_delete(HOST_READY_PATH);
        let _ = crate::vfs::vfs_kernel_delete(HOST_REQUEST_PATH);
    }
}

pub fn status_lines() -> Vec<String> {
    init();
    let (ready, partial, missing) = requirement_counts();
    let config_state = if crate::vfs::vfs_kernel_read_file(CONFIG_PATH).is_some() {
        "present"
    } else {
        "missing"
    };
    let backend_state = if wpe_backend_ready() {
        "ready"
    } else {
        "missing"
    };
    let host_state = host_status();
    let fallback_reason = if wpe_backend_ready() {
        "none"
    } else if host_state.ready {
        "wpe-backend-not-ready-test-host-only"
    } else if host_state.helper_present {
        "host-not-started"
    } else {
        "host-helper-missing"
    };
    let active = active_engine_name();
    let mut lines = vec![
        format!(
            "engine-port abi={} target={} fallback={} active={}",
            PORT_ABI_VERSION, TARGET_ENGINE, FALLBACK_ENGINE, active
        ),
        String::from(
            "goal=host a mature WebKit-class engine; native browser remains fallback/debug path",
        ),
        String::from("mode=host-bootstrap"),
        format!("config={} {}", CONFIG_PATH, config_state),
        format!("sdk_doc={} {}", SDK_DOC_PATH, sdk_doc_state()),
        format!("backend_ready={} {}", WPE_READY_PATH, backend_state),
        format!(
            "host_helper={} {}",
            HOST_HELPER_PATH,
            if host_state.helper_present {
                "present"
            } else {
                "missing"
            }
        ),
        format!(
            "host_ready={} {}",
            HOST_READY_PATH,
            if host_state.ready { "ready" } else { "missing" }
        ),
        format!(
            "host_process pid={} state={} launches={}",
            host_state
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| String::from("-")),
            host_state.process_state,
            host_state.launches
        ),
        format!(
            "host_bridge request={} log={} ipc=file-artifacts surface=rgba-gui input=gui-events",
            HOST_REQUEST_PATH, HOST_LOG_PATH
        ),
        format!("fallback active={} reason={}", active, fallback_reason),
        format!(
            "requirements ready={} partial={} missing={}",
            ready, partial, missing
        ),
        format!("readiness={}", readiness_label(ready, partial, missing)),
    ];
    lines.extend(requirement_lines());
    lines
}

pub fn manifest_lines() -> Vec<String> {
    vec![
        String::from("coolOS browser engine port manifest"),
        format!("engine_abi={}", PORT_ABI_VERSION),
        format!("preferred={}", TARGET_ENGINE),
        format!("fallback={}", FALLBACK_ENGINE),
        String::from("mode=host-bootstrap"),
        String::from("surface=rgba-shmem"),
        String::from("input=gui-events"),
        String::from("network=kernel-http-tls-and-sockets"),
        String::from("process=browser-shell-plus-browserhost-test-backend"),
        format!("host_helper={}", HOST_HELPER_PATH),
        format!("host_ready={}", HOST_READY_PATH),
        format!("host_request={}", HOST_REQUEST_PATH),
        String::from("posix=partial-libc-pthread-open-flags"),
        String::from("dynamic_linker=partial-file-mmap"),
        String::from("file_mmap=partial-readonly"),
        String::from("storage=/CONFIG/BROWSER.*, /Downloads, /TMP"),
        String::from("font_source=/FONTS"),
        String::from("backend_probe=/SYSTEM/BROWSER-ENGINE/WPE.READY"),
    ]
}

pub fn requirement_lines() -> Vec<String> {
    REQUIREMENTS
        .iter()
        .map(|req| {
            format!(
                "req.{}={} detail=\"{}\" next=\"{}\"",
                req.key,
                req.status.as_str(),
                req.detail,
                req.next
            )
        })
        .collect()
}

pub fn recovery_lines() -> Vec<String> {
    init();
    let (ready, partial, missing) = requirement_counts();
    let host_state = host_status();
    vec![
        format!(
            "browser_engine=host-bootstrap target={} active={} abi={}",
            TARGET_ENGINE,
            active_engine_name(),
            PORT_ABI_VERSION
        ),
        format!(
            "browser_engine_host ready={} pid={} state={} helper={}",
            if host_state.ready { "yes" } else { "no" },
            host_state
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| String::from("-")),
            host_state.process_state,
            if host_state.helper_present {
                "present"
            } else {
                "missing"
            }
        ),
        format!(
            "browser_engine_requirements ready={} partial={} missing={}",
            ready, partial, missing
        ),
        format!("browser_engine_config={}", CONFIG_PATH),
    ]
}

pub fn log_lines() -> Vec<String> {
    init();
    let mut lines = match crate::vfs::vfs_kernel_read_file(LOG_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(text) => text.lines().map(String::from).collect(),
        None => vec![String::from("browser engine log missing")],
    };
    lines.push(String::from("-- host --"));
    match crate::vfs::vfs_kernel_read_file(HOST_LOG_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(text) => lines.extend(text.lines().map(String::from)),
        None => lines.push(String::from("browser engine host log missing")),
    }
    lines
}

pub fn config_lines() -> Vec<String> {
    init();
    match crate::vfs::vfs_kernel_read_file(CONFIG_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(text) => text.lines().map(String::from).collect(),
        None => vec![String::from("browser engine config missing")],
    }
}

pub fn browser_page_lines() -> Vec<String> {
    let mut lines = vec![
        String::from("Browser Engine Port"),
        String::from(""),
        String::from("WPE WebKit is the selected target for modern web compatibility."),
        String::from("The current native renderer stays available as the fallback/debug browser."),
        String::from("Phase 97 adds a browserhost test backend for host IPC and surface proof."),
        String::from(""),
    ];
    lines.extend(status_lines());
    lines.push(String::from(""));
    lines.extend(manifest_lines());
    lines.push(String::from(""));
    lines.extend(host_ready_lines());
    lines
}

pub fn active_engine_name() -> &'static str {
    if wpe_backend_ready() {
        TARGET_ENGINE
    } else {
        FALLBACK_ENGINE
    }
}

pub fn ensure_host_started(url: &str, width: i32, height: i32) -> Vec<String> {
    init();
    let width = width.max(1);
    let height = height.max(1);
    let request = format!(
        "load={}\nwidth={}\nheight={}\nsurface=rgba-gui\ninput=gui-events\nsource=browser-shell\n",
        url, width, height
    );
    let _ = crate::vfs::vfs_kernel_safe_write_file(HOST_REQUEST_PATH, request.as_bytes());

    if crate::vfs::vfs_kernel_read_file(HOST_HELPER_PATH).is_none() {
        append_log("host helper missing; native fallback remains active");
        return vec![
            String::from("host_launch=failed reason=helper-missing"),
            format!("host_helper={} missing", HOST_HELPER_PATH),
        ];
    }

    {
        let mut state = HOST_STATE.lock();
        if let Some(pid) = state.pid {
            if host_pid_active(pid) {
                if crate::vfs::vfs_kernel_read_file(HOST_READY_PATH).is_none() {
                    publish_host_ready(pid, url, width, height);
                }
                append_log("host launch skipped; existing browserhost process active");
                return vec![
                    format!("host_launch=already-running pid={}", pid),
                    format!("host_request={} written", HOST_REQUEST_PATH),
                ];
            }
            state.pid = None;
        }
    }

    match crate::elf::spawn_elf_process_with_args(HOST_HELPER_PATH, &[url]) {
        Ok(pid) => {
            {
                let mut state = HOST_STATE.lock();
                state.pid = Some(pid);
                state.launches = state.launches.saturating_add(1);
            }
            publish_host_ready(pid, url, width, height);
            append_log("spawned /bin/browserhost test backend");
            vec![
                format!("host_launch=spawned pid={}", pid),
                format!("host_request={} written", HOST_REQUEST_PATH),
            ]
        }
        Err(err) => {
            append_log("host launch failed; native fallback remains active");
            vec![
                format!("host_launch=failed reason={}", err.as_str()),
                String::from("fallback=native"),
            ]
        }
    }
}

pub fn host_ready_lines() -> Vec<String> {
    match crate::vfs::vfs_kernel_read_file(HOST_READY_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(text) => text.lines().map(String::from).collect(),
        None => vec![String::from("engine_host ready=no")],
    }
}

fn wpe_backend_ready() -> bool {
    crate::vfs::vfs_kernel_read_file(WPE_READY_PATH).is_some()
}

fn readiness_label(ready: usize, partial: usize, missing: usize) -> &'static str {
    if missing == 0 && partial == 0 {
        "engine-ready"
    } else if ready >= 3 && partial >= 3 {
        "host-bootstrap"
    } else {
        "blocked"
    }
}

fn requirement_counts() -> (usize, usize, usize) {
    let mut ready = 0usize;
    let mut partial = 0usize;
    let mut missing = 0usize;
    for req in REQUIREMENTS {
        match req.status {
            RequirementStatus::Ready => ready += 1,
            RequirementStatus::Partial => partial += 1,
            RequirementStatus::Missing => missing += 1,
        }
    }
    (ready, partial, missing)
}

fn sdk_doc_state() -> &'static str {
    if crate::vfs::vfs_kernel_read_file(SDK_DOC_PATH).is_some() {
        "present"
    } else {
        "missing"
    }
}

fn ensure_file(path: &str, data: &[u8]) {
    if crate::vfs::vfs_kernel_read_file(path).is_some() {
        return;
    }
    let _ = crate::vfs::vfs_kernel_create_file(path);
    let _ = crate::vfs::vfs_kernel_write_file(path, data);
}

struct HostStatus {
    helper_present: bool,
    ready: bool,
    pid: Option<usize>,
    process_state: &'static str,
    launches: usize,
}

fn host_status() -> HostStatus {
    let helper_present = crate::vfs::vfs_kernel_read_file(HOST_HELPER_PATH).is_some();
    let ready = crate::vfs::vfs_kernel_read_file(HOST_READY_PATH).is_some();
    let state = HOST_STATE.lock();
    let pid = state.pid;
    let process_state = pid
        .and_then(|pid| crate::scheduler::task_status_exit(pid))
        .map(|(status, _)| task_status_label(status))
        .unwrap_or("not-started");
    HostStatus {
        helper_present,
        ready,
        pid,
        process_state,
        launches: state.launches,
    }
}

fn host_pid_active(pid: usize) -> bool {
    crate::scheduler::task_status_exit(pid)
        .map(|(status, _)| {
            matches!(
                status,
                crate::scheduler::TaskStatus::Ready
                    | crate::scheduler::TaskStatus::Running
                    | crate::scheduler::TaskStatus::Blocked
                    | crate::scheduler::TaskStatus::Stopped
            )
        })
        .unwrap_or(false)
}

fn task_status_label(status: crate::scheduler::TaskStatus) -> &'static str {
    match status {
        crate::scheduler::TaskStatus::Ready => "ready",
        crate::scheduler::TaskStatus::Running => "running",
        crate::scheduler::TaskStatus::Blocked => "blocked",
        crate::scheduler::TaskStatus::Stopped => "stopped",
        crate::scheduler::TaskStatus::Exited => "exited",
        crate::scheduler::TaskStatus::Reaped => "reaped",
    }
}

fn append_log(line: &str) {
    let mut text = crate::vfs::vfs_kernel_read_file(LOG_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_else(|| String::from("coolOS browser engine port log\n"));
    text.push_str(line);
    text.push('\n');
    let _ = crate::vfs::vfs_kernel_safe_write_file(LOG_PATH, text.as_bytes());
}

fn publish_host_ready(pid: usize, url: &str, width: i32, height: i32) {
    let ready = format!(
        "engine_host ready=yes backend=test process=browserhost pid={} surface=rgba-gui input=gui-events title=\"coolOS Engine Host Test\"\n",
        pid
    );
    let host_log = format!(
        "coolOS browser engine host log\nphase=97\nstatus=ready\nbackend=test\nprocess=browserhost\npid={}\nipc=file-bridge\nsurface=rgba-gui\ninput=gui-events\nrequest:\nload={}\nwidth={}\nheight={}\n",
        pid, url, width, height
    );
    let _ = crate::vfs::vfs_kernel_safe_write_file(HOST_READY_PATH, ready.as_bytes());
    let _ = crate::vfs::vfs_kernel_safe_write_file(HOST_LOG_PATH, host_log.as_bytes());
}
