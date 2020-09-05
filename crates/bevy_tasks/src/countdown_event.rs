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

    /// Decrement the counter by one. If this is the Nth call, trigger all listeners
    pub fn decrement(&self) {
        // If we are the last decrementer, notify listeners. notify inserts a SeqCst fence so
        // relaxed is sufficient.
        let value = self.inner.counter.fetch_sub(1, Ordering::Relaxed);
        if value <= 1 {
            self.inner.event.notify(std::usize::MAX);

            // Reset to 0 - wrapping an isize negative seems unlikely but should probably do it
            // anyways.
            self.inner.counter.store(0, Ordering::Relaxed);
        }
    }

    /// Awaits decrement being called N times
    pub async fn listen(&self) {
        let mut listener = None;

        // The complexity here is due to Event not necessarily signalling awaits that are placed
        // after the await is called. So we must check the counter AFTER taking a listener.
        // listen() inserts a SeqCst fence so relaxed is sufficient.
        loop {
            // We're done, break
            if self.inner.counter.load(Ordering::Relaxed) <= 0 {
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

#[test]
pub fn countdown_event_ready_after() {
    let countdown_event = CountdownEvent::new(2);
    countdown_event.decrement();
    countdown_event.decrement();
    pollster::block_on(countdown_event.listen());
}

#[test]
pub fn countdown_event_ready() {
    let countdown_event = CountdownEvent::new(2);
    countdown_event.decrement();
    let countdown_event_clone = countdown_event.clone();
    let handle = std::thread::spawn(move || pollster::block_on(countdown_event_clone.listen()));

    // Pause to give the new thread time to start blocking (ugly hack)
    std::thread::sleep(std::time::Duration::from_millis(100));

    countdown_event.decrement();
    handle.join();
}
