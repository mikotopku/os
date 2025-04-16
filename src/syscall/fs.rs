//! File and filesystem-related syscalls

const FD_STDOUT: usize = 1;
use crate::batch::{get_app_info, usrstk_info, APP_BASE_ADDRESS};
use crate::{print, debug};

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let addr = buf as usize;
    let info = get_app_info();
    if !((addr >= APP_BASE_ADDRESS) || 
         (addr >= usrstk_info().0 && addr < usrstk_info().1)) {
        debug!("{:x} {:x} {:x} {:x}\n", APP_BASE_ADDRESS, APP_BASE_ADDRESS + info.2 - info.1, usrstk_info().0, usrstk_info().1);
        panic!("syswrite: invalid buf address 0x{:x}", addr);
    }
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let s = core::str::from_utf8(slice).unwrap();
            print!("{}", s);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
