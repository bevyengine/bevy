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
    inner: Box<InnerBevyError>,
}

impl BevyError {
    /// Attempts to downcast the internal error to the given type.
    pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
        self.inner.error.downcast_ref::<E>()
    }

    fn format_backtrace(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(feature = "backtrace")]
        {
            let f = _f;
            let backtrace = &self.inner.backtrace;
            if let std::backtrace::BacktraceStatus::Captured = backtrace.status() {
                let full_backtrace = std::env::var("BEVY_BACKTRACE").is_ok_and(|val| val == "full");

                let backtrace_str = alloc::string::ToString::to_string(backtrace);
                let mut skip_next_location_line = false;
                for line in backtrace_str.split('\n') {
                    if !full_backtrace {
                        if skip_next_location_line {
                            if line.starts_with("             at") {
                                continue;
                            }
                            skip_next_location_line = false;
                        }
                        if line.contains("std::backtrace_rs::backtrace::") {
                            skip_next_location_line = true;
                            continue;
                        }
                        if line.contains("std::backtrace::Backtrace::") {
                            skip_next_location_line = true;
                            continue;
                        }
                        if line.contains("<bevy_ecs::error::bevy_error::BevyError as core::convert::From<E>>::from") {
                            skip_next_location_line = true;
                            continue;
                        }
                        if line.contains("<core::result::Result<T,F> as core::ops::try_trait::FromResidual<core::result::Result<core::convert::Infallible,E>>>::from_residual") {
                            skip_next_location_line = true;
                            continue;
                        }
                        if line.contains("__rust_begin_short_backtrace") {
                            break;
                        }
                        if line.contains("bevy_ecs::observer::Observers::invoke::{{closure}}") {
                            break;
                        }
                    }
                    writeln!(f, "{line}")?;
                }
                if !full_backtrace {
                    if std::thread::panicking() {
                        SKIP_NORMAL_BACKTRACE.set(true);
                    }
                    writeln!(f, "{FILTER_MESSAGE}")?;
                }
            }
        }
        Ok(())
    }
}

/// This type exists (rather than having a `BevyError(Box<dyn InnerBevyError)`) to make [`BevyError`] use a "thin pointer" instead of
/// a "fat pointer", which reduces the size of our Result by a usize. This does introduce an extra indirection, but error handling is a "cold path".
/// We don't need to optimize it to that degree.
/// PERF: We could probably have the best of both worlds with a "custom vtable" impl, but thats not a huge priority right now and the code simplicity
/// of the current impl is nice.
struct InnerBevyError {
    error: Box<dyn Error + Send + Sync + 'static>,
    #[cfg(feature = "backtrace")]
    backtrace: std::backtrace::Backtrace,
}

// NOTE: writing the impl this way gives us From<&str> ... nice!
impl<E> From<E> for BevyError
where
    Box<dyn Error + Send + Sync + 'static>: From<E>,
{
    #[cold]
    fn from(error: E) -> Self {
        BevyError {
            inner: Box::new(InnerBevyError {
                error: error.into(),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
}

impl Display for BevyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{}", self.inner.error)?;
        self.format_backtrace(f)?;
        Ok(())
    }
}

impl Debug for BevyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{:?}", self.inner.error)?;
        self.format_backtrace(f)?;
        Ok(())
    }
}

#[cfg(feature = "backtrace")]
const FILTER_MESSAGE: &str = "note: Some \"noisy\" backtrace lines have been filtered out. Run with `BEVY_BACKTRACE=full` for a verbose backtrace.";

#[cfg(feature = "backtrace")]
std::thread_local! {
    static SKIP_NORMAL_BACKTRACE: core::cell::Cell<bool> =
        const { core::cell::Cell::new(false) };
}

/// When called, this will skip the currently configured panic hook when a [`BevyError`] backtrace has already been printed.
#[cfg(feature = "backtrace")]
#[expect(clippy::print_stdout, reason = "Allowed behind `std` feature gate.")]
pub fn bevy_error_panic_hook(
    current_hook: impl Fn(&std::panic::PanicHookInfo),
) -> impl Fn(&std::panic::PanicHookInfo) {
    move |info| {
        if SKIP_NORMAL_BACKTRACE.replace(false) {
            if let Some(payload) = info.payload().downcast_ref::<&str>() {
                std::println!("{payload}");
            } else if let Some(payload) = info.payload().downcast_ref::<alloc::string::String>() {
                std::println!("{payload}");
            }
            return;
        }

        current_hook(info);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(not(miri))] // miri backtraces are weird
    #[cfg(not(windows))] // the windows backtrace in this context is ... unhelpful and not worth testing
    fn filtered_backtrace_test() {
        fn i_fail() -> crate::error::Result {
            let _: usize = "I am not a number".parse()?;
            Ok(())
        }

        // SAFETY: this is not safe ...  this test could run in parallel with another test
        // that writes the environment variable. We either accept that so we can write this test,
        // or we don't.

        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };

        let error = i_fail().err().unwrap();
        let debug_message = alloc::format!("{error:?}");
        let mut lines = debug_message.lines().peekable();
        assert_eq!(
            "ParseIntError { kind: InvalidDigit }",
            lines.next().unwrap()
        );

        // On mac backtraces can start with Backtrace::create
        let mut skip = false;
        if let Some(line) = lines.peek() {
            if &line[6..] == "std::backtrace::Backtrace::create" {
                skip = true;
            }
        }

        if skip {
            lines.next().unwrap();
        }

        let expected_lines = alloc::vec![
            "bevy_ecs::error::bevy_error::tests::filtered_backtrace_test::i_fail",
            "bevy_ecs::error::bevy_error::tests::filtered_backtrace_test",
            "bevy_ecs::error::bevy_error::tests::filtered_backtrace_test::{{closure}}",
            "core::ops::function::FnOnce::call_once",
        ];

        for expected in expected_lines {
            let line = lines.next().unwrap();
            assert_eq!(&line[6..], expected);
            let mut skip = false;
            if let Some(line) = lines.peek() {
                if line.starts_with("             at") {
                    skip = true;
                }
            }

            if skip {
                lines.next().unwrap();
            }
        }

        // on linux there is a second call_once
        let mut skip = false;
        if let Some(line) = lines.peek() {
            if &line[6..] == "core::ops::function::FnOnce::call_once" {
                skip = true;
            }
        }
        if skip {
            lines.next().unwrap();
        }
        let mut skip = false;
        if let Some(line) = lines.peek() {
            if line.starts_with("             at") {
                skip = true;
            }
        }

        if skip {
            lines.next().unwrap();
        }
        assert_eq!(super::FILTER_MESSAGE, lines.next().unwrap());
        assert!(lines.next().is_none());
    }
}
