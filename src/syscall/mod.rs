//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_TASKINFO: usize = 410;
pub const MAX_SYSCALL_NUM: usize = 5;

mod fs;
mod process;

use fs::*;
use process::*;
use crate::task::{UserTaskInfo, TASK_MANAGER};

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let cur = inner.current();
    match syscall_id {
        SYSCALL_EXIT | SYSCALL_GET_TIME | SYSCALL_TASKINFO | SYSCALL_WRITE | SYSCALL_YIELD => (),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
    for i in 0..MAX_SYSCALL_NUM {
        if inner.taskinfo[cur].call[i].id == syscall_id {
            inner.taskinfo[cur].call[i].times += 1;
            break;
        }
        else if inner.taskinfo[cur].call[i].id == 0 {
            inner.taskinfo[cur].call[i].id = syscall_id;
            inner.taskinfo[cur].call[i].times = 1;
            break;
        }
    }
    drop(inner);
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_TASKINFO => sys_task_info(args[0], args[1] as *mut UserTaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
