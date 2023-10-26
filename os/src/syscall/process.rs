//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, get_task_control_block,
    },
    timer::{get_time_us, get_time_ms},
    mm::{VirtAddr, MapPermission},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let task_control_block = get_task_control_block();
    let time = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    let data = &time as *const TimeVal as usize;
    unsafe { (*task_control_block).memory_set.copyout(_ts as usize, data, core::mem::size_of::<TimeVal>()); }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let task_control_block = get_task_control_block();
    let info = unsafe { TaskInfo {
                    status: (*task_control_block).task_status,
                    syscall_times: (*task_control_block).syscall_times,
                    time: get_time_ms()-(*task_control_block).time,
                }
            };
    let data = &info as *const TaskInfo as usize;
    unsafe { (*task_control_block).memory_set.copyout(_ti as usize, data, core::mem::size_of::<TaskInfo>()); }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let va = VirtAddr::from(_start);
    if !va.aligned() {
        return -1;
    }
    if ((_port & !0x7) != 0) || ((_port & 0x7) == 0) {
        return -1;
    }
    let task_control_block = get_task_control_block();
    unsafe {
        if (*task_control_block).memory_set.range_intersect(_start, _len) {
            return -1;
        }
    }
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start+_len);
    let mut perm = MapPermission::U;
    if (_port & 0x1) != 0 {
        perm |= MapPermission::R;
    }
    if (_port & 0x2) != 0 {
        perm |= MapPermission::W;
    }
    if (_port & 0x4) != 0 {
        perm |= MapPermission::X;
    }
    unsafe { (*task_control_block).memory_set.insert_framed_area(start_va, end_va, perm); };
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let va = VirtAddr::from(_start);
    if !va.aligned() {
        return -1;
    }
    let task_control_block = get_task_control_block();
    unsafe {
        if !(*task_control_block).memory_set.unmap_range(_start, _len) {
            return -1;
        }
    }
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
