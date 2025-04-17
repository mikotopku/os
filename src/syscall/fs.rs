//! File and filesystem-related syscalls

const FD_STDOUT: usize = 1;
use crate::{print, debug};

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
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
