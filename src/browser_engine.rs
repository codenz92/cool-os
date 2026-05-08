extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

pub const PORT_ABI_VERSION: u64 = 1;
pub const TARGET_ENGINE: &str = "wpe-webkit";
pub const FALLBACK_ENGINE: &str = "coolos-native";
pub const CONFIG_PATH: &str = "/CONFIG/BROWSER-ENGINE.CFG";
pub const LOG_PATH: &str = "/LOGS/BROWSER-ENGINE.TXT";
pub const SDK_DOC_PATH: &str = "/SDK/BROWSER_ENGINE_PORT.TXT";
pub const SYSTEM_DIR: &str = "/SYSTEM/BROWSER-ENGINE";
const WPE_READY_PATH: &str = "/SYSTEM/BROWSER-ENGINE/WPE.READY";

const DEFAULT_CONFIG: &[u8] = b"preferred=wpe-webkit\nfallback=coolos-native\nmode=port-prep\nengine_abi=1\nsurface=rgba-shmem\ninput=gui-events\nnetwork=kernel-http-tls\n";

const INITIAL_LOG: &[u8] = b"coolOS browser engine port log\nphase=73\npreferred=wpe-webkit\nactive=coolos-native\nstatus=port-prep\nthreads_futex=ready\ntls_pthread=partial\n";

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
        detail: "CoolFS/VFS provides files, dirs, metadata, rename, fd tables",
        next: "add broader POSIX errno/open flags, temp files, mmap-backed files",
    },
    Requirement {
        key: "memory-map",
        status: RequirementStatus::Partial,
        detail: "mmap and shared memory exist with bounded per-task limits",
        next: "raise per-process address-space caps and support file-backed mappings",
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
        detail: "thread groups, futex wait/wake, FS-base TLS, and pthread-style libcool primitives exist",
        next: "wire the hosted C/POSIX pthread ABI to the libc runtime",
    },
    Requirement {
        key: "dynamic-linker",
        status: RequirementStatus::Missing,
        detail: "ELF loader handles static no_std binaries only",
        next: "load shared objects, relocations, TLS, and C/C++ runtime support",
    },
    Requirement {
        key: "jit-execmem",
        status: RequirementStatus::Missing,
        detail: "no W^X/JIT allocation policy or executable userspace mappings",
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
    let active = active_engine_name();
    let mut lines = vec![
        format!(
            "engine-port abi={} target={} fallback={} active={}",
            PORT_ABI_VERSION, TARGET_ENGINE, FALLBACK_ENGINE, active
        ),
        String::from(
            "goal=host a mature WebKit-class engine; native browser remains fallback/debug path",
        ),
        format!("config={} {}", CONFIG_PATH, config_state),
        format!("sdk_doc={} {}", SDK_DOC_PATH, sdk_doc_state()),
        format!("backend_ready={} {}", WPE_READY_PATH, backend_state),
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
        String::from("mode=port-prep"),
        String::from("surface=rgba-shmem"),
        String::from("input=gui-events"),
        String::from("network=kernel-http-tls-and-sockets"),
        String::from("process=shell-plus-web-process-planned"),
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
    vec![
        format!(
            "browser_engine=port-prep target={} active={} abi={}",
            TARGET_ENGINE,
            active_engine_name(),
            PORT_ABI_VERSION
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
    match crate::vfs::vfs_kernel_read_file(LOG_PATH).and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(text) => text.lines().map(String::from).collect(),
        None => vec![String::from("browser engine log missing")],
    }
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
        String::from(""),
    ];
    lines.extend(status_lines());
    lines.push(String::from(""));
    lines.extend(manifest_lines());
    lines
}

pub fn active_engine_name() -> &'static str {
    if wpe_backend_ready() {
        TARGET_ENGINE
    } else {
        FALLBACK_ENGINE
    }
}

fn wpe_backend_ready() -> bool {
    crate::vfs::vfs_kernel_read_file(WPE_READY_PATH).is_some()
}

fn readiness_label(ready: usize, partial: usize, missing: usize) -> &'static str {
    if missing == 0 && partial == 0 {
        "engine-ready"
    } else if ready >= 3 && partial >= 3 {
        "port-prep"
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
