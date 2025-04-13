#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::global_asm;

mod lang_items;
mod sbi;

#[macro_use]
mod console;

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("hello, world");
    error!("hello, world\n");
    warn!("hello, world\n");
    info!("hello, world\n");
    debug!("hello, world\n");
    trace!("hello, world\n");
    panic!("shutdown machine");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}


global_asm!(include_str!("entry.asm"));