use crate::{fs::{File, Stat}, sync::{Mutex, MutexBlocking, Semaphore, UPSafeCell}, task::current_task};
use crate::debug;

pub struct MutexFD {
    content: UPSafeCell<usize>,
    block: bool,
    mutex: Option<MutexBlocking>,
    fd: usize,
}

impl MutexFD {
    pub fn new(initval: usize, block: bool, fd: usize) -> Self {
        if block {
            let a = Self {
                content: unsafe {UPSafeCell::new(initval)},
                block,
                mutex: Some(MutexBlocking::new()),
                fd,
            };
            if initval == 0 {
                a.mutex.as_ref().unwrap().lock();
            }
            a
        } else {
            Self {
                content: unsafe {UPSafeCell::new(initval)},
                block,
                mutex: None,
                fd,
            }
        }
    }
}

impl File for MutexFD {
    fn read(&self, buf: crate::mm::UserBuffer) -> usize {
        if self.block {
            debug!("{} lock", self.fd);
            self.mutex.as_ref().unwrap().lock();
        }
        else if *self.content.exclusive_access() == 0 {
            return -2isize as usize;
        }
        let bytes = self.content.exclusive_access().to_ne_bytes();
        let mut already_read = 0;
        for seg in buf.buffers {
            let to_read = seg.len().min(bytes.len() - already_read);
            seg[0..to_read].copy_from_slice(&bytes[already_read..already_read + to_read]);
            already_read += to_read;
            if already_read == bytes.len() { break; }
        }
        *self.content.exclusive_access() = 0;
        already_read
    }
    fn write(&self, buf: crate::mm::UserBuffer) -> usize {
        let old_content = *self.content.exclusive_access();
        let mut bytes = [0u8; core::mem::size_of::<usize>()];
        let mut already_write = 0;
        for seg in buf.buffers {
            let to_write = seg.len().min(bytes.len() - already_write);
            bytes[already_write..already_write + to_write].copy_from_slice(&seg[0..to_write]);
            already_write += to_write;
            if already_write == bytes.len() { break; }
        }
        let new_content = usize::from_ne_bytes(bytes);
        *self.content.exclusive_access() = new_content;
        if self.block {
            if new_content != 0 && old_content == 0 {
                debug!("{} unlock", self.fd);
                self.mutex.as_ref().unwrap().unlock();
            }
            else if new_content == 0 && old_content != 0 {
                debug!("{} lock", self.fd);
                self.mutex.as_ref().unwrap().lock();
            }
        }
        already_write
    }
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        true
    }
    fn stat(&self) -> crate::fs::Stat {
        Stat::empty()
    }
}

pub struct SemaphoreFD {
    res: UPSafeCell<usize>,
    block: bool,
    sem: Option<Semaphore>,
    fd: usize,
}

impl SemaphoreFD {
    pub fn new(initval: usize, block: bool, fd: usize) -> Self {
        if block {
            Self {
                res: unsafe {UPSafeCell::new(initval)},
                block,
                sem: Some(Semaphore::new(initval)),
                fd,
            }
        } else {
            Self {
                res: unsafe {UPSafeCell::new(initval)},
                block,
                sem: None,
                fd,
            }
        }
    }
}

impl File for SemaphoreFD {
    fn read(&self, _buf: crate::mm::UserBuffer) -> usize {
        if self.block {
            debug!("{} {} try sem down", current_task().as_ref().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid, self.fd);
            self.sem.as_ref().unwrap().down();
            debug!("{} {} sem down", current_task().as_ref().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid, self.fd);
            1
        } else {
            if *self.res.exclusive_access() as isize <= 0 {
                -2isize as usize
            } else {
                *self.res.exclusive_access() -= 1;
                1
            }
        }
    }
    fn write(&self, _buf: crate::mm::UserBuffer) -> usize {
        if self.block {
            debug!("{} {} sem up", current_task().as_ref().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid, self.fd);
            self.sem.as_ref().unwrap().up();
        } else {
            *self.res.exclusive_access() += 1;
        }
        1
    }
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        true
    }
    fn stat(&self) -> Stat {
        Stat::empty()
    }
}