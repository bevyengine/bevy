use crate::Parallel;
use alloc::vec::Vec;
use async_channel::{Receiver, Sender};
use core::ops::{Deref, DerefMut};

/// An asynchronous MPSC channel that buffers messages and reuses allocations with thread locals.
///
/// This is a building block for efficient parallel worker tasks.
///
/// Cache this channel in a system's [`Local`] to reuse allocated memory.
///
/// This is faster than sending each message individually into a channel when communicating between
/// tasks. Unlike `Parallel`, this allows you to execute a consuming task while producing tasks are
/// concurrently sending data into the channel, enabling you to run a serial processing consumer
/// at the same time as many parallel processing producers.
///
/// # Usage
///
/// ```
/// use bevy_utils::BufferedChannel;
/// use bevy_app::{App, TaskPoolPlugin, Update};
/// use bevy_ecs::system::Local;
/// use bevy_tasks::ComputeTaskPool;
///
/// App::new()
///     .add_plugins(TaskPoolPlugin::default())
///     .add_systems(Update, parallel_system)
///     .update();
///
/// fn parallel_system(channel: Local<BufferedChannel<u64>>) {
///     let (rx, tx) = channel.unbounded();
///     ComputeTaskPool::get().scope(|scope| {
///         // Spawn a single consumer task that reads from the producers. Note we can spawn this
///         // first and have it immediately start processing the messages produced in parallel.
///         // Because we are receiving asynchronously, we avoid deadlocks even on a single thread.
///         scope.spawn(async move {
///             let mut total = 0;
///             let mut count = 0;
///             while let Ok(chunk) = rx.recv().await {
///                 count += chunk.len();
///                 total += chunk.iter().sum::<u64>();
///             }
///             assert_eq!(count, 500_000);
///             assert_eq!(total, 24_999_750_000);
///         });
///
///         // Spawn a few producing tasks in parallel that send data into the buffered channel.
///         for _ in 0..5 {
///             let mut tx = tx.clone();
///             scope.spawn(async move {
///                 // Because this is buffered, we can iterate over hundreds of thousands of
///                 // entities in each task while avoiding allocation and channel overhead.
///                 // The buffer is flushed periodically, sending chunks of data to the receiver.
///                 for i in 0..100_000 {
///                     tx.send(i).await;
///                 }
///             });
///         }
///
///         // Drop the unused sender so the channel can close.
///         drop(tx);
///     });
/// }
/// ```
pub struct BufferedChannel<T: Send> {
    /// The minimum length of a `Vec` of buffered data before it is sent through the channel.
    pub chunk_size: usize,
    /// A pool of reusable vectors to minimize allocations.
    pool: Parallel<Vec<Vec<T>>>,
}

impl<T: Send> Default for BufferedChannel<T> {
    fn default() -> Self {
        Self {
            // This was tuned based on benchmarks across a wide range of sizes.
            chunk_size: 1024,
            pool: Parallel::default(),
        }
    }
}

/// A wrapper around a [`Receiver`] that returns [`RecycledVec`]s to automatically return
/// buffers to the [`BufferedChannel`] pool.
pub struct BufferedReceiver<'a, T: Send> {
    channel: &'a BufferedChannel<T>,
    rx: Receiver<Vec<T>>,
}

impl<'a, T: Send> BufferedReceiver<'a, T> {
    /// Receive a message asynchronously.
    ///
    /// The returned [`RecycledVec`] will automatically return the buffer to the pool when dropped.
    pub async fn recv(&self) -> Result<RecycledVec<'_, T>, async_channel::RecvError> {
        let buffer = self.rx.recv().await?;
        Ok(RecycledVec {
            buffer: Some(buffer),
            channel: self.channel,
        })
    }

    /// Receive a message blocking.
    ///
    /// The returned [`RecycledVec`] will automatically return the buffer to the pool when dropped.
    pub fn recv_blocking(&self) -> Result<RecycledVec<'_, T>, async_channel::RecvError> {
        let buffer = self.rx.recv_blocking()?;
        Ok(RecycledVec {
            buffer: Some(buffer),
            channel: self.channel,
        })
    }
}

/// A wrapper around a `Vec<T>` that automatically returns it to the [`BufferedChannel`]'s pool when
/// dropped.
pub struct RecycledVec<'a, T: Send> {
    buffer: Option<Vec<T>>,
    channel: &'a BufferedChannel<T>,
}

impl<'a, T: Send> Deref for RecycledVec<'a, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().unwrap()
    }
}

impl<'a, T: Send> DerefMut for RecycledVec<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.as_mut().unwrap()
    }
}

impl<'a, T: Send> IntoIterator for RecycledVec<'a, T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        self.buffer.take().unwrap().into_iter()
    }
}

impl<'a, 'b, T: Send> IntoIterator for &'b RecycledVec<'a, T> {
    type Item = &'b T;
    type IntoIter = core::slice::Iter<'b, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.buffer.as_ref().unwrap().iter()
    }
}

impl<'a, 'b, T: Send> IntoIterator for &'b mut RecycledVec<'a, T> {
    type Item = &'b mut T;
    type IntoIter = core::slice::IterMut<'b, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.buffer.as_mut().unwrap().iter_mut()
    }
}

impl<'a, T: Send> Drop for RecycledVec<'a, T> {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.take() {
            buffer.clear();
            self.channel.pool.borrow_local_mut().push(buffer);
        }
    }
}

/// A [`BufferedChannel`] sender that buffers messages locally, flushing it when the sender is
/// dropped or [`BufferedChannel::chunk_size`] is reached.
pub struct BufferedSender<'a, T: Send> {
    channel: &'a BufferedChannel<T>,
    /// We use an `Option` to lazily allocate the buffer or pull from the channel's buffer pool.
    buffer: Option<Vec<T>>,
    tx: Sender<Vec<T>>,
}

impl<T: Send> BufferedChannel<T> {
    fn get_buffer(&self) -> Vec<T> {
        self.pool
            .borrow_local_mut()
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.chunk_size))
    }

    /// Create an unbounded channel and return the receiver and sender.
    ///
    /// The created channel can hold an unlimited number of messages.
    pub fn unbounded(&self) -> (BufferedReceiver<'_, T>, BufferedSender<'_, T>) {
        let (tx, rx) = async_channel::unbounded();
        (
            BufferedReceiver { channel: self, rx },
            BufferedSender {
                channel: self,
                buffer: None,
                tx,
            },
        )
    }

    /// Create a bounded channel and return the receiver and sender.
    ///
    /// The created channel has space to hold at most `cap` messages at a time.
    ///
    /// # Panics
    ///
    /// Capacity must be a positive number. If `cap` is zero, this function will panic.
    pub fn bounded(&self, cap: usize) -> (BufferedReceiver<'_, T>, BufferedSender<'_, T>) {
        let (tx, rx) = async_channel::bounded(cap);
        (
            BufferedReceiver { channel: self, rx },
            BufferedSender {
                channel: self,
                buffer: None,
                tx,
            },
        )
    }
}

impl<'a, T: Send> BufferedSender<'a, T> {
    /// Send a message asynchronously.
    ///
    /// This is buffered and will not be sent into the channel until [`BufferedChannel::chunk_size`]
    /// messages are accumulated or the sender is dropped.
    pub async fn send(&mut self, msg: T) -> Result<(), async_channel::SendError<Vec<T>>> {
        let buffer = self.buffer.get_or_insert_with(|| self.channel.get_buffer());
        buffer.push(msg);
        if buffer.len() >= self.channel.chunk_size {
            let full_buffer = self.buffer.take().unwrap();
            self.tx.send(full_buffer).await?;
        }
        Ok(())
    }

    /// Send an item blocking.
    ///
    /// This is buffered and will not be sent into the channel until [`BufferedChannel::chunk_size`]
    /// messages are accumulated or the sender is dropped.
    pub fn send_blocking(&mut self, msg: T) -> Result<(), async_channel::SendError<Vec<T>>> {
        let buffer = self.buffer.get_or_insert_with(|| self.channel.get_buffer());
        buffer.push(msg);
        if buffer.len() >= self.channel.chunk_size {
            let full_buffer = self.buffer.take().unwrap();
            self.tx.send_blocking(full_buffer)?;
        }
        Ok(())
    }

    /// Flush any remaining messages in the local buffer, sending them into the channel.
    fn flush(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            if !buffer.is_empty() {
                // The allocation is sent through the channel and will be reused when dropped.
                let _ = self.tx.send_blocking(buffer);
            } else {
                // If it's empty, just return it to the pool.
                self.channel.pool.borrow_local_mut().push(buffer);
            }
        }
    }
}

impl<'a, T: Send> Clone for BufferedSender<'a, T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel,
            buffer: None,
            tx: self.tx.clone(),
        }
    }
}

/// Automatically flush the buffer when a sender is dropped.
impl<'a, T: Send> Drop for BufferedSender<'a, T> {
    fn drop(&mut self) {
        self.flush();
    }
}
