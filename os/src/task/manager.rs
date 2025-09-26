//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use lazy_static::*;
use core::cmp::Ordering;

// Constant for stride scheduling
const BIG_STRIDE: usize = 0x7FFFFFFF;

/// Wrapper for TaskControlBlock to implement stride scheduling comparison
struct StrideTask {
    task: Arc<TaskControlBlock>,
    pass: usize,
}

impl StrideTask {
    fn new(task: Arc<TaskControlBlock>) -> Self {
        let pass = task.get_pass();
        Self { task, pass }
    }
}

impl PartialEq for StrideTask {
    fn eq(&self, other: &Self) -> bool {
        self.pass == other.pass
    }
}

impl Eq for StrideTask {}

impl PartialOrd for StrideTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StrideTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // For min-heap, we want smaller pass values first
        other.pass.cmp(&self.pass)
    }
}

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: BinaryHeap<StrideTask>,
}

/// A Stride scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(StrideTask::new(task));
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(stride_task) = self.ready_queue.pop() {
            let task = stride_task.task;
            let priority = task.get_priority();
            let stride = BIG_STRIDE / priority;
            let new_pass = task.get_pass() + stride;
            task.set_pass(new_pass);
            Some(task)
        } else {
            None
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
