use crate::bitflags::bitflags;
use crate::{error, debug};
use crate::task::{current_user_token, user_map_page, TASK_MANAGER};
use crate::mm::{frame_alloc, MapPermission, MemorySet, PTEFlags, PageTable, VPNRange, VirtAddr};

bitflags! {
    #[derive(Copy, Clone)]
    pub struct UserMapPermission: usize {
        const R = 1 << 0;
        const W = 1 << 1;
        const X = 1 << 2;
    }
}

impl UserMapPermission {
    pub fn is_valid(&self) -> bool {
        (self.bits() & !7 == 0) && self.bits() != 0
    }
}

impl Into<bool> for UserMapPermission {
    fn into(self) -> bool {
        self.bits() != 0
    }
}

impl Into<MapPermission> for UserMapPermission {
    fn into(self) -> MapPermission {
        let mut res = MapPermission::empty();
        if (self & Self::R).into() {
            res |= MapPermission::R;
        }
        if (self & Self::W).into() {
            res |= MapPermission::W;
        }
        if (self & Self::X).into() {
            res |= MapPermission::X;
        }
        res |= MapPermission::U;
        res
    }
}

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    debug!("sys_mmap(start = {:x}, len = {:x}, prot = {})", start, len, prot);
    if let Some(perm) = UserMapPermission::from_bits(prot) {
        if !perm.is_valid() {
            error!("prot is not valid {}", prot);
            return -1;
        }
        let mut table = PageTable::from_token(current_user_token());
        let range = 
            VPNRange::new(VirtAddr::from(start).floor(), VirtAddr::from(start + len).ceil());
        debug!("VPNRange [{:x}, {:x})", range.get_start().0, range.get_end().0);
        let perm : MapPermission = perm.into();
        for vpn in range {
            if let Some(pte) = table.translate(vpn) && pte.is_valid(){
                if pte.is_valid() {
                    error!("cannot map a mapped area {:x} {:x}", vpn.0, pte.ppn().0);
                    return -1;
                }
            }
            user_map_page(vpn, perm);
            if let Some(pte) = table.translate(vpn) && pte.is_valid() {
                debug!("map {:x} -> {:x}", vpn.0, pte.ppn().0);
            }
            else {
                error!("map failed {:x}", vpn.0);
                return -1;
            }
        }
        0
    }
    else {
        error!("prot is not valid {}", prot);
        -1
    }
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    let mut table = PageTable::from_token(current_user_token());
    let range = 
        VPNRange::new(VirtAddr::from(start).floor(), VirtAddr::from(start + len).ceil());
    for vpn in range {
        if let None = table.translate(vpn) {
            error!("cannot unmap an unmapped area {}", vpn.0);
            return -1;
        }
        else {
            table.unmap(vpn);
        }
    }
    0
}