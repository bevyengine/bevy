use concurrent_queue::ConcurrentQueue;
use std::sync::Arc;

pub(crate) struct EventQueue<T>(Arc<ConcurrentQueue<T>>);

impl<T> EventQueue<T> {
    pub fn new() -> Self {
        Self(Arc::new(ConcurrentQueue::unbounded()))
    }

    pub fn sender(&self) -> EventSender<T> {
        EventSender(self.0.clone())
    }

    pub fn receiver(&self) -> EventReceiver<T> {
        EventReceiver(self.0.clone())
    }

    pub fn send(&self, value: T) {
        self.0
            .push(value)
            .unwrap_or_else(|_| panic!("Failed to send value."));
    }

    pub fn try_iter(&self) -> concurrent_queue::TryIter<T> {
        self.0.try_iter()
    }
}

impl<T> Clone for EventQueue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// A strictly non-blocking sender for a multi-producer multi-consumer channel.
#[derive(Debug)]
pub struct EventSender<T>(Arc<ConcurrentQueue<T>>);

impl<T: Send> EventSender<T> {
    pub fn send(&self, value: T) {
        self.0
            .push(value)
            .unwrap_or_else(|_| panic!("Failed to send value."));
    }
}

impl<T> Clone for EventSender<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// A strictly non-blocking reciever for a multi-producer multi-consumer channel.
#[derive(Debug)]
pub struct EventReceiver<T>(Arc<ConcurrentQueue<T>>);

impl<T: Send> EventReceiver<T> {
    pub fn try_recv(&self) -> Result<T, concurrent_queue::PopError> {
        self.0.pop()
    }

    pub fn try_iter(&self) -> concurrent_queue::TryIter<T> {
        self.0.try_iter()
    }
}

impl<T> Clone for EventReceiver<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
