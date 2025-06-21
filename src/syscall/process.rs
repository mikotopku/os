//! Process management syscalls
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::config::PAGE_SIZE;
use crate::fs::{open_file, OpenFlags};
use crate::task::{add_task, current_task, current_user_token, exit_current_and_run_next, pid2task, suspend_current_and_run_next, Mail, SignalAction, SignalFlags, UserTaskInfo, MAIL_MAXLEN, MAX_SIG};
use crate::timer::get_time_ms;
use crate::{println, debug};
use crate::mm::{translated_args_vec, translated_byte_buffer, translated_ref, translated_refmut, translated_str};
use core::mem::size_of;

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
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

// pub fn sys_task_info(id: usize, ts: *mut UserTaskInfo) -> isize {
//     if id >= TASK_MANAGER.num_app() { -1 }
//     else {
//         let inner = TASK_MANAGER.inner.exclusive_access();
//         let sz = size_of::<UserTaskInfo>();
//         let src = inner.taskinfo[id].user();
//         drop(inner);
//         unsafe {
//             let buf = translated_byte_buffer(current_user_token(), 
//                 ts as usize as *const u8, sz);
//             let mut srcptr = &src as *const UserTaskInfo as usize;
//             for seg in buf {
//                 core::ptr::copy(srcptr as *const u8, seg.as_mut_ptr(), seg.len());
//                 srcptr += seg.len();
//             }
//         }
//         0
//     }
// }

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8, args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let args_vec: Vec<String> = translated_args_vec(token, args);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(), args_vec);
        // return argc because cx.x[10] will be covered with it later
        argc as isize
    } else {
        -1
    }
}

pub fn sys_spawn(path: *const u8, args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let args_vec = translated_args_vec(token, args);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let data = &app_inode.read_all();
        let current_task = current_task().unwrap();
        let ntask = current_task.spawn(data, args_vec);
        let pid = ntask.getpid();
        add_task(ntask);
        pid as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    // find a child process

    // ---- access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB lock exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child TCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB lock automatically
}

pub fn sys_set_priority(prio: u8) -> isize {
    if prio < 2 {
        -1
    } else {
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        inner.priority = prio;
        0
    }
}

pub fn sys_kill(pid: usize, signum: i32) -> isize {
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signum) {
            // insert the signal if legal
            let mut task_ref = task.inner_exclusive_access();
            if task_ref.signals.contains(flag) {
                return -1;
            }
            task_ref.signals.insert(flag);
            0
        } else {
            debug!("sys_kill: from_bit failed");
            -1
        }
    } else {
        debug!("sys_kill: pid2task failed");
        -2
    }
}

pub fn sys_sigprocmask(mask: u32) -> isize {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        let old_mask = inner.signal_mask;
        if let Some(flag) = SignalFlags::from_bits(mask) {
            inner.signal_mask = flag;
            old_mask.bits() as isize
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigreturn() -> isize {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        inner.handling_sig = -1;
        // restore the trap context
        let trap_ctx = inner.get_trap_cx();
        *trap_ctx = inner.trap_ctx_backup.unwrap();
        // Here we return the value of a0 in the trap_ctx,
        // otherwise it will be overwritten after we trap
        // back to the original execution of the application.
        trap_ctx.x[10] as isize
    } else {
        -1
    }
}

fn check_sigaction_error(signal: SignalFlags, action: usize, old_action: usize) -> bool {
    if action == 0
        || old_action == 0
        || signal == SignalFlags::SIGKILL
        || signal == SignalFlags::SIGSTOP
    {
        true
    } else {
        false
    }
}

pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if signum as usize > MAX_SIG {
        return -1;
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag, action as usize, old_action as usize) {
            return -1;
        }
        let prev_action = inner.signal_actions.table[signum as usize];
        *translated_refmut(token, old_action) = prev_action;
        inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
        0
    } else {
        -1
    }
}

pub fn sys_mailread(buf: *mut u8, len: usize) -> isize {
    let task = current_task().unwrap();
    debug!("{} readable: {}", task.getpid(), task.mailread_available());
    if len == 0 {
        if task.mailread_available() > 0 { return 0; }
        else { return -1; }
    }
    let mail = task.mailread();
    drop(task);
    if let Some(mail) = mail {
        let token = current_user_token();
        let len = len.min(MAIL_MAXLEN);
        let tr = translated_byte_buffer(token, buf, len);
        let mut already_read = 0;
        for b in tr {
            let toread = mail.len.min(already_read + b.len()) - already_read;
            b[..toread].copy_from_slice(&mail.content[already_read..already_read + toread]);
            already_read += toread;
            if already_read == mail.len { break; }
        }
        already_read as isize
    } else {
        -1
    }
}

pub fn sys_mailwrite(pid: usize, buf: *const u8, len: usize) -> isize {
    let task = pid2task(pid);
    if let Some(task) = task
    {
        debug!("{} writable: {}", pid, task.mailwrite_available());
        if task.mailwrite_available() > 0 {
            if len == 0 { return 0; }
        } else { return -1; }
        let mut mail = Mail::empty();
        let token = current_user_token();
        let len = len.min(MAIL_MAXLEN);
        let tr = translated_byte_buffer(token, buf, len);
        let mut already_write = 0;
        for b in tr {
            let towrite = b.len().min(len - already_write);
            mail.content[already_write..already_write + towrite].copy_from_slice(b);
            already_write += towrite;
        }
        mail.len = already_write;
        task.mailwrite(&mail)
    } else {
        -1
    }
}
