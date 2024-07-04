use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use std::task::{Context, Poll};
use bevy_tasks::IoTaskPool;

#[derive(Debug, Copy, Clone)]
pub struct IoTimer {
    expiry: Instant,
}

impl IoTimer {
    pub fn new(expiry: Instant) -> Self {
        Self {
            expiry
        }
    }

    pub fn reset(&mut self, new_expiry: Instant) -> &mut Self {
        self.expiry = new_expiry;
        self
    }

    pub fn expires(&self) -> Instant {
        self.expiry
    }
}

impl Future for IoTimer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let now = Instant::now();

        if now >= self.expiry {
            return Poll::Ready(())
        }
        let waker = cx.waker().clone();
        IoTaskPool::get().spawn(async move {
            waker.wake()
        }).detach();
        Poll::Pending
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Timed<F> {
    inner: F,
    total: Duration
}

impl<F> Timed<F>
where F: Future {
    pub fn new(fut: F) -> Self {
        Self {
            inner: fut,
            total: Duration::ZERO
        }
    }

    pub fn awake(&self) -> Duration {
        self.total
    }
}

pub struct TimedOutput<T> {
    pub output: T,
    pub awake_time: Duration
}

impl<F> Future for Timed<F>
where F: Future {
    type Output = TimedOutput<<F as Future>::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let start = Instant::now();

        match self.inner.poll(cx) {
            Poll::Ready(output) => {
                let time_taken = Instant::now() - start;
                self.total += time_taken;

                Poll::Ready(TimedOutput { output: output, awake_time: self.total })
            }
            Poll::Pending => {
                let time_taken = Instant::now() - start;
                self.total += time_taken;
                Poll::Pending
            }
        }
    }
}

pub fn time_limit<F>(fut: F, limit: Duration) -> TimeLimit<F>
where F: Future {
    TimeLimit::new(fut, limit)
}

impl<F> TimeLimit<F>
where F: Future {
    pub fn new(fut: F, limit: Duration) -> Self {
        Self { 
            inner: Timed::new(fut), 
            limit: limit 
        }
    }
}

pub struct TimeLimit<F> {
    inner: Timed<F>,
    limit: Duration
}

impl<F> Future for TimeLimit<F>
where F: Future {
    type Output = Option<TimedOutput<<F as Future>::Output>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.inner.awake() >= self.limit {
            return Poll::Ready(None)
        }

        match self.inner.poll(cx) {
            Poll::Ready(output) => {
                Poll::Ready(Some(output))
            }
            Poll::Pending => {
                Poll::Pending
            }
        }
    }
}