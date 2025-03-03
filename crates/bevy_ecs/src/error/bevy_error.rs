use alloc::boxed::Box;
use core::{
    error::Error,
    fmt::{Debug, Display},
};

/// The built in "universal" Bevy error type. This has a blanket [`From`] impl for any type that implements Rust's [`Error`],
/// meaning it can be used as a "catch all" error.
///
/// # Backtraces
///
/// When used with the `backtrace` Cargo feature, it will capture a backtrace when the error is constructed (generally in the [`From`] impl]).
/// When printed, the backtrace will be displayed. By default, the backtrace will be trimmed down to filter out noise. To see the full backtrace,
/// set the `BEVY_BACKTRACE=full` environment variable.
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// fn fallible_system() -> Result<(), BevyError> {
///     // This will result in Rust's built-in ParseIntError, which will automatically
///     // be converted into a BevyError.
///     let parsed: usize = "I am not a number".parse()?;
///     Ok(())
/// }
/// ```
pub struct BevyError {
    inner: Box<dyn InnerError>,
}

impl BevyError {
    /// Creates a new error with the given message.
    pub fn message<M: Display + Debug + Send + Sync + 'static>(message: M) -> Self {
        BevyError {
            inner: Box::new(ErrorImpl {
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
                error: MessageError(message),
            }),
        }
    }

    /// Attempts to downcast the internal error to the given type.
    pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
        self.inner.error().downcast_ref::<E>()
    }
}

trait InnerError: Send + Sync + 'static {
    #[cfg(feature = "backtrace")]
    fn backtrace(&self) -> &std::backtrace::Backtrace;
    fn error(&self) -> &(dyn Error + Send + Sync + 'static);
}

struct ErrorImpl<E: Error + Send + Sync + 'static> {
    error: E,
    #[cfg(feature = "backtrace")]
    backtrace: std::backtrace::Backtrace,
}

impl<E: Error + Send + Sync + 'static> InnerError for ErrorImpl<E> {
    #[cfg(feature = "backtrace")]
    fn backtrace(&self) -> &std::backtrace::Backtrace {
        &self.backtrace
    }

    fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        &self.error
    }
}

impl<E> From<E> for BevyError
where
    E: Error + Send + Sync + 'static,
{
    #[cold]
    fn from(error: E) -> Self {
        BevyError {
            inner: Box::new(ErrorImpl {
                error,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
}

impl Display for BevyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{}", self.inner.error())?;
        Ok(())
    }
}

impl Debug for BevyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{:?}", self.inner.error())?;
        #[cfg(feature = "backtrace")]
        {
            let backtrace = self.inner.backtrace();
            if let std::backtrace::BacktraceStatus::Captured = backtrace.status() {
                let full_backtrace = std::env::var("BEVY_BACKTRACE").is_ok_and(|val| val == "full");

                let backtrace_str = alloc::string::ToString::to_string(backtrace);
                let mut skip_next = false;
                for line in backtrace_str.split('\n') {
                    if skip_next {
                        skip_next = false;
                        continue;
                    }
                    if !full_backtrace {
                        if line.starts_with("   0: <bevy_ecs::error::bevy_error::BevyError as core::convert::From<E>>::from") {
                            skip_next = true;
                            continue;
                        }
                        if line.starts_with("   1: <core::result::Result<T,F> as core::ops::try_trait::FromResidual<core::result::Result<core::convert::Infallible,E>>>::from_residual") {
                            skip_next = true;
                            continue;
                        }
                        if line.contains("__rust_begin_short_backtrace") {
                            break;
                        }
                        if line.contains("bevy_ecs::observer::Observers::invoke::{{closure}}") {
                            break;
                        }
                    }
                    writeln!(f, "{}", line)?;
                }
                if !full_backtrace {
                    if std::thread::panicking() {
                        SKIP_NORMAL_BACKTRACE.store(1, core::sync::atomic::Ordering::Relaxed);
                    }
                    writeln!(f, "note: Some \"noisy\" backtrace lines have been filtered out. Run with `BEVY_BACKTRACE=full` for a verbose backtrace.")?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "backtrace")]
static SKIP_NORMAL_BACKTRACE: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// When called, this will skip the currently configured panic hook when a [`BevyError`] backtrace has already been printed.
#[cfg(feature = "std")]
pub fn bevy_error_panic_hook(
    current_hook: impl Fn(&std::panic::PanicHookInfo),
) -> impl Fn(&std::panic::PanicHookInfo) {
    move |info| {
        if SKIP_NORMAL_BACKTRACE.load(core::sync::atomic::Ordering::Relaxed) > 0 {
            if let Some(payload) = info.payload().downcast_ref::<&str>() {
                std::println!("{payload}");
            } else if let Some(payload) = info.payload().downcast_ref::<alloc::string::String>() {
                std::println!("{payload}");
            }
            SKIP_NORMAL_BACKTRACE.store(0, core::sync::atomic::Ordering::Relaxed);
            return;
        }

        current_hook(info);
    }
}

/// An error containing a print-able message.
pub struct MessageError<M>(pub(crate) M);

impl<M> Display for MessageError<M>
where
    M: Display + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl<M> Debug for MessageError<M>
where
    M: Display + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<M> Error for MessageError<M> where M: Display + Debug + 'static {}

/// Returns a Result containing a given message if the given value does not exist.
pub trait OkOrMessage<T> {
    /// Returns a Result containing a given message if the given value does not exist.
    fn ok_or_message<M: Display + Debug + Send + Sync + 'static>(
        self,
        message: M,
    ) -> Result<T, MessageError<M>>;
}

impl<T> OkOrMessage<T> for Option<T> {
    fn ok_or_message<M: Display + Debug + Send + Sync + 'static>(
        self,
        message: M,
    ) -> Result<T, MessageError<M>> {
        match self {
            Some(value) => Ok(value),
            None => Err(MessageError(message)),
        }
    }
}
