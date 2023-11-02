use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        process_inner.work_list[id] = 1;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.work_list.push(1);
        if process_inner.need_list.is_empty() {
            let len = process_inner.work_list.len();
            process_inner.need_list.push(vec![0; len]);
            process_inner.alloc_list.push(vec![0; len]);
        } else {
            // process_inner.need_list[0].push(0);
            for need_list in process_inner.need_list.iter_mut() {
                need_list.push(0);
            }
            for alloc_list in process_inner.alloc_list.iter_mut() {
                alloc_list.push(0);
            }
        }
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    // let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    if process_inner.deadlock_detect_enable {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        process_inner.need_list[tid][mutex_id] += 1;
        let mut finish = alloc::vec![false; process_inner.tasks.len()];
        let mut work = alloc::vec![0; process_inner.mutex_list.len()];
        for (i, available) in process_inner.work_list.iter().enumerate() {
            work[i] = *available;
        }

        let mut found = true;
        while found {
            found = false;
            for (tid, need) in process_inner.need_list.iter().enumerate() {
                if finish[tid] {
                    continue;
                }
                let mut ok = true;
                for (i, available) in work.iter().enumerate() {
                    if need[i] == 0 || need[i] <= *available {
                        continue;
                    }
                    ok = false;
                    break;
                }
                if ok {
                    finish[tid] = true;
                    for (i, alloc) in process_inner.alloc_list[tid].iter().enumerate() {
                        work[i] += *alloc;
                    }
                    found = true;
                    break;
                }
            }
        }

        for finished in finish {
            if !finished {
                debug!("lock deadlock detected");
                return -0xDEAD;
            }
        }
        debug!("lock deadlock not detected");
    }
    drop(process_inner);
    drop(process);
    mutex.lock();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.deadlock_detect_enable {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        process_inner.need_list[tid][mutex_id] -= 1;
        process_inner.alloc_list[tid][mutex_id] += 1;
        process_inner.work_list[mutex_id] -= 1;
    }
    drop(process_inner);
    drop(process);
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.deadlock_detect_enable {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        process_inner.alloc_list[tid][mutex_id] -= 1;
        process_inner.work_list[mutex_id] += 1;
    }
    drop(process_inner);
    drop(process);
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        process_inner.work_list[id] = res_count;
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.work_list.push(res_count);
        if process_inner.need_list.is_empty() {
            let len = process_inner.work_list.len();
            process_inner.need_list.push(vec![0; len]);
            process_inner.alloc_list.push(vec![0; len]);
        } else {
            // process_inner.need_list[0].push(0);
            let deadlock_detect_enable = process_inner.deadlock_detect_enable;
            for need_list in process_inner.need_list.iter_mut() {
                need_list.push(0);
                if deadlock_detect_enable {
                    // debug!("semaphore create need list len: {}", need_list.len());
                }
            }
            for alloc_list in process_inner.alloc_list.iter_mut() {
                alloc_list.push(0);
                if deadlock_detect_enable {
                    // debug!("semaphore create alloc list len: {}", alloc_list.len());
                }
            }
        }
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    // let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    if process_inner.deadlock_detect_enable {
        debug!("sem_up [sem_id: {}]", sem_id);
        process_inner.alloc_list[tid][sem_id] -= 1;
        process_inner.work_list[sem_id] += 1;
    }
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    // let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    if process_inner.deadlock_detect_enable {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        process_inner.need_list[tid][sem_id] += 1;
        let mut finish = alloc::vec![false; process_inner.tasks.len()];
        let mut work = alloc::vec![0; process_inner.semaphore_list.len()];
        for (i, available) in process_inner.work_list.iter().enumerate() {
            work[i] = *available;
        }

        let mut found = true;
        while found {
            found = false;
            for (tid, need) in process_inner.need_list.iter().enumerate() {
                if finish[tid] {
                    continue;
                }
                let mut ok = true;
                for (i, available) in work.iter().enumerate() {
                    if need[i] == 0 || need[i] <= *available {
                        continue;
                    }
                    ok = false;
                    break;
                }
                if ok {
                    finish[tid] = true;
                    for (i, alloc) in process_inner.alloc_list[tid].iter().enumerate() {
                        work[i] += *alloc;
                    }
                    found = true;
                    break;
                }
            }
        }

        for finished in finish {
            if !finished {
                debug!("semaphore deadlock detected: [sem_id: {}]", sem_id);
                return -0xDEAD;
            }
        }
        debug!("semaphore deadlock not detected: [sem_id: {}]", sem_id);
    }
    drop(process_inner);
    sem.down();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.deadlock_detect_enable {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        debug!("semaphore lock get: [sem_id: {}]", sem_id);
        process_inner.need_list[tid][sem_id] -= 1;
        process_inner.alloc_list[tid][sem_id] += 1;
        process_inner.work_list[sem_id] -= 1;
    }
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    if _enabled == 0 {
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detect_enable = false;
        0
    } else if _enabled == 1{
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detect_enable = true;
        debug!("deadlock detect enable");
        0
    } else {
        -1
    }
}
