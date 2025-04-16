#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::global_asm;

mod lang_items;
mod sbi;
pub mod batch;
mod sync;
pub mod trap;
pub mod syscall;

#[macro_use]
mod console;



#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] hello, world");
    trap::init();
    batch::init();
    batch::run_next_app();
}

fn clear_bss() {
    unsafe extern "C" {
        fn sbss();
        fn ebss();
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
    }
    
    unsafe {
        core::slice::from_raw_parts_mut(sbss as *mut u8, ebss as usize - sbss as usize).fill(0);
    }
}


global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));