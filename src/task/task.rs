//!Implementation of [`TaskControlBlock`]
use super::{PidAllocator, TaskContext};
use super::{pid_alloc, KernelStack, PidHandle};
use crate::config::TRAP_CONTEXT;
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{translated_refmut, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::syscall::MAX_SYSCALL_NUM;
use crate::task::mail::{Mail, MailBox};
use crate::task::{SignalActions, SignalFlags};
use crate::trap::{self, trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::{Vec};
use alloc::vec;
use core::cell::RefMut;
use core::ops::Deref;

pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,
    pub stride: u8,
    pub priority: u8,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub signals: SignalFlags,
    pub signal_mask: SignalFlags,
    // the signal which is being handling
    pub handling_sig: isize,
    // Signal actions
    pub signal_actions: SignalActions,
    // if the task is killed
    pub killed: bool,
    // if the task is frozen by a signal
    pub frozen: bool,
    pub trap_ctx_backup: Option<TrapContext>,
    pub mailbox: MailBox,
}

impl TaskControlBlockInner {
    /*
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
    */
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a task context which goes to trap_return to the top of kernel stack
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    stride: 0,
                    priority: 16,
                    fd_table: vec![
                        Some(Arc::new(Stdin)),
                        Some(Arc::new(Stdout)),
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    signal_mask: SignalFlags::empty(),
                    handling_sig: -1,
                    signal_actions: SignalActions::default(),
                    killed: false,
                    frozen: false,
                    trap_ctx_backup: None,
                    mailbox: MailBox::new(),
                })
            },
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    fn init_args(token: usize, user_sp: usize, args: &Vec<String>) -> (usize, usize) {
        // push arguments on user stack
        let mut user_sp = user_sp;
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(token, p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();
        (user_sp, argv_base)
    }
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, mut user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // push arguments on user stack
        let argv_base;
        (user_sp, argv_base) = TaskControlBlock::init_args(memory_set.token(), user_sp, &args);

        // **** access current TCB exclusively
        let mut inner = self.inner_exclusive_access();
        // substitute memory_set
        inner.memory_set = memory_set;
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;
        // initialize trap_cx
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *inner.get_trap_cx() = trap_cx;
        // **** release current PCB
    }
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();
        // copy user space(include trap context)
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    stride: 0,
                    priority: 16,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    // inherit the signal_mask and signal_action
                    signal_mask: parent_inner.signal_mask,
                    handling_sig: -1,
                    signal_actions: parent_inner.signal_actions.clone(),
                    killed: false,
                    frozen: false,
                    trap_ctx_backup: None,
                    mailbox: parent_inner.mailbox.clone(),
                })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        // return
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }
    pub fn spawn(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>) -> Arc<Self>{
        let (memory_set, mut user_sp, entry) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = 
            memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let argv_base;
        (user_sp, argv_base) = TaskControlBlock::init_args(memory_set.token(), user_sp, &args);
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        let tcb = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe { UPSafeCell::new(TaskControlBlockInner {
                trap_cx_ppn,
                base_size: user_sp,
                task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                task_status: TaskStatus::Ready,
                memory_set,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
                stride: 0,
                priority: 16,
                fd_table: vec![
                    Some(Arc::new(Stdin)),
                    Some(Arc::new(Stdout)),
                    Some(Arc::new(Stdout)),
                ],
                signals: SignalFlags::empty(),
                signal_mask: SignalFlags::empty(),
                handling_sig: -1,
                signal_actions: SignalActions::default(),
                killed: false,
                frozen: false,
                trap_ctx_backup: None,
                mailbox: MailBox::new(),
            })}
        });
        let trap_cx = tcb.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        let mut parent_inner = self.inner_exclusive_access();
        parent_inner.children.push(tcb.clone());
        tcb
    }
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
    pub fn mailread(&self) -> Option<Mail> {
        let mut inner = self.inner_exclusive_access();
        if inner.mailbox.available_read() > 0 {
            Some(inner.mailbox.read())
        }else {
            None
        }
    }
    pub fn mailwrite(&self, mail: &Mail) -> isize {
        let mut inner = self.inner_exclusive_access();
        if inner.mailbox.available_write() > 0 {
            inner.mailbox.write(mail);
            mail.len as isize
        } else {
            -1
        }
    }
    pub fn mailread_available(&self) -> usize {
        self.inner_exclusive_access().mailbox.available_read()
    }
    pub fn mailwrite_available(&self) -> usize {
        self.inner_exclusive_access().mailbox.available_write()
    }
}

fn tcb_cmp(this: &TaskControlBlock, other: &TaskControlBlock) -> core::cmp::Ordering {
    let this = this.inner_exclusive_access();
    let another = other.inner_exclusive_access();
    let delta = this.stride - another.stride;
    if delta == 0 {
        core::cmp::Ordering::Equal
    }
    else if delta <= 128 {
        core::cmp::Ordering::Less
    }
    else {
        core::cmp::Ordering::Greater
    }
}

impl PartialEq for TaskControlBlock {
    fn eq(&self, other: &Self) -> bool {
        let this = self.inner_exclusive_access();
        let other = other.inner_exclusive_access();
        this.stride == other.stride
    }
}

impl Eq for TaskControlBlock {}

impl PartialOrd for TaskControlBlock {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(tcb_cmp(self, other))
    }
}

impl Ord for TaskControlBlock {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        tcb_cmp(self, other)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}


pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: Vec<SyscallInfo>,
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
            status: TaskStatus::Ready,
            call: Vec::new(),
            total_time: 0,
            last_start: 0,
        }
    }
    pub fn user(&self) -> UserTaskInfo {
        let mut ret = UserTaskInfo {
            id: self.id,
            status: self.status,
            call: [SyscallInfo{id: 0, times: 0}; MAX_SYSCALL_NUM],
            total_time: self.total_time
        };
        for (ind, info) in self.call.iter().enumerate() {
            ret.call[ind] = *info;
        }
        ret
    }
}

pub struct UserTaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub total_time: usize,
}