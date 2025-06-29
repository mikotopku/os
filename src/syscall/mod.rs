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

const SYSCALL_DUP: usize = 24;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_TASKINFO: usize = 410;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_SIGACTION: usize = 134;
const SYSCALL_SIGPROCMASK: usize = 135;
const SYSCALL_SIGRETURN: usize = 139;
const SYSCALL_KILL: usize = 129;
const SYSCALL_MAILREAD: usize = 401;
const SYSCALL_MAILWRITE: usize = 402;
pub const MAX_SYSCALL_NUM: usize = 27;

mod fs;
mod process;
mod mem;

use fs::*;
use process::*;
use crate::{fs::Stat, task::{SignalAction, UserTaskInfo}};
use mem::*;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 7]) -> isize {
    // let mut inner = TASK_MANAGER.inner.exclusive_access();
    // let cur = inner.current();
    // match syscall_id {
    //     SYSCALL_EXIT | SYSCALL_GET_TIME | SYSCALL_TASKINFO | SYSCALL_READ |
    //     SYSCALL_WRITE | SYSCALL_YIELD | SYSCALL_MMAP | SYSCALL_MUNMAP | 
    //     SYSCALL_GETPID | SYSCALL_FORK | SYSCALL_EXEC | SYSCALL_WAITPID => (),
    //     _ => panic!("Unsupported syscall_id: {}", syscall_id),
    // }
    // for i in 0..MAX_SYSCALL_NUM {
    //     if inner.taskinfo[cur].call[i].id == syscall_id {
    //         inner.taskinfo[cur].call[i].times += 1;
    //         break;
    //     }
    //     else if inner.taskinfo[cur].call[i].id == 0 {
    //         inner.taskinfo[cur].call[i].id = syscall_id;
    //         inner.taskinfo[cur].call[i].times = 1;
    //         break;
    //     }
    // }
    // drop(inner);
    match syscall_id {
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut usize),
        SYSCALL_OPEN => sys_open(args[0] as *const u8, args[1] as u32),
        SYSCALL_CLOSE => sys_close(args[0] as usize),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        // SYSCALL_TASKINFO => sys_task_info(args[0], args[1] as *mut UserTaskInfo),
        SYSCALL_MMAP => sys_mmap(args[0], args[1] , args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0]),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as u8),
        SYSCALL_LINKAT => sys_linkat(args[0] as i32, args[1] as *const u8, args[2] as i32, args[3] as *const u8, args[4] as u32),
        SYSCALL_UNLINKAT => sys_unlinkat(args[0] as i32, args[1] as *const u8, args[2] as u32),
        SYSCALL_FSTAT => sys_fstat(args[0] as i32, args[1] as *mut Stat),
        SYSCALL_KILL => sys_kill(args[0], args[1] as i32),
        SYSCALL_SIGACTION => sys_sigaction(
            args[0] as i32,
            args[1] as *const SignalAction,
            args[2] as *mut SignalAction,
        ),
        SYSCALL_SIGPROCMASK => sys_sigprocmask(args[0] as u32),
        SYSCALL_SIGRETURN => sys_sigreturn(),
        SYSCALL_MAILREAD => sys_mailread(args[0] as *mut u8, args[1]),
        SYSCALL_MAILWRITE => sys_mailwrite(args[0], args[1] as *const u8, args[2]),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
