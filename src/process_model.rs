extern crate alloc;

use alloc::{format, string::String, vec::Vec};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Term,
    Int,
    User1,
    Stop,
    Continue,
}

impl Signal {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "term" | "TERM" | "sigterm" => Some(Signal::Term),
            "int" | "INT" | "sigint" => Some(Signal::Int),
            "usr1" | "USR1" | "user1" => Some(Signal::User1),
            "stop" | "STOP" | "sigstop" => Some(Signal::Stop),
            "cont" | "CONT" | "continue" | "sigcont" => Some(Signal::Continue),
            _ => None,
        }
    }

    pub const fn from_code(code: u64) -> Option<Self> {
        match code {
            2 => Some(Signal::Int),
            10 => Some(Signal::User1),
            15 => Some(Signal::Term),
            18 => Some(Signal::Continue),
            19 => Some(Signal::Stop),
            _ => None,
        }
    }

    pub const fn code(self) -> u64 {
        match self {
            Signal::Int => 2,
            Signal::User1 => 10,
            Signal::Term => 15,
            Signal::Continue => 18,
            Signal::Stop => 19,
        }
    }

    pub const fn exit_code(self) -> Option<u64> {
        match self {
            Signal::Term => Some(143),
            Signal::Int => Some(130),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Signal::Term => "TERM",
            Signal::Int => "INT",
            Signal::User1 => "USR1",
            Signal::Stop => "STOP",
            Signal::Continue => "CONT",
        }
    }
}

pub fn status_lines() -> Vec<String> {
    let sched = crate::scheduler::SCHEDULER.lock();
    let mut lines = Vec::new();
    for (pid, task) in sched.tasks.iter().enumerate() {
        let mut parent = String::new();
        if let Some(id) = task.parent {
            push_usize(&mut parent, id);
        } else {
            parent.push('-');
        }
        let mut wake = String::new();
        if let Some(tick) = task.wake_tick {
            wake.push('@');
            push_usize(&mut wake, tick as usize);
        } else {
            wake.push('-');
        }
        lines.push(format!(
            "pid={} ppid={} pgid={} uid={} gid={} caps={} signal={} wake={} status={:?} name={}",
            pid,
            parent,
            task.process_group,
            task.credentials.uid,
            task.credentials.gid,
            crate::security::capability_label(task.credentials.caps),
            task.pending_signal
                .map(|signal| signal.label())
                .unwrap_or("-"),
            wake,
            task.status,
            task.name
        ));
    }
    lines
}

pub fn zombie_policy_lines() -> Vec<String> {
    alloc::vec![
        String::from("policy: exited children remain zombies until waitpid/reap"),
        String::from("shell reap command may reap all exited tasks"),
        String::from("service supervisor restarts failed service work under service credentials"),
        String::from(
            "signals: TERM/INT exit, STOP removes from run queue, CONT resumes stopped tasks"
        ),
    ]
}

pub fn signal_selftest_passes() -> bool {
    Signal::from_code(Signal::Term.code()) == Some(Signal::Term)
        && Signal::from_code(Signal::Int.code()) == Some(Signal::Int)
        && Signal::from_code(Signal::User1.code()) == Some(Signal::User1)
        && Signal::from_code(Signal::Stop.code()) == Some(Signal::Stop)
        && Signal::from_code(Signal::Continue.code()) == Some(Signal::Continue)
        && Signal::parse("stop") == Some(Signal::Stop)
        && Signal::parse("cont") == Some(Signal::Continue)
        && Signal::Term.exit_code() == Some(143)
        && Signal::Int.exit_code() == Some(130)
}

fn push_usize(out: &mut String, mut value: usize) {
    if value == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for idx in (0..len).rev() {
        out.push(digits[idx] as char);
    }
}
