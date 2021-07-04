use event_listener::Event;
use std::sync::{
    atomic::{AtomicIsize, Ordering},
    Arc,
};

#[derive(Debug)]
struct CountdownEventInner {
    /// Async primitive that can be awaited and signalled. We fire it when counter hits 0.
    event: Event,

    /// The number of decrements remaining
    counter: AtomicIsize,
}

/// A counter that starts with an initial count `n`. Once it is decremented `n` times, it will be
/// "ready". Call `listen` to get a future that can be awaited.
#[derive(Clone, Debug)]
pub struct CountdownEvent {
    inner: Arc<CountdownEventInner>,
}

impl CountdownEvent {
    /// Creates a CountdownEvent that must be decremented `n` times for listeners to be
    /// signalled
    pub fn new(n: isize) -> Self {
        let inner = CountdownEventInner {
            event: Event::new(),
            counter: AtomicIsize::new(n),
        };

        CountdownEvent {
            inner: Arc::new(inner),
        }
    }

    /// Get the number of times decrement must be called to trigger notifying all listeners
    pub fn get(&self) -> isize {
        self.inner.counter.load(Ordering::Acquire)
    }

    /// Decrement the counter by one. If this is the Nth call, trigger all listeners
    pub fn decrement(&self) {
        // If we are the last decrementer, notify listeners
        let value = self.inner.counter.fetch_sub(1, Ordering::AcqRel);
        if value <= 1 {
            self.inner.event.notify(std::usize::MAX);

            // Reset to 0 - wrapping an isize negative seems unlikely but should probably do it
            // anyways.
            self.inner.counter.store(0, Ordering::Release);
        }
    }

    /// Resets the counter. Any listens following this point will not be notified until decrement
    /// is called N times
    pub fn reset(&self, n: isize) {
        self.inner.counter.store(n, Ordering::Release);
    }

    /// Awaits decrement being called N times
    pub async fn listen(&self) {
        let mut listener = None;

        // The complexity here is due to Event not necessarily signalling awaits that are placed
        // after the await is called. So we must check the counter AFTER taking a listener.
        loop {
            // We're done, break
            if self.inner.counter.load(Ordering::Acquire) <= 0 {
                break;
            }

            match listener.take() {
                None => {
                    listener = Some(self.inner.event.listen());
                }
                Some(l) => {
                    l.await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_event_ready_after() {
        let countdown_event = CountdownEvent::new(2);
        countdown_event.decrement();
        countdown_event.decrement();
        futures_lite::future::block_on(countdown_event.listen());
    }

    #[test]
    fn countdown_event_ready() {
        let countdown_event = CountdownEvent::new(2);
        countdown_event.decrement();
        let countdown_event_clone = countdown_event.clone();
        let handle = std::thread::spawn(move || {
            futures_lite::future::block_on(countdown_event_clone.listen())
        });

        // Pause to give the new thread time to start blocking (ugly hack)
        std::thread::sleep(instant::Duration::from_millis(100));

        countdown_event.decrement();
        handle.join().unwrap();
    }

    #[test]
    fn event_resets_if_listeners_are_cleared() {
        let event = Event::new();

        // notify all listeners
        let listener1 = event.listen();
        event.notify(std::usize::MAX);
        futures_lite::future::block_on(listener1);

        // If all listeners are notified, the structure should now be cleared. We're free to listen
        // again
        let listener2 = event.listen();
        let listener3 = event.listen();

        // Verify that we are still blocked
        assert!(!listener2.wait_timeout(instant::Duration::from_millis(10)));

        // Notify all and verify the remaining listener is notified
        event.notify(std::usize::MAX);
        futures_lite::future::block_on(listener3);
    }
}
