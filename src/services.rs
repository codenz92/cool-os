extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

pub const CONFIG_PATH: &str = "/CONFIG/SERVICES.CFG";
pub const HISTORY_PATH: &str = "/LOGS/SERVICES.TXT";

const CONFIG_DIR: &str = "/CONFIG";
const LOG_DIR: &str = "/LOGS";

const NO_DEPS: &[&str] = &[];
const EVENT_BUS_DEPS: &[&str] = &["event-bus"];
const DEVICE_DEPS: &[&str] = &["device-registry"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Running,
    Failed,
}

impl ServiceState {
    pub const fn label(self) -> &'static str {
        match self {
            ServiceState::Stopped => "stopped",
            ServiceState::Running => "running",
            ServiceState::Failed => "failed",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        if label.eq_ignore_ascii_case("stopped") {
            Some(ServiceState::Stopped)
        } else if label.eq_ignore_ascii_case("running") {
            Some(ServiceState::Running)
        } else if label.eq_ignore_ascii_case("failed") {
            Some(ServiceState::Failed)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct Service {
    pub name: &'static str,
    pub order: u8,
    pub restart: &'static str,
    pub dependencies: &'static [&'static str],
    pub state: ServiceState,
    pub credentials: crate::security::Credentials,
    pub restarts: u32,
    pub failures: u32,
    pub last_failure_tick: u64,
    pub next_restart_tick: u64,
    pub last_tick: u64,
    pub loops: u64,
    pub reap_count: u32,
    pub last_reason: &'static str,
}

#[derive(Clone, Copy)]
pub struct ServiceHealth {
    pub total: usize,
    pub running: usize,
    pub stopped: usize,
    pub failed: usize,
    pub degraded: usize,
    pub backoff: usize,
}

static SERVICES: Mutex<Vec<Service>> = Mutex::new(Vec::new());

pub fn init() {
    let mut services = alloc::vec![
        service("event-bus", 1, "always", NO_DEPS),
        service("device-registry", 2, "always", EVENT_BUS_DEPS),
        service("search-index", 3, "manual", EVENT_BUS_DEPS),
        service("package-db", 4, "on-failure", EVENT_BUS_DEPS),
        service("notification-center", 5, "always", EVENT_BUS_DEPS),
        service("network-stack", 6, "manual", DEVICE_DEPS),
        service("power-manager", 7, "manual", DEVICE_DEPS),
        service("writeback", 8, "always", EVENT_BUS_DEPS),
    ];
    apply_persisted_state(&mut services);
    services.sort_by(|a, b| a.order.cmp(&b.order));
    *SERVICES.lock() = services;
    persist_documents("boot");
    crate::event_bus::emit("services", "boot", "service supervisor initialized");
    crate::profiler::record_service("supervisor", "initialized");
}

pub fn start(name: &str) -> bool {
    let mut events = Vec::new();
    let ok = mutate_services("manual-start", |services, now| {
        start_recursive(services, name, now, "manual-start", &mut events)
    });
    for service_name in events {
        crate::event_bus::emit("services", "running", service_name);
        crate::profiler::record_service(service_name, "manual-start");
    }
    ok
}

pub fn restart(name: &str) -> bool {
    let mut events = Vec::new();
    let ok = mutate_services("manual-restart", |services, now| {
        if !start_recursive(services, name, now, "manual-restart", &mut events) {
            return false;
        }
        if let Some(service) = find_mut(services, name) {
            service.restarts = service.restarts.saturating_add(1);
            service.reap_count = service.reap_count.saturating_add(1);
            service.last_tick = now;
            service.last_reason = "manual-restart";
        }
        true
    });
    for service_name in events {
        crate::event_bus::emit("services", "restart", service_name);
        crate::profiler::record_service(service_name, "manual-restart");
    }
    ok
}

pub fn stop(name: &str) -> bool {
    let mut stopped = Vec::new();
    let ok = mutate_services("manual-stop", |services, now| {
        let Some(target) = find_mut(services, name) else {
            return false;
        };
        let target_name = target.name;
        target.state = ServiceState::Stopped;
        target.last_tick = now;
        target.next_restart_tick = 0;
        target.last_reason = "manual-stop";
        stopped.push(target_name);
        stop_dependents(services, now, &mut stopped);
        true
    });
    for service_name in stopped {
        crate::event_bus::emit("services", "stopped", service_name);
        crate::profiler::record_service(service_name, "manual-stop");
    }
    ok
}

pub fn fail(name: &str) -> bool {
    let mut failed_name = None;
    let ok = mutate_services("failed", |services, now| {
        let Some(service) = find_mut(services, name) else {
            return false;
        };
        mark_failed(service, now);
        failed_name = Some(service.name);
        true
    });
    if let Some(service_name) = failed_name {
        crate::event_bus::emit("services", "failed", service_name);
        crate::profiler::record_service(service_name, "failed");
    }
    ok
}

pub fn supervise() {
    supervise_inner(false);
}

pub fn supervise_once() {
    supervise_inner(true);
}

fn supervise_inner(force_restart: bool) {
    let now = crate::interrupts::ticks();
    let mut restart_events = Vec::new();
    let mut docs = None;
    {
        let mut services = SERVICES.lock();
        let states = state_snapshot(&services);
        let mut changed = false;
        for service in services.iter_mut() {
            if service.state == ServiceState::Failed
                && (service.restart == "always" || service.restart == "on-failure")
                && (force_restart || now >= service.next_restart_tick)
                && dependencies_ready_from_states(service, &states)
            {
                service.state = ServiceState::Running;
                service.restarts = service.restarts.saturating_add(1);
                service.last_tick = now;
                service.next_restart_tick = 0;
                service.reap_count = service.reap_count.saturating_add(1);
                service.last_reason = "supervisor-restart";
                restart_events.push(service.name);
                changed = true;
            }

            if service.state != ServiceState::Running
                || !dependencies_ready_from_states(service, &states)
            {
                continue;
            }
            let interval = match service.name {
                "search-index" => crate::interrupts::ticks_for_millis(5000),
                "package-db" => crate::interrupts::ticks_for_millis(3000),
                "notification-center" | "event-bus" => crate::interrupts::ticks_for_millis(1000),
                _ => crate::interrupts::ticks_for_millis(1500),
            };
            if service.last_tick == 0 || now.wrapping_sub(service.last_tick) >= interval {
                service.last_tick = now;
                service.loops = service.loops.saturating_add(1);
                match service.name {
                    "search-index" => {
                        crate::deferred::enqueue(crate::deferred::DeferredWork::RefreshSearchIndex)
                    }
                    "package-db" => crate::deferred::enqueue(
                        crate::deferred::DeferredWork::FlushFilesystemJournal,
                    ),
                    "event-bus" | "notification-center" => {
                        crate::deferred::enqueue(crate::deferred::DeferredWork::FlushKernelLog)
                    }
                    "writeback" => {
                        crate::deferred::enqueue(crate::deferred::DeferredWork::FlushWriteback)
                    }
                    _ => {}
                }
            }
        }
        if changed {
            docs = Some(serialized_documents_locked(
                &services,
                now,
                "supervisor-restart",
            ));
        }
    }
    for service_name in restart_events {
        crate::event_bus::emit("services", "restart", service_name);
        crate::profiler::record_service(service_name, "supervisor-restart");
    }
    if let Some((config, history)) = docs {
        let _ = write_supervisor_files(&config, &history);
    }
}

pub fn lines() -> Vec<String> {
    let now = crate::interrupts::ticks();
    SERVICES
        .lock()
        .iter()
        .map(|service| service_line(service, now, true))
        .collect()
}

pub fn status_lines(name: &str) -> Option<Vec<String>> {
    let now = crate::interrupts::ticks();
    let services = SERVICES.lock();
    let service = services
        .iter()
        .find(|service| service.name.eq_ignore_ascii_case(name))?;
    Some(alloc::vec![
        service_line(service, now, false),
        format!("config={} history={}", CONFIG_PATH, HISTORY_PATH),
    ])
}

pub fn health() -> ServiceHealth {
    let now = crate::interrupts::ticks();
    let services = SERVICES.lock();
    health_locked(&services, now)
}

pub fn recovery_lines() -> Vec<String> {
    let now = crate::interrupts::ticks();
    let services = SERVICES.lock();
    let states = state_snapshot(&services);
    let mut lines = alloc::vec![
        health_summary_locked(&services, now),
        format!("config={} history={}", CONFIG_PATH, HISTORY_PATH),
    ];
    let mut degraded = 0usize;
    for service in services.iter() {
        if service.state != ServiceState::Running
            || !dependencies_ready_from_states(service, &states)
        {
            degraded += 1;
            lines.push(format!(
                "{} state={} deps={} failures={} restarts={} backoff={} reason={}",
                service.name,
                service.state.label(),
                deps_label(service.dependencies),
                service.failures,
                service.restarts,
                backoff_label(service, now),
                service.last_reason
            ));
        }
    }
    if degraded == 0 {
        lines.push(String::from("all supervised services healthy"));
    }
    lines
}

pub fn history_lines() -> Vec<String> {
    let Some(data) = crate::vfs::vfs_kernel_read_file(HISTORY_PATH) else {
        return alloc::vec![format!("history={} missing", HISTORY_PATH)];
    };
    let Ok(text) = core::str::from_utf8(&data) else {
        return alloc::vec![format!("history={} unreadable", HISTORY_PATH)];
    };
    let mut lines = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            lines.push(String::from(line));
        }
    }
    if lines.is_empty() {
        lines.push(format!("history={} empty", HISTORY_PATH));
    }
    lines
}

pub fn service_roundtrip_for_test() -> bool {
    let now = crate::interrupts::ticks();
    let mut services = SERVICES.lock();
    let original = services.clone();
    let Some(idx) = services
        .iter()
        .position(|service| service.name.eq_ignore_ascii_case("package-db"))
    else {
        return false;
    };
    let deps = services[idx].dependencies;
    for dep in deps {
        if let Some(service) = find_mut(&mut services, dep) {
            service.state = ServiceState::Running;
        }
    }
    mark_failed(&mut services[idx], now);
    let states = state_snapshot(&services);
    if services[idx].state == ServiceState::Failed
        && dependencies_ready_from_states(&services[idx], &states)
    {
        services[idx].state = ServiceState::Running;
        services[idx].restarts = services[idx].restarts.saturating_add(1);
        services[idx].reap_count = services[idx].reap_count.saturating_add(1);
        services[idx].next_restart_tick = 0;
        services[idx].last_reason = "test-restart";
    }
    let service = &services[idx];
    let ok = service.state == ServiceState::Running
        && service.restarts > 0
        && service.failures > 0
        && service.credentials.uid == crate::security::SERVICE_UID
        && crate::security::can_write_files(service.credentials);
    *services = original;
    ok
}

fn mutate_services<F>(persist_reason: &'static str, mut f: F) -> bool
where
    F: FnMut(&mut Vec<Service>, u64) -> bool,
{
    let now = crate::interrupts::ticks();
    let (ok, docs) = {
        let mut services = SERVICES.lock();
        let ok = f(&mut services, now);
        let docs = if ok {
            Some(serialized_documents_locked(&services, now, persist_reason))
        } else {
            None
        };
        (ok, docs)
    };
    if let Some((config, history)) = docs {
        let _ = write_supervisor_files(&config, &history);
    }
    ok
}

fn apply_persisted_state(services: &mut [Service]) {
    let Some(data) = crate::vfs::vfs_kernel_read_file(CONFIG_PATH) else {
        return;
    };
    let Ok(text) = core::str::from_utf8(&data) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("coolOS") {
            continue;
        }
        let mut words = line.split_whitespace();
        let Some(name) = words.next() else {
            continue;
        };
        let mut state = None;
        for word in words {
            if let Some(value) = word.strip_prefix("state=") {
                state = ServiceState::from_label(value);
            }
        }
        let Some(state) = state else {
            continue;
        };
        if let Some(service) = services
            .iter_mut()
            .find(|service| service.name.eq_ignore_ascii_case(name))
        {
            service.state = state;
            service.last_reason = "persisted";
        }
    }
}

fn start_recursive(
    services: &mut [Service],
    name: &str,
    now: u64,
    reason: &'static str,
    events: &mut Vec<&'static str>,
) -> bool {
    let Some(idx) = services
        .iter()
        .position(|service| service.name.eq_ignore_ascii_case(name))
    else {
        return false;
    };
    let deps = services[idx].dependencies;
    for dep in deps {
        if !start_recursive(services, dep, now, reason, events) {
            return false;
        }
    }
    let service = &mut services[idx];
    service.state = ServiceState::Running;
    service.last_tick = now;
    service.next_restart_tick = 0;
    service.last_reason = reason;
    events.push(service.name);
    true
}

fn stop_dependents(services: &mut [Service], now: u64, stopped: &mut Vec<&'static str>) {
    loop {
        let states = state_snapshot(services);
        let mut changed = false;
        for service in services.iter_mut() {
            if service.state != ServiceState::Running {
                continue;
            }
            if service.dependencies.iter().any(|dep| {
                states.iter().any(|(name, state)| {
                    name.eq_ignore_ascii_case(dep) && *state == ServiceState::Stopped
                })
            }) {
                service.state = ServiceState::Stopped;
                service.last_tick = now;
                service.next_restart_tick = 0;
                service.last_reason = "dependency-stopped";
                stopped.push(service.name);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn mark_failed(service: &mut Service, now: u64) {
    service.state = ServiceState::Failed;
    service.failures = service.failures.saturating_add(1);
    service.last_failure_tick = now;
    service.next_restart_tick = now.saturating_add(backoff_ticks(service.failures));
    service.last_tick = now;
    service.last_reason = "failed";
}

fn backoff_ticks(failures: u32) -> u64 {
    if failures <= 1 {
        return 0;
    }
    let step = failures.saturating_sub(2).min(5);
    let millis = 500u64.saturating_mul(1u64 << step);
    crate::interrupts::ticks_for_millis(millis.min(8000))
}

fn find_mut<'a>(services: &'a mut [Service], name: &str) -> Option<&'a mut Service> {
    services
        .iter_mut()
        .find(|service| service.name.eq_ignore_ascii_case(name))
}

fn state_snapshot(services: &[Service]) -> Vec<(&'static str, ServiceState)> {
    services
        .iter()
        .map(|service| (service.name, service.state))
        .collect()
}

fn dependencies_ready_from_states(
    service: &Service,
    states: &[(&'static str, ServiceState)],
) -> bool {
    service.dependencies.iter().all(|dep| {
        states
            .iter()
            .any(|(name, state)| name.eq_ignore_ascii_case(dep) && *state == ServiceState::Running)
    })
}

fn health_locked(services: &[Service], now: u64) -> ServiceHealth {
    let states = state_snapshot(services);
    let mut health = ServiceHealth {
        total: services.len(),
        running: 0,
        stopped: 0,
        failed: 0,
        degraded: 0,
        backoff: 0,
    };
    for service in services {
        match service.state {
            ServiceState::Running => health.running += 1,
            ServiceState::Stopped => health.stopped += 1,
            ServiceState::Failed => health.failed += 1,
        }
        let dependency_blocked = service.state == ServiceState::Running
            && !dependencies_ready_from_states(service, &states);
        if service.state != ServiceState::Running || dependency_blocked {
            health.degraded += 1;
        }
        if service.state == ServiceState::Failed && service.next_restart_tick > now {
            health.backoff += 1;
        }
    }
    health
}

fn health_summary_locked(services: &[Service], now: u64) -> String {
    let health = health_locked(services, now);
    format!(
        "services total={} running={} stopped={} failed={} degraded={} backoff={}",
        health.total,
        health.running,
        health.stopped,
        health.failed,
        health.degraded,
        health.backoff
    )
}

fn service_line(service: &Service, now: u64, include_order: bool) -> String {
    if include_order {
        format!(
            "{:02} {} state={} restart={} uid={} gid={} caps={} deps={} failures={} restarts={} loops={} reaped={} last_tick={} backoff={} reason={}",
            service.order,
            service.name,
            service.state.label(),
            service.restart,
            service.credentials.uid,
            service.credentials.gid,
            crate::security::capability_label(service.credentials.caps),
            deps_label(service.dependencies),
            service.failures,
            service.restarts,
            service.loops,
            service.reap_count,
            service.last_tick,
            backoff_label(service, now),
            service.last_reason
        )
    } else {
        format!(
            "{} state={} restart={} uid={} gid={} caps={} deps={} failures={} restarts={} loops={} reaped={} last_tick={} last_failure={} next_restart={} backoff={} reason={}",
            service.name,
            service.state.label(),
            service.restart,
            service.credentials.uid,
            service.credentials.gid,
            crate::security::capability_label(service.credentials.caps),
            deps_label(service.dependencies),
            service.failures,
            service.restarts,
            service.loops,
            service.reap_count,
            service.last_tick,
            service.last_failure_tick,
            service.next_restart_tick,
            backoff_label(service, now),
            service.last_reason
        )
    }
}

fn deps_label(deps: &[&str]) -> String {
    if deps.is_empty() {
        return String::from("none");
    }
    let mut out = String::new();
    for (idx, dep) in deps.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(dep);
    }
    out
}

fn backoff_label(service: &Service, now: u64) -> String {
    if service.state != ServiceState::Failed {
        return String::from("none");
    }
    if service.next_restart_tick <= now {
        return String::from("ready");
    }
    format!("{}t", service.next_restart_tick.saturating_sub(now))
}

fn persist_documents(reason: &str) {
    let now = crate::interrupts::ticks();
    let (config, history) = {
        let services = SERVICES.lock();
        serialized_documents_locked(&services, now, reason)
    };
    let _ = write_supervisor_files(&config, &history);
}

fn serialized_documents_locked(services: &[Service], now: u64, reason: &str) -> (String, String) {
    (
        serialize_config(services),
        serialize_history(services, now, reason),
    )
}

fn serialize_config(services: &[Service]) -> String {
    let mut out = String::from("coolOS service config\n# name state restart deps\n");
    for service in services {
        out.push_str(&format!(
            "{} state={} restart={} deps={}\n",
            service.name,
            service.state.label(),
            service.restart,
            deps_label(service.dependencies)
        ));
    }
    out
}

fn serialize_history(services: &[Service], now: u64, reason: &str) -> String {
    let mut out = String::from("coolOS service supervisor\n");
    out.push_str(&format!("reason={}\n", reason));
    out.push_str(&format!("tick={}\n", now));
    out.push_str(&format!("{}\n", health_summary_locked(services, now)));
    out.push_str(&format!("config={}\n", CONFIG_PATH));
    out.push_str(&format!("history={}\n", HISTORY_PATH));
    for service in services {
        out.push_str(&format!(
            "{} state={} restart={} deps={} failures={} restarts={} reaped={} loops={} last_tick={} last_failure={} next_restart={} reason={}\n",
            service.name,
            service.state.label(),
            service.restart,
            deps_label(service.dependencies),
            service.failures,
            service.restarts,
            service.reap_count,
            service.loops,
            service.last_tick,
            service.last_failure_tick,
            service.next_restart_tick,
            service.last_reason
        ));
    }
    out
}

fn write_supervisor_files(config: &str, history: &str) -> bool {
    let _ = crate::vfs::vfs_kernel_create_dir(CONFIG_DIR);
    let _ = crate::vfs::vfs_kernel_create_dir(LOG_DIR);
    let config_ok = crate::vfs::vfs_kernel_safe_write_file(CONFIG_PATH, config.as_bytes()).is_ok();
    let history_ok =
        crate::vfs::vfs_kernel_safe_write_file(HISTORY_PATH, history.as_bytes()).is_ok();
    config_ok && history_ok
}

fn service(
    name: &'static str,
    order: u8,
    restart: &'static str,
    dependencies: &'static [&'static str],
) -> Service {
    Service {
        name,
        order,
        restart,
        dependencies,
        state: ServiceState::Running,
        credentials: crate::security::service_credentials(name),
        restarts: 0,
        failures: 0,
        last_failure_tick: 0,
        next_restart_tick: 0,
        last_tick: 0,
        loops: 0,
        reap_count: 0,
        last_reason: "boot",
    }
}
