//! Utilities for working with [`Future`]s.
use core::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

/// Consumes a future, polls it once, and immediately returns the output
/// or returns `None` if it wasn't ready yet.
///
/// This will cancel the future if it's not ready.
pub fn now_or_never<F: Future>(future: F) -> Option<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(future).poll(&mut cx) {
        Poll::Ready(x) => Some(x),
        _ => None,
    }
}

/// Polls a future once, and returns the output if ready
/// or returns `None` if it wasn't ready yet.
pub fn check_ready<F: Future + Unpin>(future: &mut F) -> Option<F::Output> {
    now_or_never(future)
}
