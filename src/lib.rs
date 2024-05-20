#![allow(unused)]
#![allow(dead_code)]

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex};

/// 实现一个One-Shot channel
/// One-shot: 从一个线程向另一个线程准确地发送一条消息
/// 使用到的工具:
///     1.UnsafeCell 用于存储message，
///     2.AtomicBool 用于指示其状态(消息是否可以被消费).

/// 为了防止一个函数被多次调用，我们可以让它按值接受一个参数，对于非 Copy 类型，它会消耗该对象。
/// 一个对象被消耗或移动后，它就从调用者那里消失了，防止它被再次使用。
/// 通过将调用send 或receive 的能力分别表示为单独的(非 Copy)类型，并在执行操作时使用该对象，我们可以确保每个调用只能发生一次。
/// 这将我们带到以下接口设计中，其中通道由一对 Sender 和 Receiver 表示。

pub struct Sender<'a, T> {
    inner: &'a Channel<T>,
}

pub struct Receiver<'a, T> {
    inner: &'a Channel<T>,
}

impl<T> Sender<'_, T> {
    pub fn send(self, msg: T) {
        unsafe { (*self.inner.message.get()).write(msg) };
        self.inner
            .ready
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl<T> Receiver<'_, T> {
    pub fn is_ready(&self) -> bool {
        self.inner.ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn recv(self) -> T {
        if !self
            .inner
            .ready
            .swap(false, std::sync::atomic::Ordering::Acquire)
        {
            panic!("there's no data to read");
        }
        unsafe { (*self.inner.message.get()).assume_init_read() }
    }
}

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    // ready : 表示通道里是否有可用的元素.
    ready: AtomicBool,
}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    /// 此外，我们需要一种方法，让用户创建一个 Sender 和 Receiver对象来借用这个通道。
    /// 这将需要独占借用(&mut Channel)，以确保同一通道不能有多个发送者或接收者。
    /// 通过同时提供 Sender 和 Receiver ，我们可以将独占借用分成两个共享借用，
    /// 这样发送方和接收方都可以引用通道，同时防止其他任何东西接触通道。
    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        *self = Self::new();
        (Sender { inner: self }, Receiver { inner: self })
    }
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    // import
    use super::*;

    // testing
    #[test]
    fn it_works() {
        let mut channel = Channel::new();
        thread::scope(|s| {
            let (sender, receiver) = channel.split();
            let t = thread::current();
            // Sender
            s.spawn(move || {
                sender.send("hello rustacean!");
                t.unpark();
            });
            // Receiver
            while !receiver.is_ready() {
                thread::park()
            }
            // Print Receive message.
            assert_eq!(receiver.recv(), "hello rustacean!");
        });
    }
}
