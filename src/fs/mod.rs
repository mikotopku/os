//! File system in os
mod inode;
mod stdio;
mod pipe;

use crate::mm::UserBuffer;

#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}

impl Stat {
    pub fn empty() -> Self {
        Self {
            dev: 0,
            ino: 0,
            mode: StatMode::NULL,
            nlink: 0, 
            pad: [0; 7],
        }
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

/// File trait
pub trait File: Send + Sync {
    /// If readable
    fn readable(&self) -> bool;
    /// If writable
    fn writable(&self) -> bool;
    /// Read file to `UserBuffer`
    fn read(&self, buf: UserBuffer) -> usize;
    /// Write `UserBuffer` to file
    fn write(&self, buf: UserBuffer) -> usize;
    fn stat(&self) -> Stat;
}

pub use inode::{OSInode, OpenFlags, list_apps, open_file, create_hard_link, delete_hard_link, hard_link_cnt};
pub use stdio::{Stdin, Stdout};
pub use pipe::make_pipe;
