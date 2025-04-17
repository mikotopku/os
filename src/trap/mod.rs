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

use crate::syscall::syscall;
use core::{arch::global_asm, usize};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Trap},
    stval, stvec::{self, Stvec},
};
use riscv::interrupt::Exception;
use crate::println;
use crate::task::exit_current_and_run_next;

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        let mut v = Stvec::from_bits(0);
        v.set_address(__alltraps as usize);
        v.set_trap_mode(TrapMode::Direct);
        stvec::write(v);
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(exnum) => {
            match unsafe {core::mem::transmute(exnum)}{
                Exception::UserEnvCall => {
                    cx.sepc += 4;
                    cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
                }
                Exception::StoreFault | Exception::StorePageFault => {
                    println!("[kernel] PageFault in application, kernel killed it.");
                    exit_current_and_run_next();
                }
                Exception::IllegalInstruction => {
                    println!("[kernel] IllegalInstruction in application, kernel killed it.");
                    exit_current_and_run_next();
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
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::TrapContext;
