use bevy_platform::sync::{Arc, Mutex};

/// WakeSignal is a custom signaling primitive used in order to fufill our specific requirements for
/// our async bridge. We need to wait at the sync point, after waking all the futures and only when
/// all the futures have had a chance to run we stop waiting.
/// We need this signaling to occur also if the future is dropped, or if the future panics
/// so we implement the signaling *on* the Drop implementation.
/// This also makes replacing the wake signal automatically drop and signal the previous one.
#[derive(Clone)]
#[cfg(feature = "std")]
pub(crate) struct WakeSignal(
    #[cfg(feature = "std")] Arc<(Mutex<bool>, std::sync::Condvar)>,
    #[cfg(not(feature = "std"))] Arc<(Mutex<bool>)>,
);

impl WakeSignal {
    #[inline]
    pub(crate) fn new() -> Self {
        #[cfg(feature = "std")]
        {
            WakeSignal(Arc::new((Mutex::new(false), std::sync::Condvar::new())))
        }
        #[cfg(not(feature = "std"))]
        {
            WakeSignal(Arc::new(Mutex::new(false)))
        }
    }

    /// Waits until another cloned instance of `WakeSignal` has been dropped.
    /// If any cloned instance of `WakeSignal` is dropped then this wait stops waiting.
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
            loop {
                if self.0.lock().unwrap() {
                    break;
                }
            }
        }
    }
}
impl Drop for WakeSignal {
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
    fn drop(&mut self) {
        *self.0.lock().unwrap() = true;
    }
}
