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
    // in_use: 表示通道是否已经被占用(有thread在写)
    in_use: AtomicBool,
    // ready : 表示通道里是否有可用的元素.
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

impl<T> Channel<T> {
    /// constrcutor
    pub fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            in_use: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    /// use `in_use` to ensures that Only sned One message.
    pub fn send(&self, message: T) {
        if self.in_use.swap(true, std::sync::atomic::Ordering::Relaxed) {
            panic!("can't send more than One msg!");
        }
        unsafe { (*self.message.get()).write(message) };
        self.ready.store(true, std::sync::atomic::Ordering::Release);
    }

    /// check whether the message is available
    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn receive(&self) -> T {
        if !self.ready.load(std::sync::atomic::Ordering::Acquire) {
            panic!("no message available!");
        }
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn it_works() {
        let channel = Channel::new();
        let t = thread::current();
        thread::scope(|s| {
            s.spawn(|| {
                channel.send("hello world!");
                t.unpark();
            });
            while !channel.is_ready() {
                thread::park();
            }
            assert_eq!(channel.receive(), "hello world!");
        })
    }
}
