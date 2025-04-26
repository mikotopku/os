//! Process management syscalls
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next, UserTaskInfo, TASK_MANAGER};
use crate::timer::get_time_ms;
use crate::{println, debug};

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// get time in milliseconds
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_task_info(id: usize, ts: *mut UserTaskInfo) -> isize {
    if id >= TASK_MANAGER.num_app() { -1 }
    else {
        let inner = TASK_MANAGER.inner.exclusive_access();
        unsafe {
            *ts = inner.taskinfo[id].user();
        }
        0
    }
}
