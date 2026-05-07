/// Preemptive round-robin scheduler for cool-os (Phase 8).
///
/// The timer ISR saves 15 GP registers on top of the CPU's 5-word interrupt
/// frame, giving a 20-word (160-byte) context block on the current task's
/// stack.  `timer_schedule` is called with the RSP value that points to the
/// bottom of that block and returns the RSP of whichever task should run next.
///
/// Stack layout (each slot = 8 bytes, lower address = lower index):
///
///   [stack_ptr +   0]  r15   ← stack_ptr points here (pushed last by ISR)
///   [stack_ptr +   8]  r14
///   [stack_ptr +  16]  r13
///   [stack_ptr +  24]  r12
///   [stack_ptr +  32]  r11
///   [stack_ptr +  40]  r10
///   [stack_ptr +  48]  r9
///   [stack_ptr +  56]  r8
///   [stack_ptr +  64]  rbp
///   [stack_ptr +  72]  rdi
///   [stack_ptr +  80]  rsi
///   [stack_ptr +  88]  rdx
///   [stack_ptr +  96]  rcx
///   [stack_ptr + 104]  rbx
///   [stack_ptr + 112]  rax   (pushed first by ISR)
///   [stack_ptr + 120]  RIP   ← CPU interrupt frame begins here
///   [stack_ptr + 128]  CS
///   [stack_ptr + 136]  RFLAGS
///   [stack_ptr + 144]  RSP   (task's stack pointer restored by iretq)
///   [stack_ptr + 152]  SS
use alloc::{format, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::paging::PhysFrame;

extern crate alloc;

// ── Public counter — incremented by the counter_task demo ────────────────────

pub static BACKGROUND_COUNTER: AtomicU64 = AtomicU64::new(0);

// ── Constants ─────────────────────────────────────────────────────────────────

/// Size of each task's private kernel stack (64 KiB).
const STACK_SIZE: usize = 64 * 1024;

// ── TaskStatus ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskStatus {
    Ready,
    Running,
    Blocked,
    Stopped,
    Exited,
    Reaped,
}

// ── Task ──────────────────────────────────────────────────────────────────────

pub struct Task {
    /// Human-readable name (for debugging).
    #[allow(dead_code)]
    pub name: &'static str,
    /// Heap-allocated kernel stack.  Empty for the idle task (uses boot stack).
    #[allow(dead_code)]
    stack: Vec<u8>,
    /// Top of the private kernel stack used for syscall entry on this task.
    pub syscall_stack_top: usize,
    /// Saved RSP — the address of the bottom of the 20-word context block.
    /// For the idle task this starts as 0 and is filled in on the first timer
    /// preemption.
    pub stack_ptr: usize,
    pub status: TaskStatus,
    pub exit_code: Option<u64>,
    pub parent: Option<usize>,
    pub process_group: usize,
    pub controlling_tty: Option<u64>,
    pub pending_signal: Option<crate::process_model::Signal>,
    pub wake_tick: Option<u64>,
    pub credentials: crate::security::Credentials,
    /// Per-process PML4 frame.  None = kernel task, shares the boot PML4.
    pub pml4: Option<PhysFrame>,
}

// ── Scheduler ─────────────────────────────────────────────────────────────────

pub struct Scheduler {
    pub tasks: Vec<Task>,
    /// Index of the currently running task.
    pub current: usize,
}

impl Scheduler {
    /// Create an empty scheduler.  `const fn` so the global static can be
    /// initialised without a heap allocation (Vec::new() is allocation-free).
    pub const fn empty() -> Self {
        Scheduler {
            tasks: Vec::new(),
            current: 0,
        }
    }

    /// Register the idle task (index 0).
    ///
    /// The idle task represents the current kernel boot stack; we don't
    /// allocate a separate stack for it.  Its `stack_ptr` will be filled in
    /// the first time the timer preempts it.
    pub fn add_idle(&mut self) {
        self.tasks.push(Task {
            name: "idle",
            stack: Vec::new(),
            syscall_stack_top: 0,
            stack_ptr: 0,
            status: TaskStatus::Running,
            exit_code: None,
            parent: None,
            process_group: 0,
            controlling_tty: None,
            pending_signal: None,
            wake_tick: None,
            credentials: crate::security::interactive_credentials(),
            pml4: None,
        });
        crate::vfs::init_task(0);
        CURRENT_SYSCALL_STACK_TOP.store(0, Ordering::Relaxed);
        crate::gdt::set_privilege_stack_top(crate::gdt::default_privilege_stack_top());
    }

    fn spawn_context(
        &mut self,
        name: &'static str,
        rip: u64,
        cs: u64,
        rsp: Option<u64>,
        ss: u64,
        pml4: Option<PhysFrame>,
    ) -> usize {
        // Allocate and zero-initialise the stack buffer.
        let mut stack: Vec<u8> = Vec::new();
        stack.resize(STACK_SIZE, 0u8);
        crate::slab::record_alloc("task-stack", STACK_SIZE);

        // Round the top of the buffer down to a 16-byte boundary so that the
        // stack pointer is properly aligned for the System V AMD64 ABI.
        let stack_top = (stack.as_ptr() as usize + STACK_SIZE) & !0xf;

        // The saved RSP is the bottom of the 20-word (160-byte) context block.
        let stack_ptr_addr = stack_top - 20 * 8; // stack_top - 160

        // Populate the context block.
        //
        // frame[0..15]  → GP registers (r15 first, rax last) — all 0
        // frame[15]     → RIP  (task entry point)
        // frame[16]     → CS
        // frame[17]     → RFLAGS: IF=1 (bit 9) + reserved bit 1 = 0x202
        // frame[18]     → RSP
        // frame[19]     → SS
        //
        // SAFETY: stack_ptr_addr is 16-byte aligned (stack_top is 16-byte
        // aligned and 160 = 10×16), and the entire 160-byte range lies within
        // the allocated Vec buffer.
        let frame = unsafe { core::slice::from_raw_parts_mut(stack_ptr_addr as *mut u64, 20) };

        for slot in frame[0..15].iter_mut() {
            *slot = 0;
        }
        frame[15] = rip;
        frame[16] = cs; // CS
        frame[17] = 0x202; // RFLAGS: IF=1, reserved bit 1
        frame[18] = rsp.unwrap_or((stack_top - 8) as u64);
        frame[19] = ss; // SS

        let parent = if self.tasks.is_empty() {
            None
        } else {
            Some(self.current)
        };
        let credentials = parent
            .and_then(|parent_id| self.tasks.get(parent_id).map(|task| task.credentials))
            .unwrap_or_else(crate::security::interactive_credentials);
        let process_group = parent
            .and_then(|parent_id| self.tasks.get(parent_id).map(|task| task.process_group))
            .unwrap_or_else(|| self.tasks.len());
        let controlling_tty = parent.and_then(|parent_id| {
            self.tasks
                .get(parent_id)
                .and_then(|task| task.controlling_tty)
        });
        self.tasks.push(Task {
            name,
            stack,
            syscall_stack_top: stack_top,
            stack_ptr: stack_ptr_addr,
            status: TaskStatus::Ready,
            exit_code: None,
            parent,
            process_group,
            controlling_tty,
            pending_signal: None,
            wake_tick: None,
            credentials,
            pml4,
        });
        let task_id = self.tasks.len() - 1;
        crate::vfs::init_task(task_id);
        task_id
    }

    /// Allocate a 64 KiB kernel stack for a new task, pre-populate its saved
    /// context so that the first `iretq` begins execution at `entry`, and
    /// add the task to the run queue as `Ready`.
    /// Spawn a kernel-mode task (shares the boot PML4, ring 0).
    #[allow(dead_code)]
    pub fn spawn(&mut self, name: &'static str, entry: fn() -> !) {
        self.spawn_with_pml4(name, entry, None);
    }

    /// Spawn a task with an optional private PML4.  When `pml4` is `Some`,
    /// the scheduler loads it into CR3 whenever this task is scheduled.
    pub fn spawn_with_pml4(
        &mut self,
        name: &'static str,
        entry: fn() -> !,
        pml4: Option<PhysFrame>,
    ) {
        // Read the current kernel selectors. These must match exactly what the
        // CPU expects for a ring-0 iretq frame.
        let cs: u64;
        let ss: u64;
        unsafe {
            core::arch::asm!("mov {0:x}, cs", out(reg) cs);
            core::arch::asm!("mov {0:x}, ss", out(reg) ss);
        }
        self.spawn_context(name, entry as usize as u64, cs, None, ss, pml4);
    }

    /// Spawn a ring-3 task that will enter at `entry` with the given user stack.
    #[allow(dead_code)]
    pub fn spawn_user(&mut self, name: &'static str, entry: u64, user_rsp: u64, pml4: PhysFrame) {
        self.spawn_user_with_fds(name, entry, user_rsp, pml4, &[]);
    }

    /// Spawn a ring-3 task and selectively inherit fd mappings from the
    /// currently running task.
    pub fn spawn_user_with_fds(
        &mut self,
        name: &'static str,
        entry: u64,
        user_rsp: u64,
        pml4: PhysFrame,
        inherited_fds: &[(usize, usize)],
    ) -> bool {
        self.spawn_user_with_fds_and_credentials(name, entry, user_rsp, pml4, inherited_fds, None)
    }

    pub fn spawn_user_with_fds_and_credentials(
        &mut self,
        name: &'static str,
        entry: u64,
        user_rsp: u64,
        pml4: PhysFrame,
        inherited_fds: &[(usize, usize)],
        credentials: Option<crate::security::Credentials>,
    ) -> bool {
        let user_cs = crate::gdt::user_code_selector().0 as u64;
        let user_ss = crate::gdt::user_data_selector().0 as u64;
        let parent = self.current;
        let task_id = self.spawn_context(name, entry, user_cs, Some(user_rsp), user_ss, Some(pml4));
        if let Some(credentials) = credentials {
            if let Some(task) = self.tasks.get_mut(task_id) {
                task.credentials = credentials;
            }
        }
        if crate::vfs::inherit_fds(parent, task_id, inherited_fds) {
            true
        } else {
            crate::vfs::drop_task(task_id);
            self.tasks.pop();
            crate::vmm::free_address_space(pml4);
            false
        }
    }

    /// Core round-robin scheduling decision.
    ///
    /// 1. Saves `current_rsp` as the current task's stack pointer and marks
    ///    it `Ready` (if it was `Running`).
    /// 2. Searches forward (wrapping) for the next `Ready` task.
    /// 3. Falls back to task 0 (idle) if none found.
    /// 4. Marks the winner `Running`, updates `self.current`, and returns its
    ///    saved stack pointer.
    pub fn schedule(&mut self, current_rsp: usize) -> usize {
        if self.tasks.is_empty() {
            return current_rsp;
        }

        // ── Save the current task ────────────────────────────────────────────
        let cur = self.current;
        self.tasks[cur].stack_ptr = current_rsp;
        if self.tasks[cur].status == TaskStatus::Running {
            self.tasks[cur].status = TaskStatus::Ready;
        }

        let now = crate::interrupts::ticks();
        for (idx, task) in self.tasks.iter_mut().enumerate() {
            if task.status == TaskStatus::Blocked
                && task.wake_tick.map(|wake| now >= wake).unwrap_or(false)
            {
                task.wake_tick = None;
                task.status = TaskStatus::Ready;
            }
            if let Some(signal) = task.pending_signal.take() {
                match signal {
                    crate::process_model::Signal::User1 => {
                        task.wake_tick = None;
                        if task.status == TaskStatus::Blocked {
                            task.status = TaskStatus::Ready;
                        }
                    }
                    crate::process_model::Signal::Stop => {
                        if idx != 0
                            && task.status != TaskStatus::Exited
                            && task.status != TaskStatus::Reaped
                        {
                            task.status = TaskStatus::Stopped;
                            task.wake_tick = None;
                        }
                    }
                    crate::process_model::Signal::Continue => {
                        if task.status == TaskStatus::Stopped {
                            task.status = TaskStatus::Ready;
                        }
                    }
                    crate::process_model::Signal::Term | crate::process_model::Signal::Int => {}
                }
            }
        }

        // ── Find the next Ready task (round-robin) ───────────────────────────
        let n = self.tasks.len();
        let mut next = (cur + 1) % n;
        let mut found = false;
        for _ in 0..n {
            if self.tasks[next].status == TaskStatus::Ready {
                found = true;
                break;
            }
            next = (next + 1) % n;
        }
        if !found {
            // No runnable task — fall back to the idle task.
            next = 0;
        }

        // ── Activate the winner ──────────────────────────────────────────────
        self.tasks[next].status = TaskStatus::Running;
        self.current = next;
        CURRENT_SYSCALL_STACK_TOP
            .store(self.tasks[next].syscall_stack_top as u64, Ordering::Relaxed);
        let privilege_stack_top = if self.tasks[next].syscall_stack_top == 0 {
            crate::gdt::default_privilege_stack_top()
        } else {
            self.tasks[next].syscall_stack_top as u64
        };
        crate::gdt::set_privilege_stack_top(privilege_stack_top);

        // Switch address space: load the winning task's PML4, or restore the
        // boot PML4 for kernel tasks (pml4=None) so they never run with a
        // user process's address space accidentally loaded.
        match self.tasks[next].pml4 {
            Some(pml4) => unsafe { crate::vmm::switch_to(pml4) },
            None => unsafe { crate::vmm::switch_to_boot() },
        }

        self.tasks[next].stack_ptr
    }
}

// ── Global scheduler instance ─────────────────────────────────────────────────

pub static SCHEDULER: spin::Mutex<Scheduler> = spin::Mutex::new(Scheduler::empty());
pub static CURRENT_SYSCALL_STACK_TOP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum KillError {
    CannotKillIdle,
    CannotKillCurrent,
    InvalidTask,
    AlreadyExited,
    AlreadyReaped,
    NotChild,
    PermissionDenied,
}

impl KillError {
    pub const fn as_str(self) -> &'static str {
        match self {
            KillError::CannotKillIdle => "cannot kill idle task",
            KillError::CannotKillCurrent => "cannot kill current task",
            KillError::InvalidTask => "no such task",
            KillError::AlreadyExited => "task already exited",
            KillError::AlreadyReaped => "task already reaped",
            KillError::NotChild => "not a child task",
            KillError::PermissionDenied => "permission denied",
        }
    }
}

pub enum WaitError {
    InvalidTask,
    NotChild,
    NotExited,
    AlreadyReaped,
}

impl WaitError {
    pub const fn as_str(self) -> &'static str {
        match self {
            WaitError::InvalidTask => "no such task",
            WaitError::NotChild => "not a child task",
            WaitError::NotExited => "task has not exited",
            WaitError::AlreadyReaped => "task already reaped",
        }
    }
}

pub enum SignalError {
    InvalidTask,
    InvalidGroup,
    CannotSignalIdle,
    PermissionDenied,
    AlreadyExited,
    AlreadyReaped,
}

impl SignalError {
    pub const fn as_str(self) -> &'static str {
        match self {
            SignalError::InvalidTask => "no such task",
            SignalError::InvalidGroup => "no such process group",
            SignalError::CannotSignalIdle => "cannot signal idle task",
            SignalError::PermissionDenied => "permission denied",
            SignalError::AlreadyExited => "task already exited",
            SignalError::AlreadyReaped => "task already reaped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessGroupStatus {
    Empty,
    Running,
    Stopped,
    Exited,
}

// ── Blocking helpers ─────────────────────────────────────────────────────────

pub fn current_task_id() -> usize {
    SCHEDULER.lock().current
}

pub fn current_credentials() -> Option<crate::security::Credentials> {
    let sched = SCHEDULER.lock();
    sched.tasks.get(sched.current).map(|task| task.credentials)
}

pub fn set_current_credentials(credentials: crate::security::Credentials) {
    let mut sched = SCHEDULER.lock();
    let current = sched.current;
    if let Some(task) = sched.tasks.get_mut(current) {
        task.credentials = credentials;
    }
}

#[allow(dead_code)]
pub fn task_credentials(task_id: usize) -> Option<crate::security::Credentials> {
    SCHEDULER
        .lock()
        .tasks
        .get(task_id)
        .map(|task| task.credentials)
}

fn can_control_task_locked(sched: &Scheduler, actor: usize, target: usize) -> bool {
    if actor == target {
        return true;
    }
    let Some(actor_task) = sched.tasks.get(actor) else {
        return false;
    };
    let Some(target_task) = sched.tasks.get(target) else {
        return false;
    };
    crate::security::can_admin(actor_task.credentials)
        || target_task.parent == Some(actor)
        || actor_task.credentials.uid == target_task.credentials.uid
}

pub fn task_name(task_id: usize) -> Option<&'static str> {
    SCHEDULER.lock().tasks.get(task_id).map(|task| task.name)
}

pub fn task_status_exit(task_id: usize) -> Option<(TaskStatus, Option<u64>)> {
    SCHEDULER
        .lock()
        .tasks
        .get(task_id)
        .map(|task| (task.status, task.exit_code))
}

pub fn current_process_group() -> usize {
    let sched = SCHEDULER.lock();
    sched
        .tasks
        .get(sched.current)
        .map(|task| task.process_group)
        .unwrap_or(0)
}

pub fn current_tty() -> Option<u64> {
    let sched = SCHEDULER.lock();
    sched
        .tasks
        .get(sched.current)
        .and_then(|task| task.controlling_tty)
}

pub fn set_task_tty(task_id: usize, tty: Option<u64>) -> Result<(), SignalError> {
    let actor = {
        let sched = SCHEDULER.lock();
        sched.current
    };
    set_task_tty_as(actor, task_id, tty)
}

pub fn set_task_tty_as(actor: usize, task_id: usize, tty: Option<u64>) -> Result<(), SignalError> {
    if task_id == 0 {
        return Err(SignalError::CannotSignalIdle);
    }
    let mut sched = SCHEDULER.lock();
    if sched.tasks.get(task_id).is_none() {
        return Err(SignalError::InvalidTask);
    }
    if !can_control_task_locked(&sched, actor, task_id) {
        return Err(SignalError::PermissionDenied);
    }
    let task = sched
        .tasks
        .get_mut(task_id)
        .ok_or(SignalError::InvalidTask)?;
    if task.status == TaskStatus::Exited {
        return Err(SignalError::AlreadyExited);
    }
    if task.status == TaskStatus::Reaped {
        return Err(SignalError::AlreadyReaped);
    }
    task.controlling_tty = tty;
    Ok(())
}

pub fn process_group_status(group: usize) -> ProcessGroupStatus {
    let sched = SCHEDULER.lock();
    let mut saw_stopped = false;
    let mut saw_exited = false;
    for (idx, task) in sched.tasks.iter().enumerate() {
        if idx == 0 || task.process_group != group {
            continue;
        }
        match task.status {
            TaskStatus::Ready | TaskStatus::Running | TaskStatus::Blocked => {
                return ProcessGroupStatus::Running;
            }
            TaskStatus::Stopped => saw_stopped = true,
            TaskStatus::Exited | TaskStatus::Reaped => saw_exited = true,
        }
    }
    if saw_stopped {
        ProcessGroupStatus::Stopped
    } else if saw_exited {
        ProcessGroupStatus::Exited
    } else {
        ProcessGroupStatus::Empty
    }
}

pub fn process_group_exit_code(group: usize) -> Option<u64> {
    let sched = SCHEDULER.lock();
    sched
        .tasks
        .iter()
        .find(|task| {
            task.process_group == group
                && matches!(task.status, TaskStatus::Exited | TaskStatus::Reaped)
        })
        .and_then(|task| task.exit_code)
}

pub fn current_task_blocked() -> bool {
    let sched = SCHEDULER.lock();
    sched
        .tasks
        .get(sched.current)
        .map(|task| task.status == TaskStatus::Blocked)
        .unwrap_or(false)
}

pub fn current_has_pending_signal() -> bool {
    let sched = SCHEDULER.lock();
    sched
        .tasks
        .get(sched.current)
        .map(|task| task.pending_signal.is_some())
        .unwrap_or(false)
}

pub fn block_current() {
    let mut sched = SCHEDULER.lock();
    let cur = sched.current;
    if let Some(task) = sched.tasks.get_mut(cur) {
        task.status = TaskStatus::Blocked;
    }
}

pub fn block_current_until(wake_tick: u64) {
    let mut sched = SCHEDULER.lock();
    let cur = sched.current;
    if let Some(task) = sched.tasks.get_mut(cur) {
        task.status = TaskStatus::Blocked;
        task.wake_tick = Some(wake_tick);
    }
}

pub fn unblock(task_id: usize) {
    let mut sched = SCHEDULER.lock();
    if let Some(task) = sched.tasks.get_mut(task_id) {
        if task.status == TaskStatus::Blocked {
            task.wake_tick = None;
            task.status = TaskStatus::Ready;
        }
    }
}

pub fn exit_current(code: u64) {
    let (task_id, name, parent) = {
        let sched = SCHEDULER.lock();
        let task_id = sched.current;
        let task = sched.tasks.get(task_id);
        let name = task.map(|task| task.name).unwrap_or("task");
        let parent = task.and_then(|task| task.parent);
        (task_id, name, parent)
    };

    // Keep the exiting task schedulable while cleanup may allocate or take
    // locks. If it is marked Exited first, a timer tick can switch it out
    // permanently in the middle of cleanup and strand a held lock.
    crate::vfs::drop_task(task_id);
    crate::wm::close_user_gui_windows_for_owner(task_id);
    crate::profiler::record_task(task_id, name, "exited");
    crate::crashdump::record_task_report(task_id, "task exited");
    crate::notifications::push_transient("Task exited", &format!("pid {} exit {}", task_id, code));
    crate::app_lifecycle::record_process_exit(task_id, &format!("exit {}", code));
    crate::deferred::enqueue(crate::deferred::DeferredWork::PersistTaskSnapshot);
    crate::deferred::enqueue(crate::deferred::DeferredWork::FlushKernelLog);

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();
        if let Some(task) = sched.tasks.get_mut(task_id) {
            task.status = TaskStatus::Exited;
            task.exit_code = Some(code);
            task.wake_tick = None;
        }
    });
    wake_waiting_parent(parent);
}

pub fn kill_task(task_id: usize, code: u64) -> Result<(), KillError> {
    if task_id == 0 {
        return Err(KillError::CannotKillIdle);
    }

    let current = {
        let sched = SCHEDULER.lock();
        sched.current
    };
    if task_id == current {
        return Err(KillError::CannotKillCurrent);
    }

    {
        let sched = SCHEDULER.lock();
        let Some(task) = sched.tasks.get(task_id) else {
            return Err(KillError::InvalidTask);
        };
        if !can_control_task_locked(&sched, current, task_id) {
            return Err(KillError::PermissionDenied);
        }
        if task.status == TaskStatus::Exited {
            return Err(KillError::AlreadyExited);
        }
        if task.status == TaskStatus::Reaped {
            return Err(KillError::AlreadyReaped);
        }
    }

    let (name, parent) = {
        let mut sched = SCHEDULER.lock();
        let task = sched.tasks.get_mut(task_id).ok_or(KillError::InvalidTask)?;
        let name = task.name;
        let parent = task.parent;
        task.status = TaskStatus::Exited;
        task.exit_code = Some(code);
        task.wake_tick = None;
        (name, parent)
    };
    wake_waiting_parent(parent);
    crate::vfs::drop_task(task_id);
    crate::wm::close_user_gui_windows_for_owner(task_id);
    crate::profiler::record_task(task_id, name, "killed");
    crate::crashdump::record_task_report(task_id, "task killed");
    crate::notifications::push_transient("Task killed", &format!("pid {} exit {}", task_id, code));
    crate::app_lifecycle::record_process_exit(task_id, &format!("killed {}", code));
    crate::deferred::enqueue(crate::deferred::DeferredWork::PersistTaskSnapshot);
    crate::deferred::enqueue(crate::deferred::DeferredWork::FlushKernelLog);
    Ok(())
}

pub fn fault_current(code: u64, reason: &'static str) -> usize {
    let (task_id, name, parent) = {
        let mut sched = SCHEDULER.lock();
        let task_id = sched.current;
        let task = sched.tasks.get(task_id);
        let name = task.map(|task| task.name).unwrap_or("task");
        let parent = task.and_then(|task| task.parent);
        if let Some(task) = sched.tasks.get_mut(task_id) {
            task.status = TaskStatus::Exited;
            task.exit_code = Some(code);
        }
        (task_id, name, parent)
    };
    wake_waiting_parent(parent);
    crate::vfs::drop_task(task_id);
    crate::wm::close_user_gui_windows_for_owner(task_id);
    crate::profiler::record_task(task_id, name, reason);
    crate::crashdump::record_task_report(task_id, reason);
    crate::notifications::push_transient("Task faulted", &format!("pid {} {}", task_id, reason));
    crate::app_lifecycle::record_process_exit(task_id, &format!("fault {} {}", code, reason));
    crate::deferred::enqueue(crate::deferred::DeferredWork::PersistTaskSnapshot);
    crate::deferred::enqueue(crate::deferred::DeferredWork::FlushKernelLog);
    task_id
}

pub fn send_signal(
    task_id: usize,
    signal: crate::process_model::Signal,
) -> Result<(), SignalError> {
    if task_id == 0 {
        return Err(SignalError::CannotSignalIdle);
    }
    if let Some(code) = signal.exit_code() {
        let current = {
            let sched = SCHEDULER.lock();
            sched.current
        };
        if task_id == current {
            exit_current(code);
            return Ok(());
        }
        return kill_task(task_id, code).map_err(signal_error_from_kill);
    }

    {
        let mut sched = SCHEDULER.lock();
        let current = sched.current;
        if sched.tasks.get(task_id).is_none() {
            return Err(SignalError::InvalidTask);
        }
        if !can_control_task_locked(&sched, current, task_id) {
            return Err(SignalError::PermissionDenied);
        }
        let task = sched
            .tasks
            .get_mut(task_id)
            .ok_or(SignalError::InvalidTask)?;
        if task.status == TaskStatus::Exited {
            return Err(SignalError::AlreadyExited);
        }
        if task.status == TaskStatus::Reaped {
            return Err(SignalError::AlreadyReaped);
        }
        match signal {
            crate::process_model::Signal::User1 => {
                task.pending_signal = Some(signal);
                if task.status == TaskStatus::Blocked {
                    task.wake_tick = None;
                    task.status = TaskStatus::Ready;
                }
            }
            crate::process_model::Signal::Stop => {
                task.pending_signal = None;
                task.wake_tick = None;
                task.status = TaskStatus::Stopped;
            }
            crate::process_model::Signal::Continue => {
                task.pending_signal = None;
                if task.status == TaskStatus::Stopped {
                    task.status = TaskStatus::Ready;
                }
            }
            crate::process_model::Signal::Term | crate::process_model::Signal::Int => {}
        }
    }
    crate::notifications::push_transient(
        "Signal delivered",
        &format!("pid {} {}", task_id, signal.label()),
    );
    Ok(())
}

pub fn send_signal_to_group(
    group: usize,
    signal: crate::process_model::Signal,
) -> Result<usize, SignalError> {
    let ids = {
        let sched = SCHEDULER.lock();
        let current = sched.current;
        let ids: Vec<usize> = sched
            .tasks
            .iter()
            .enumerate()
            .filter(|(idx, task)| {
                *idx != 0
                    && task.process_group == group
                    && task.status != TaskStatus::Exited
                    && task.status != TaskStatus::Reaped
                    && can_control_task_locked(&sched, current, *idx)
            })
            .map(|(idx, _)| idx)
            .collect();
        ids
    };
    if ids.is_empty() {
        return Err(SignalError::InvalidGroup);
    }
    let mut delivered = 0usize;
    for id in ids {
        if send_signal(id, signal).is_ok() {
            delivered += 1;
        }
    }
    if delivered == 0 {
        Err(SignalError::PermissionDenied)
    } else {
        Ok(delivered)
    }
}

pub fn set_process_group(task_id: usize, group: usize) -> Result<(), SignalError> {
    let current = {
        let sched = SCHEDULER.lock();
        sched.current
    };
    set_process_group_as(current, task_id, group)
}

pub fn set_process_group_as(actor: usize, task_id: usize, group: usize) -> Result<(), SignalError> {
    if task_id == 0 {
        return Err(SignalError::CannotSignalIdle);
    }
    let mut sched = SCHEDULER.lock();
    if sched.tasks.get(task_id).is_none() {
        return Err(SignalError::InvalidTask);
    }
    if !can_control_task_locked(&sched, actor, task_id) {
        return Err(SignalError::PermissionDenied);
    }
    let task = sched
        .tasks
        .get_mut(task_id)
        .ok_or(SignalError::InvalidTask)?;
    if task.status == TaskStatus::Exited {
        return Err(SignalError::AlreadyExited);
    }
    if task.status == TaskStatus::Reaped {
        return Err(SignalError::AlreadyReaped);
    }
    task.process_group = group;
    Ok(())
}

pub fn get_process_group(task_id: usize) -> Result<usize, SignalError> {
    let sched = SCHEDULER.lock();
    let task = sched.tasks.get(task_id).ok_or(SignalError::InvalidTask)?;
    if task.status == TaskStatus::Reaped {
        return Err(SignalError::AlreadyReaped);
    }
    Ok(task.process_group)
}

fn signal_error_from_kill(err: KillError) -> SignalError {
    match err {
        KillError::CannotKillIdle => SignalError::CannotSignalIdle,
        KillError::InvalidTask => SignalError::InvalidTask,
        KillError::AlreadyExited => SignalError::AlreadyExited,
        KillError::AlreadyReaped => SignalError::AlreadyReaped,
        KillError::PermissionDenied => SignalError::PermissionDenied,
        KillError::CannotKillCurrent | KillError::NotChild => SignalError::InvalidTask,
    }
}

pub fn waitpid(parent: usize, task_id: usize) -> Result<u64, WaitError> {
    if task_id == 0 {
        return Err(WaitError::InvalidTask);
    }
    let mut pml4_to_free = None;
    let result = {
        let mut sched = SCHEDULER.lock();
        let task = sched.tasks.get_mut(task_id).ok_or(WaitError::InvalidTask)?;
        if task.parent != Some(parent) && parent != 0 {
            return Err(WaitError::NotChild);
        }
        match task.status {
            TaskStatus::Exited => {
                let code = task.exit_code.unwrap_or(0);
                task.status = TaskStatus::Reaped;
                task.stack.clear();
                crate::slab::record_free("task-stack", STACK_SIZE);
                task.stack_ptr = 0;
                task.syscall_stack_top = 0;
                pml4_to_free = task.pml4.take();
                Ok(code)
            }
            TaskStatus::Reaped => Err(WaitError::AlreadyReaped),
            _ => Err(WaitError::NotExited),
        }
    };
    if result.is_ok() {
        if let Some(pml4) = pml4_to_free {
            crate::vmm::free_address_space(pml4);
        }
        crate::deferred::enqueue(crate::deferred::DeferredWork::PersistTaskSnapshot);
    }
    result
}

fn wake_waiting_parent(parent: Option<usize>) {
    if let Some(parent) = parent {
        crate::wait_queue::wake("waitpid", parent);
        unblock(parent);
    }
}

pub fn reap_all_exited(parent: usize) -> usize {
    let mut pml4s_to_free = Vec::new();
    let count = {
        let mut sched = SCHEDULER.lock();
        let mut count = 0usize;
        for (idx, task) in sched.tasks.iter_mut().enumerate() {
            if idx == 0 || task.status != TaskStatus::Exited {
                continue;
            }
            if task.parent != Some(parent) && parent != 0 {
                continue;
            }
            task.status = TaskStatus::Reaped;
            task.stack.clear();
            crate::slab::record_free("task-stack", STACK_SIZE);
            task.stack_ptr = 0;
            task.syscall_stack_top = 0;
            if let Some(pml4) = task.pml4.take() {
                pml4s_to_free.push(pml4);
            }
            count += 1;
        }
        count
    };
    if count > 0 {
        for pml4 in pml4s_to_free {
            crate::vmm::free_address_space(pml4);
        }
        crate::deferred::enqueue(crate::deferred::DeferredWork::PersistTaskSnapshot);
    }
    count
}

// ── Timer ISR entry point (called from timer_naked in interrupts.rs) ──────────

/// Called from the naked timer ISR with `current_rsp` equal to RSP after
/// the ISR has pushed all 15 GP registers.  Returns the RSP of the task that
/// should run next.
///
/// Handles the empty-task-list case gracefully (returns `current_rsp`
/// unchanged) so that timer preemptions before `add_idle` / `spawn` are
/// harmless.
///
/// # Safety
/// Must only be called from the naked timer ISR with all GP registers already
/// pushed onto the stack and interrupts disabled by the CPU.
pub unsafe extern "C" fn timer_schedule(current_rsp: usize) -> usize {
    let Some(mut sched) = SCHEDULER.try_lock() else {
        return current_rsp;
    };
    if sched.tasks.is_empty() {
        return current_rsp;
    }
    sched.schedule(current_rsp)
}
