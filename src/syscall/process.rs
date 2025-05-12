//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next, UserTaskInfo, TASK_MANAGER, current_user_token};
use crate::timer::get_time_ms;
use crate::{println, debug};
use crate::mm::translated_byte_buffer;
use core::mem::size_of;

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
        let sz = size_of::<UserTaskInfo>();
        let src = inner.taskinfo[id].user();
        drop(inner);
        unsafe {
            let buf = translated_byte_buffer(current_user_token(), 
                ts as usize as *const u8, sz);
            let mut srcptr = &src as *const UserTaskInfo as usize;
            for seg in buf {
                core::ptr::copy(srcptr as *const u8, seg.as_mut_ptr(), seg.len());
                srcptr += seg.len();
            }
        }
        0
    }
}
