//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        if let Some(min_task) = self.ready_queue.front() {
            let mut min_stride = min_task.get_stride();
            let mut min_idx = 0;
            for (i, task) in self.ready_queue.iter().enumerate() {
                if task.get_stride() < min_stride {
                    min_stride = task.get_stride();
                    min_idx = i;
                }
            }
            return self.ready_queue.remove(min_idx);
        }
        // if this schedueling algorithm pass test, test gose wrong
        // if let Some(max_task) = self.ready_queue.front() {
        //     let mut max_stride = max_task.get_stride();
        //     let mut max_idx = 0;
        //     for (i, task) in self.ready_queue.iter().enumerate() {
        //         if task.get_stride() > max_stride {
        //             max_stride = task.get_stride();
        //             max_idx = i;
        //         }
        //     }
        //     return self.ready_queue.remove(max_idx);
        // }
        return None;
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
