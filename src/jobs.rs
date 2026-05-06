extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

const MAX_JOBS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Paused,
    Cancelled,
    Complete,
    Failed,
}

impl JobState {
    pub const fn label(self) -> &'static str {
        match self {
            JobState::Running => "running",
            JobState::Paused => "paused",
            JobState::Cancelled => "cancelled",
            JobState::Complete => "complete",
            JobState::Failed => "failed",
        }
    }
}

#[derive(Clone)]
pub struct Job {
    pub id: u64,
    pub tick: u64,
    pub title: String,
    pub detail: String,
    pub progress: u8,
    pub state: JobState,
    pub process: Option<usize>,
}

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);
static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

pub fn start(title: &str, detail: &str) -> u64 {
    let id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
    let mut jobs = JOBS.lock();
    jobs.push(Job {
        id,
        tick: crate::interrupts::ticks(),
        title: String::from(title),
        detail: String::from(detail),
        progress: 0,
        state: JobState::Running,
        process: None,
    });
    if jobs.len() > MAX_JOBS {
        jobs.remove(0);
    }
    crate::event_bus::emit("jobs", "start", title);
    crate::wm::request_repaint();
    id
}

pub fn start_process(title: &str, detail: &str, pid: usize) -> u64 {
    let id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
    let mut jobs = JOBS.lock();
    jobs.push(Job {
        id,
        tick: crate::interrupts::ticks(),
        title: String::from(title),
        detail: String::from(detail),
        progress: 0,
        state: JobState::Running,
        process: Some(pid),
    });
    if jobs.len() > MAX_JOBS {
        jobs.remove(0);
    }
    crate::event_bus::emit("jobs", "start-process", title);
    crate::wm::request_repaint();
    id
}

pub fn complete(id: u64, detail: &str) {
    update(id, 100, JobState::Complete, detail);
}

pub fn progress(id: u64, progress: u8, detail: &str) {
    update(id, progress, JobState::Running, detail);
}

pub fn cancel(id: u64) -> bool {
    if let Some(pid) = process_for_job(id) {
        let _ = crate::scheduler::send_signal(pid, crate::process_model::Signal::Term);
    }
    set_state(id, JobState::Cancelled, "cancel requested")
}

pub fn pause(id: u64) -> bool {
    if let Some(pid) = process_for_job(id) {
        if crate::scheduler::send_signal(pid, crate::process_model::Signal::Stop).is_err() {
            return false;
        }
    }
    set_state(id, JobState::Paused, "paused")
}

pub fn resume(id: u64) -> bool {
    if let Some(pid) = process_for_job(id) {
        if crate::scheduler::send_signal(pid, crate::process_model::Signal::Continue).is_err() {
            return false;
        }
    }
    set_state(id, JobState::Running, "resume requested")
}

pub fn is_cancelled(id: u64) -> bool {
    JOBS.lock()
        .iter()
        .find(|job| job.id == id)
        .map(|job| job.state == JobState::Cancelled)
        .unwrap_or(false)
}

pub fn is_paused(id: u64) -> bool {
    JOBS.lock()
        .iter()
        .find(|job| job.id == id)
        .map(|job| job.state == JobState::Paused)
        .unwrap_or(false)
}

pub fn fail(id: u64, detail: &str) {
    update(id, 100, JobState::Failed, detail);
}

pub fn latest_id() -> Option<u64> {
    JOBS.lock().last().map(|job| job.id)
}

#[allow(dead_code)]
pub fn recent(limit: usize) -> Vec<Job> {
    refresh_process_jobs();
    let jobs = JOBS.lock();
    let start = jobs.len().saturating_sub(limit);
    jobs[start..].to_vec()
}

pub fn lines() -> Vec<String> {
    refresh_process_jobs();
    let jobs = JOBS.lock();
    if jobs.is_empty() {
        return alloc::vec![String::from("no background jobs")];
    }
    jobs.iter()
        .rev()
        .take(12)
        .map(|job| {
            if let Some(pid) = job.process {
                format!(
                    "#{} t={} {} {}% pid={} {} - {}",
                    job.id,
                    job.tick,
                    job.state.label(),
                    job.progress,
                    pid,
                    job.title,
                    job.detail
                )
            } else {
                format!(
                    "#{} t={} {} {}% {} - {}",
                    job.id,
                    job.tick,
                    job.state.label(),
                    job.progress,
                    job.title,
                    job.detail
                )
            }
        })
        .collect()
}

fn process_for_job(id: u64) -> Option<usize> {
    JOBS.lock()
        .iter()
        .find(|job| job.id == id)
        .and_then(|job| job.process)
}

fn refresh_process_jobs() {
    let mut jobs = JOBS.lock();
    for job in jobs.iter_mut() {
        let Some(pid) = job.process else {
            continue;
        };
        if matches!(
            job.state,
            JobState::Cancelled | JobState::Complete | JobState::Failed
        ) {
            continue;
        }
        let Some((status, exit_code)) = crate::scheduler::task_status_exit(pid) else {
            job.progress = 100;
            job.state = JobState::Failed;
            job.detail = String::from("process missing");
            continue;
        };
        match status {
            crate::scheduler::TaskStatus::Exited | crate::scheduler::TaskStatus::Reaped => {
                job.progress = 100;
                let code = exit_code.unwrap_or(0);
                if code == 0 {
                    job.state = JobState::Complete;
                    job.detail = String::from("process exited 0");
                } else if code == 130 || code == 143 {
                    job.state = JobState::Cancelled;
                    job.detail = format!("process signalled {}", code);
                } else {
                    job.state = JobState::Failed;
                    job.detail = format!("process exited {}", code);
                }
            }
            crate::scheduler::TaskStatus::Stopped => {
                job.state = JobState::Paused;
                job.detail = String::from("process stopped");
            }
            crate::scheduler::TaskStatus::Ready
            | crate::scheduler::TaskStatus::Running
            | crate::scheduler::TaskStatus::Blocked => {
                if job.state == JobState::Paused {
                    job.state = JobState::Running;
                    job.detail = String::from("process running");
                }
            }
        }
    }
}

fn update(id: u64, progress: u8, state: JobState, detail: &str) {
    let mut jobs = JOBS.lock();
    if let Some(job) = jobs.iter_mut().find(|job| job.id == id) {
        job.progress = progress.min(100);
        job.state = state;
        job.detail.clear();
        job.detail.push_str(detail);
        crate::event_bus::emit("jobs", state.label(), &job.title);
    }
    crate::wm::request_repaint();
}

fn set_state(id: u64, state: JobState, detail: &str) -> bool {
    let mut jobs = JOBS.lock();
    let Some(job) = jobs.iter_mut().find(|job| job.id == id) else {
        return false;
    };
    job.state = state;
    job.detail.clear();
    job.detail.push_str(detail);
    crate::event_bus::emit("jobs", state.label(), &job.title);
    crate::wm::request_repaint();
    true
}
