//! Types related to task management

use super::TaskContext;
use crate::syscall::MAX_SYSCALL_NUM;

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Clone, Copy)]
pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub total_time: usize,
    pub last_start: usize,
}

#[derive(Clone, Copy)]
pub struct SyscallInfo {
    pub id: usize,
    pub times: usize
}

impl TaskInfo {
    pub fn init(id: usize) -> Self {
        Self {
            id: id,
            status: TaskStatus::UnInit,
            call: [SyscallInfo {id: 0, times: 0}; MAX_SYSCALL_NUM],
            total_time: 0,
            last_start: 0,
        }
    }
    pub fn user(&self) -> UserTaskInfo {
        UserTaskInfo { id: self.id, status: self.status, call: self.call, total_time: self.total_time }
    }
}

pub struct UserTaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub total_time: usize,
}