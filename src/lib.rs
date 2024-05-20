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

pub struct Sender<T> {
    inner: Arc<Channel<T>>,
}

pub struct Receiver<T> {
    inner: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    pub fn send(self, msg: T) {
        unsafe { (*self.inner.message.get()).write(msg) };
        self.inner
            .ready
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl<T> Receiver<T> {
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

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    // ready : 表示通道里是否有可用的元素.
    ready: AtomicBool,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(Channel {
        message: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });
    (Sender { inner: a.clone() }, Receiver { inner: a })
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
        let (sender, receiver) = channel();
        let t = thread::current();
        thread::scope(|s| {
            // Sender
            s.spawn(move || {
                sender.send("hello rustacean!");
                t.unpark();
            });
            // Receiver
            while !receiver.is_ready() {
                thread::park()
            }
        });

        // Print Receive message.
        assert_eq!(receiver.recv(), "hello rustacean!");
    }
}
