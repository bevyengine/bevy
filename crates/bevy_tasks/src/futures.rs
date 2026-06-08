//! Utilities for working with [`Future`]s.
use alloc::task::Wake;
use bevy_platform::sync::Arc;
use core::{
    future::Future,
    pin::{pin, Pin},
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

/// Wraps a future such that the Waker given to the future also runs the "kicker".
///
/// This allows us to trigger an action (the "kicker") in addition to just waking the future. The
/// kicker is also triggered when the future resolves (i.e., returns [`Poll::Ready`]).
pub(crate) struct KickOnWake<F> {
    /// The "kicker" that will be invoked when the future wakes up or resolves.
    pub(crate) kicker: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// The inner future.
    pub(crate) f: F,
}

impl<F: Future> Future for KickOnWake<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(kicker) = self.kicker.clone() else {
            #[expect(
                unsafe_code,
                reason = "We need to manually pin so we can support wrapping any future."
            )]
            // SAFETY: We don't move out of `this` inside the closure, and we don't move out of `f`
            // in any case - we assume that pinning `self` also means pinning `self.f`.
            return unsafe { self.map_unchecked_mut(|this| &mut this.f) }.poll(cx);
        };
        let wrapped_waker = Waker::from(Arc::new(KickThenWake {
            kicker,
            waker: cx.waker().clone(),
        }));
        let mut cx = Context::from_waker(&wrapped_waker);
        #[expect(
            unsafe_code,
            reason = "We need to manually pin so we can support wrapping any future."
        )]
        // SAFETY: We don't move out of `this` inside the closure, and we don't move out of `f`
        // in any case - we assume that pinning `self` also means pinning `self.f`.
        let result = unsafe { self.map_unchecked_mut(|this| &mut this.f) }.poll(&mut cx);
        // Also kick if the future resolves.
        if result.is_ready() {
            wrapped_waker.wake_by_ref();
        }
        result
    }
}

/// A waker that wraps another waker, but first executing the "kicker".
struct KickThenWake {
    /// The "kicker" that will be invoked when the future wakes up or resolves.
    kicker: Arc<dyn Fn() + Send + Sync + 'static>,
    /// The actual waker to invoke after the kicker.
    waker: Waker,
}

impl Wake for KickThenWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        (*self.kicker)();
        self.waker.wake_by_ref();
    }
}
