use core::iter::Empty;

pub const MAIL_BUFFER_SIZE: usize = 16;
pub const MAIL_MAXLEN: usize = 256;

#[derive(Clone, Copy)]
pub struct Mail{
    pub content: [u8; MAIL_MAXLEN],
    pub len: usize,
}

impl Mail {
    pub fn empty() -> Self {
        Self { content: [0; MAIL_MAXLEN], len: 0 }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

#[derive(Clone)]
pub struct MailRingBuffer {
    mails: [Mail; MAIL_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
}

impl MailRingBuffer {
    pub fn new() -> Self {
        Self {
            mails: core::array::from_fn(|_| Mail::empty()),
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
        }
    }
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + MAIL_BUFFER_SIZE - self.head
        }
    }
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            MAIL_BUFFER_SIZE - self.available_read()
        }
    }
    pub fn read(&mut self) -> Mail {
        self.status = RingBufferStatus::Normal;
        let m = self.mails[self.head];
        self.head = (self.head + 1) % MAIL_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        m
    }
    pub fn write(&mut self, mail: &Mail){
        self.status = RingBufferStatus::Normal;
        self.mails[self.tail] = *mail;
        self.tail = (self.tail + 1) % MAIL_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }
}

pub type MailBox = MailRingBuffer;