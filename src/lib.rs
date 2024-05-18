#![allow(unused)]
#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

/// 实现一个简单的channel: 用于多线程之间传递信息(send messages)
/// 最简单的idea:
///     1. 用VecDeque作底层数据结构存储信息,
///        a.当tx写时, 把message入队.
///        b.当rx读时, 把message出队.
///     2. 用Mutex<>包裹VecDeque以实现互斥(粗粒度的全局锁).
///     3. 用CondVar实现阻塞性:
///        cond1. 当tx写时, 在这里VecDequeue可能会自己扩容,所以不需要担心没位置(full).
///        cond2. 当rx读时, 如果channel为空也给老子等着(用Convar), 等着tx写然后读了再返回.
///

// Very Naive unbounded MPMS
pub struct Channel<T> {
    queue: Mutex<VecDeque<T>>,
    ready: Condvar,
}

impl<T> Channel<T> {
    /// constrcutor
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }

    /// When tx sends a message, push_back the message into the Vecqueue.
    pub fn send(&self, message: T) {
        // Get the "temp" Owernship of the queue by `lock()`.
        self.queue.lock().unwrap().push_back(message);
        // Wakes up one blocked thread on this condvar.
        self.ready.notify_one();
    }

    /// When rx wanna receive a msg, just pop the front
    /// If there's no data remaining, then use Convar.wait() to block.
    pub fn receive(&self) -> T {
        let mut b = self.queue.lock().unwrap();
        loop {
            if let Some(msg) = b.pop_front() {
                return msg;
            }
            // attention: Convard::wait() will unlock Mutex() when waiting
            // and will re-lock the Mutex before returning(when the cond is arrived)
            b = self.ready.wait(b).unwrap();
        }
    }
}
