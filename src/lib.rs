#![allow(unused)]
#![allow(dead_code)]

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::{Condvar, Mutex};

/// 实现一个One-Shot channel
/// One-shot: 从一个线程向另一个线程准确地发送一条消息
/// 使用到的工具:
///     1.UnsafeCell 用于存储message，
///     2.AtomicBool 用于指示其状态(消息是否可以被消费).

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    /// constrcutor
    pub fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    /// Satefy: user, you should ensures `Once` Semantics
    pub unsafe fn send(&self, message: T) {
        (*self.message.get()).write(message);
        self.ready.store(true, std::sync::atomic::Ordering::Release);
    }

    /// check whether the message is available
    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub unsafe fn receive(&self) -> T {
        if !self.ready.load(std::sync::atomic::Ordering::Acquire) {
            panic!("no message available!");
        }
        (*self.message.get()).assume_init_read()
    }
}
