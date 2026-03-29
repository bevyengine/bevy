use bevy_platform::sync::{Arc, Mutex};

/// [`WakeSignaler`] is a custom signaling primitive used in order to fulfill our specific requirements for
/// our async bridge. We need to wait at the sync point, after waking all the futures and only when
/// all the futures have had a chance to run we stop waiting.
/// We need this signaling to occur also if the future is dropped, or if the future panics
/// so we implement the signaling *on* the Drop implementation.
/// This also makes replacing the wake signal automatically drop and signal the previous one.
pub(crate) struct WakeSignaler(
    #[cfg(feature = "std")] Arc<(Mutex<bool>, std::sync::Condvar)>,
    #[cfg(not(feature = "std"))] (),
);
/// Counterpart to the [`WakeSignaler`], the [`WakeWaiter`] waits for the [`WakeSignaler`] to drop and notify.
pub(crate) struct WakeWaiter(
    #[cfg(feature = "std")] Arc<(Mutex<bool>, std::sync::Condvar)>,
    #[cfg(not(feature = "std"))] (),
);

#[inline]
pub(crate) fn pair() -> (WakeSignaler, WakeWaiter) {
    #[cfg(feature = "std")]
    let inner = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    #[cfg(not(feature = "std"))]
    let inner = ();
    (WakeSignaler(inner.clone()), WakeWaiter(inner))
}

impl WakeWaiter {
    /// Waits until another cloned instance of [`WakeSignaler`] has been dropped.
    /// If any cloned instance of [`WakeSignaler`] is dropped then this wait stops waiting.
    #[cfg(feature = "std")]
    #[inline]
    pub(crate) fn wait(&self) {
        #[cfg(feature = "std")]
        {
            let (lock, cv) = &*self.0;
            let mut signaled = lock.lock().unwrap();
            while !*signaled {
                signaled = cv.wait(signaled).unwrap();
            }
        }
        #[cfg(not(feature = "std"))]
        {
            // No-op on std, since we are only using local futures we should tick them
            // prior to reaching this point.
            return;
        }
    }
}
impl Drop for WakeSignaler {
    #[cfg(feature = "std")]
    #[inline]
    fn drop(&mut self) {
        let (lock, cv) = &*self.0;
        let mut signaled = lock.lock().unwrap();
        *signaled = true;
        cv.notify_one();
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    fn drop(&mut self) {}
}
