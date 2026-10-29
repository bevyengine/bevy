//! Platform-aware future utilities.

crate::cfg::switch! {
    #[cfg(feature = "async-io")] => {
        pub use async_io::block_on;
    }
    #[cfg(feature = "futures-lite")] => {
        pub use futures_lite::future::block_on;
    }
    _ => {
        /// Blocks on the supplied `future`.
        /// This implementation will busy-wait until it is completed.
        /// Consider enabling the `async-io` or `futures-lite` features.
        pub fn block_on<T>(future: impl Future<Output = T>) -> T {
            use core::task::{Poll, Context};

            // Pin the future on the stack.
            let mut future = core::pin::pin!(future);

            // We don't care about the waker as we're just going to poll as fast as possible.
            let cx = &mut Context::from_waker(core::task::Waker::noop());

            // Keep polling until the future is ready.
            loop {
                match future.as_mut().poll(cx) {
                    Poll::Ready(output) => return output,
                    Poll::Pending => core::hint::spin_loop(),
                }
            }
        }
    }
}
