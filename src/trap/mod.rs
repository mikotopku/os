//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].
mod context;

use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::{println, debug};
use crate::syscall::syscall;
use crate::task::{
    check_signals_error_of_current, current_add_signal, current_trap_cx, current_user_token, exit_current_and_run_next, handle_signals, suspend_current_and_run_next, SignalFlags
};
use crate::timer::set_next_trigger;
use core::arch::{asm, global_asm};
use riscv::register::stvec::Stvec;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Trap},
    sie, stval, stvec,
};
use riscv::interrupt::{Exception, Interrupt};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        let mut v = Stvec::from_bits(0);
        v.set_address(trap_from_kernel as usize);
        v.set_trap_mode(TrapMode::Direct);
        stvec::write(v);
    }
}

fn set_user_trap_entry() {
    unsafe {
        let mut v = Stvec::from_bits(0);
        v.set_address(TRAMPOLINE);
        v.set_trap_mode(TrapMode::Direct);
        stvec::write(v);
    }
}

/// enable timer interrupt in sie CSR
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(exnum) => {
            match unsafe {core::mem::transmute(exnum)}{
                Exception::UserEnvCall => {
                    let mut cx = current_trap_cx();
                    cx.sepc += 4;
                    let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15], cx.x[16]]) as usize;
                    cx = current_trap_cx();
                    cx.x[10] = result;
                }
                Exception::StoreFault
                | Exception::StorePageFault 
                | Exception::LoadFault
                | Exception::LoadPageFault
                | Exception::InstructionFault
                | Exception::InstructionPageFault
                => {
                    current_add_signal(SignalFlags::SIGSEGV);
                }
                Exception::IllegalInstruction => {
                    current_add_signal(SignalFlags::SIGILL);
                }
                _ => {
                    panic!(
                        "Unsupported trap {:?}, stval = {:#x}!",
                        scause.cause(),
                        stval
                    );
                }
            }
        }
        Trap::Interrupt(intnum) => {
            match unsafe {core::mem::transmute(intnum)}{
                Interrupt::SupervisorTimer => {
                    set_next_trigger();
                    suspend_current_and_run_next();
                }
                _ => {
                    panic!(
                        "Unsupported trap {:?}, stval = {:#x}!",
                        scause.cause(),
                        stval
                    );
                }
            }
        }
    }
    // handle signals (handle the sent signal)
    //println!("[K] trap_handler:: handle_signals");
    handle_signals();

    // check error signals (if error then exit)
    if let Some((errno, msg)) = check_signals_error_of_current() {
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }
    trap_return();
}

#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}

#[no_mangle]
/// Unimplement: traps/interrupts/exceptions from kernel mode
/// Todo: Chapter 9: I/O device
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

pub use context::TrapContext;
