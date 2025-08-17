use alloc::fmt;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::cfg;

/// Wraps `async_executor::Task`, a spawned future.
///
/// Tasks are also futures themselves and yield the output of the spawned future.
///
/// When a task is dropped, its gets canceled and won't be polled again. To cancel a task a bit
/// more gracefully and wait until it stops running, use the [`Task::cancel()`] method.
///
/// Tasks that panic get immediately canceled. Awaiting a canceled task also causes a panic.
#[must_use = "Tasks are canceled when dropped, use `.detach()` to run them in the background."]
pub struct Task<T>(
    cfg::web! {
        if {
            async_channel::Receiver<Result<T, Panic>>
        } else {
            async_task::Task<T>
        }
    },
);

// Custom constructors for web and non-web platforms
cfg::web! {
    if {
        impl<T: 'static> Task<T> {
            /// Creates a new task by passing the given future to the web
            /// runtime as a promise.
            pub(crate) fn wrap_future(future: impl Future<Output = T> + 'static) -> Self {
                use bevy_platform::exports::wasm_bindgen_futures::spawn_local;
                let (sender, receiver) = async_channel::bounded(1);
                spawn_local(async move {
                    // Catch any panics that occur when polling the future so they can
                    // be propagated back to the task handle.
                    let value = CatchUnwind(AssertUnwindSafe(future)).await;
                    let _ = sender.send(value);
                });
                Self(receiver)
            }
        }
    } else {
        impl<T> Task<T> {
            /// Creates a new task from a given `async_executor::Task`
            pub(crate) fn new(task: async_task::Task<T>) -> Self {
                Self(task)
            }
        }
    }
}

impl<T> Task<T> {
    /// Detaches the task to let it keep running in the background.
    ///
    /// # Platform-Specific Behavior
    ///
    /// When building for the web, this method has no effect.
    pub fn detach(self) {
        cfg::web! {
            if {
                // Tasks are already treated as detached on the web.
            } else {
                self.0.detach();
            }
        }
    }

    /// Cancels the task and waits for it to stop running.
    ///
    /// Returns the task's output if it was completed just before it got canceled, or [`None`] if
    /// it didn't complete.
    ///
    /// While it's possible to simply drop the [`Task`] to cancel it, this is a cleaner way of
    /// canceling because it also waits for the task to stop running.
    ///
    /// # Platform-Specific Behavior
    ///
    /// Canceling tasks is unsupported on the web, and this is the same as awaiting the task.
    pub async fn cancel(self) -> Option<T> {
        cfg::web! {
            if {
                // Await the task and handle any panics.
                match self.0.recv().await {
                    Ok(Ok(value)) => Some(value),
                    Err(_) => None,
                    Ok(Err(panic)) => {
                        // drop this to prevent the panic payload from resuming the panic on drop.
                        // this also leaks the box but I'm not sure how to avoid that
                        core::mem::forget(panic);
                        None
                    }
                }
            } else {
                // Wait for the task to become canceled
                self.0.cancel().await
            }
        }
    }

    /// Returns `true` if the current task is finished.
    ///
    /// Unlike poll, it doesn't resolve the final value, it just checks if the task has finished.
    /// Note that in a multithreaded environment, this task can be finished immediately after calling this function.
    pub fn is_finished(&self) -> bool {
        cfg::web! {
            if {
                // We treat the task as unfinished until the result is sent over the channel.
                !self.0.is_empty()
            } else {
                // Defer to the `async_task` implementation.
                self.0.is_finished()
            }
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    cfg::web! {
        if {
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                // `recv()` returns a future, so we just poll that and hand the result.
                let recv = core::pin::pin!(self.0.recv());
                match recv.poll(cx) {
                    Poll::Ready(Ok(Ok(value))) => Poll::Ready(value),
                    // NOTE: Propagating the panic here sorta has parity with the async_executor behavior.
                    // For those tasks, polling them after a panic returns a `None` which gets `unwrap`ed, so
                    // using `resume_unwind` here is essentially keeping the same behavior while adding more information.
                    Poll::Ready(Ok(Err(_panic))) => crate::cfg::switch! {{
                        crate::cfg::std => {
                            std::panic::resume_unwind(_panic)
                        }
                        _ => {
                            unreachable!("catching a panic is only possible with std")
                        }
                    }},
                    Poll::Ready(Err(_)) => panic!("Polled a task after it finished running"),
                    Poll::Pending => Poll::Pending,
                }
            }
        } else {
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                // `async_task` has `Task` implement `Future`, so we just poll it.
                Pin::new(&mut self.0).poll(cx)
            }
        }
    }
}

// All variants of Task<T> are expected to implement Unpin
impl<T> Unpin for Task<T> {}

// Derive doesn't work for macro types, so we have to implement this manually.
impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// Utilities for catching unwinds on the web.
cfg::web! {
    use alloc::boxed::Box;
    use core::{
        panic::{AssertUnwindSafe, UnwindSafe},
        any::Any,
    };

    type Panic = Box<dyn Any + Send + 'static>;

    #[pin_project::pin_project]
    struct CatchUnwind<F: UnwindSafe>(#[pin] F);

    impl<F: Future + UnwindSafe> Future for CatchUnwind<F> {
        type Output = Result<F::Output, Panic>;
        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let f = AssertUnwindSafe(|| self.project().0.poll(cx));

            let result = cfg::std! {
                if {
                    std::panic::catch_unwind(f)?
                } else {
                    f()
                }
            };

            result.map(Ok)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Task;

    #[test]
    fn task_is_sync() {
        fn is_sync<T: Sync>() {}
        is_sync::<Task<()>>();
    }

    #[test]
    fn task_is_send() {
        fn is_send<T: Send>() {}
        is_send::<Task<()>>();
    }
}
