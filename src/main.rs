#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::global_asm;

mod lang_items;
mod sbi;

#[macro_use]
mod console;

extern "C" {
    fn sbss();
    fn ebss();
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
}

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("hello, world");
    error!("hello, world\n");
    warn!("hello, world\n");
    info!("hello, world\n");
    debug!("hello, world\n");
    trace!("hello, world\n");
    debug!(".text [{:#x} {:#x})\n", stext as usize, etext as usize);
    debug!(".rodata [{:#x} {:#x})\n", srodata as usize, erodata as usize);
    debug!(".data [{:#x} {:#x})\n", sdata as usize, erodata as usize);
    panic!("shutdown machine");
}

fn clear_bss() {
    
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}


global_asm!(include_str!("entry.asm"));