use crate::bitflags::bitflags;
use crate::{error, debug};
use crate::task::{current_user_token, user_insert_area, user_unmap_area};
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
        }
        user_insert_area(range.get_start().into(), range.get_end().into(), perm);
        0
    }
    else {
        error!("prot is not valid {}", prot);
        -1
    }
}

pub fn sys_munmap(start: usize) -> isize {
    user_unmap_area(VirtAddr::from(start))
}