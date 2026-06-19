use alloc::{borrow::Cow, boxed::Box};
use core::{
    error::Error,
    fmt::{Debug, Display},
};

/// The built in "universal" Bevy error type. This has a blanket [`From`] impl for any type that implements Rust's [`Error`],
/// meaning it can be used as a "catch all" error.
///
/// # Severity
///
/// Each [`BevyError`] carries a [`Severity`] value that indicates how serious the error is.
/// While the levels within [`Severity`] correspond to traditional logging levels,
/// these levels are fundamentally advisory metadata.
/// The fallback error handler ultimately has discretion to respond to each of these errors
/// according to its configuration.
/// The error handler ultimately has discretion to respond to each of these errors according to its configuration.
/// You can change the behavior of the fallback handler by modifying the [`FallbackErrorHandler`] resource.
///
/// By default, errors without an assigned severity use [`Severity::Panic`], and will cause your application to panic.
/// You can change the severity of an error by using [`with_severity`], or [`map_severity`] on any [`Result`] type.
///
/// [`FallbackErrorHandler`]: crate::error::handler::FallbackErrorHandler
/// [`with_severity`]: ResultSeverityExt::with_severity
/// [`map_severity`]: ResultSeverityExt::map_severity
///
/// # Backtraces
///
/// When used with the `backtrace` Cargo feature, it can capture a backtrace when the error is constructed (generally in the [`From`] impl).
///
/// To enable backtrace capture on supported platforms,
/// set the `RUST_BACKTRACE` environment variable.
/// See [`Backtrace::capture`] for details.
///
/// When the error is printed, the backtrace will be displayed.
/// By default, the backtrace will be trimmed down to filter out noise.
/// To see the full backtrace, set the `BEVY_BACKTRACE=full` environment variable.
///
/// [`Backtrace::capture`]: https://doc.rust-lang.org/std/backtrace/struct.Backtrace.html#method.capture
///
/// # Context
///
/// You can attach a context message to a [`Result`] or [`Option`] value to turn it into
/// a [`Result`] with a [`BevyError`] using [`context`] or [`with_context`].
/// The resulting error will have the message passed to [`context`] added to it.
///
/// [`context`]: ContextExt::context
/// [`with_context`]: ContextExt::with_context
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// fn fallible_system() -> Result<(), BevyError> {
///     // This will result in Rust's built-in ParseIntError, which will automatically
///     // be converted into a BevyError with an additional message.
///     let parsed: usize = "I am not a number".parse().context("failed to parse number")?;
///     Ok(())
/// }
/// ```
pub struct BevyError {
    inner: Box<InnerBevyError>,
}

impl BevyError {
    /// Constructs a new [`BevyError`] with the given [`Severity`].
    ///
    /// The error will be stored as a `Box<dyn Error + Send + Sync>`.
    ///
    /// The easiest way to use this is to pass in a string.
    /// This works because any type that can be converted into a `Box<dyn Error + Send + Sync>` can be used,
    /// and [`str`] is one such type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::error::{BevyError, Severity};
    ///
    /// fn some_function(val: i64) -> Result<(), BevyError> {
    ///     if val < 0 {
    ///         // Consider using the bevy_error! or even the bail! macro for format! support
    ///         let error =
    ///             BevyError::new(Severity::Panic, format!("Value can't be negative {val}"));
    ///         return Err(error);
    ///     }
    ///
    ///     // ...
    ///     Ok(())
    /// }
    /// ```
    pub fn new<E>(severity: Severity, error: E) -> Self
    where
        Box<dyn Error + Sync + Send>: From<E>,
    {
        Self::from(error).with_severity(severity)
    }

    /// Constructs a new [`BevyError`] with the given [`Severity`].
    ///
    /// Like [`BevyError::new`], but if the `backtrace` cargo feature is enabled
    /// it will use the supplied backtrace instead of capturing a new one.
    #[cfg(feature = "std")]
    pub fn new_with_backtrace<E>(
        severity: Severity,
        error: E,
        backtrace: std::backtrace::Backtrace,
    ) -> Self
    where
        Box<dyn Error + Sync + Send>: From<E>,
    {
        #[cfg(not(feature = "backtrace"))]
        drop(backtrace);
        BevyError {
            inner: Box::new(InnerBevyError {
                error: error.into(),
                severity,
                context: alloc::vec![],
                #[cfg(feature = "backtrace")]
                backtrace,
            }),
        }
    }

    /// Creates a new [`BevyError`] with the [`Severity::Ignore`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Ignore, error)](BevyError::new)</code>.
    pub fn ignore<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Ignore, error)
    }

    /// Creates a new [`BevyError`] with the [`Severity::Trace`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Trace, error)](BevyError::new)</code>.
    pub fn trace<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Trace, error)
    }

    /// Creates a new [`BevyError`] with the [`Severity::Debug`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Debug, error)](BevyError::new)</code>.
    pub fn debug<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Debug, error)
    }

    /// Creates a new [`BevyError`] with the [`Severity::Info`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Info, error)](BevyError::new)</code>.
    pub fn info<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Info, error)
    }

    /// Creates a new [`BevyError`] with the [`Severity::Warning`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Warning, error)](BevyError::new)</code>.
    pub fn warning<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Warning, error)
    }

    /// Creates a new [`BevyError`] with the [`Severity::Error`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Error, error)](BevyError::new)</code>.
    pub fn error<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Error, error)
    }

    /// Creates a new [`BevyError`] with the [`Severity::Panic`] severity.
    ///
    /// This is a shorthand for <code>[BevyError::new(Severity::Panic, error)](BevyError::new)</code>.
    pub fn panic<E>(error: E) -> Self
    where
        Box<dyn Error + Send + Sync>: From<E>,
    {
        Self::new(Severity::Panic, error)
    }

    /// Checks if the internal error is of the given type.
    pub fn is<E: Error + 'static>(&self) -> bool {
        self.inner.error.is::<E>()
    }

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
                // TODO: Cache
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
/// a "fat pointer", which reduces the size of our `Result` by a `usize`. This does introduce an extra indirection, but error handling is a "cold path".
/// We don't need to optimize it to that degree.
/// PERF: We could probably have the best of both worlds with a "custom vtable" impl, but that's not a huge priority right now and the code simplicity
/// of the current impl is nice.
struct InnerBevyError {
    error: Box<dyn Error + Send + Sync + 'static>,
    context: alloc::vec::Vec<Cow<'static, str>>,
    severity: Severity,
    #[cfg(feature = "backtrace")]
    backtrace: std::backtrace::Backtrace,
}

/// Indicates how severe a [`BevyError`] is.
///
/// These levels correspond to traditional logging levels,
/// but the severity is advisory metadata used by error handlers to decide how to react (for example: ignore, log, or panic).
///
/// To change the behavior of unhandled errors returned from systems,
/// you can modify the [fallback error handler], and read the [`Severity`] stored inside of each [`BevyError`].
///
/// You can change the severity of an error (including assigning an error severity) to an ordinary result
/// by calling [`with_severity`] or [`map_severity`].
///
/// [`with_severity`]: ResultSeverityExt::with_severity
/// [`map_severity`]: ResultSeverityExt::map_severity
/// [fallback error handler]: crate::error::handler::FallbackErrorHandler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum Severity {
    /// The error can be safely ignored, and can be completely discarded.
    Ignore,
    /// The error can be ignored, unless verbose debugging is required.
    Trace,
    /// The error can be safely ignored, but may need to be surfaced during debugging.
    Debug,
    /// Nothing has gone wrong, but the error is useful to the user and should be reported.
    Info,
    /// Something unexpected but recoverable happened.
    ///
    /// Something has probably gone wrong.
    Warning,
    /// A real error occurred, but the program may continue.
    Error,
    /// A fatal error; the program cannot continue.
    Panic,
}

impl BevyError {
    /// Returns the severity of this error.
    pub fn severity(&self) -> Severity {
        self.inner.severity
    }

    /// Returns this error with its severity overridden.
    ///
    /// Note that this doesn't change the underlying error value;
    /// only the [`Severity`] metadata used by the error handler.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.inner.severity = severity;
        self
    }
}

/// Extension methods for annotating errors with a [`Severity`].
pub trait ResultSeverityExt<T, E>: Sized {
    /// Overrides the [`Severity`] of the error if this result is `Err`.
    /// This does not change control flow; it only annotates the error.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::error::{BevyError, ResultSeverityExt, Severity};
    /// fn fallible() -> Result<(), BevyError> {
    ///     // This failure is expected in some contexts, so we downgrade its severity.
    ///     let _parsed: usize = "I am not a number"
    ///         .parse()
    ///         .with_severity(Severity::Warning)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// For more fine grained control see [`Result::map_severity`](ResultSeverityExt::map_severity)
    fn with_severity(self, severity: Severity) -> Result<T, BevyError>;

    /// Overrides the [`Severity`] of the error if this result is `Err`.
    /// This does not change control flow; it only annotates the error.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::error::{BevyError, ResultSeverityExt, Severity};
    /// # use thiserror::Error;
    /// # fn validate(_string: &str) -> Result<usize, ValidationError> {
    /// #     Err(ValidationError::IncorrectVersion)
    /// # }
    ///
    /// #[derive(Error, Debug)]
    /// pub enum ValidationError {
    ///     #[error("Incorrect version")]
    ///     IncorrectVersion,
    ///     #[error("Syntax error")]
    ///     SyntaxError,
    /// }
    ///
    /// fn fallible() -> Result<(), BevyError> {
    ///     // This failure is expected in some contexts, so we downgrade its severity.
    ///     let _parsed: usize = validate("I am not a number")
    ///         .map_severity(|e| match e {
    ///             ValidationError::IncorrectVersion => Severity::Debug,
    ///             ValidationError::SyntaxError => Severity::Error,
    ///         })?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// If you don't need to inspect the error, use [`Result::with_severity`](ResultSeverityExt::with_severity)
    fn map_severity(self, f: impl FnOnce(&E) -> Severity) -> Result<T, BevyError>;

    /// Overrides the severity of the error with [`Severity::Ignore`]. See [`Result::with_severity`]
    ///
    /// This is shorthand for `self.with_severity(Severity::Ignore)`
    fn ignore(self) -> Result<T, BevyError> {
        self.with_severity(Severity::Ignore)
    }

    /// Overrides the severity of the error with [`Severity::Trace`]. See [`Result::with_severity`]
    ///
    /// This is shorthand for `self.with_severity(Severity::Trace)`
    fn trace(self) -> Result<T, BevyError> {
        self.with_severity(Severity::Trace)
    }

    /// Overrides the severity of the error with [`Severity::Info`]. See [`Result::with_severity`]
    ///
    /// This is shorthand for `self.with_severity(Severity::Info)`
    fn info(self) -> Result<T, BevyError> {
        self.with_severity(Severity::Info)
    }

    /// Overrides the severity of the error with [`Severity::Warning`]. See [`Result::with_severity`]
    ///
    /// This is shorthand for `self.with_severity(Severity::Warning)`
    fn warn(self) -> Result<T, BevyError> {
        self.with_severity(Severity::Warning)
    }

    /// Overrides the severity of the error with [`Severity::Error`]. See [`Result::with_severity`]
    ///
    /// This is shorthand for `self.with_severity(Severity::Error)`
    fn error(self) -> Result<T, BevyError> {
        self.with_severity(Severity::Error)
    }

    /// Overrides the severity of the error with [`Severity::Panic`]. See [`Result::with_severity`]
    ///
    /// This is shorthand for `self.with_severity(Severity::Panic)`
    fn panic(self) -> Result<T, BevyError> {
        self.with_severity(Severity::Panic)
    }
}

impl<T, E> ResultSeverityExt<T, E> for Result<T, E>
where
    E: Into<BevyError>,
{
    fn with_severity(self, severity: Severity) -> Result<T, BevyError> {
        self.map_err(|e| e.into().with_severity(severity))
    }

    fn map_severity(self, f: impl FnOnce(&E) -> Severity) -> Result<T, BevyError> {
        self.map_err(|e| {
            let severity = f(&e);
            e.into().with_severity(severity)
        })
    }
}

/// Extension methods for adding additional context messages to a [`BevyError`]
pub trait ContextExt<T>: Sized {
    /// Annotate the error with a context message.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::error::{BevyError, ContextExt};
    /// fn fallible() -> Result<(), BevyError> {
    ///     // Produces a `BevyError` with the message
    ///     // "failed to parse number: invalid digit found in string"
    ///     let _parsed: usize = "I am not a number"
    ///         .parse()
    ///         .context("failed to parse number")?;
    ///
    ///     Ok(())
    /// }
    /// ```
    fn context<C>(self, context: C) -> Result<T, BevyError>
    where
        C: Into<Cow<'static, str>>,
    {
        self.with_context(move || context)
    }

    /// Annotate the error with a context message from a closure
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::error::{BevyError, ContextExt};
    /// # use std::fs;
    /// fn fallible() -> Result<(), BevyError> {
    ///     let path = "some_file.txt";
    ///     let _message = fs::read_to_string(path)
    ///         .with_context(|| format!("failed to read {path}"))?;
    ///
    ///     Ok(())
    /// }
    /// ```
    fn with_context<C>(self, context: impl FnOnce() -> C) -> Result<T, BevyError>
    where
        C: Into<Cow<'static, str>>;
}
impl<T, E> ContextExt<T> for Result<T, E>
where
    E: Into<BevyError>,
{
    fn with_context<C>(self, context: impl FnOnce() -> C) -> Result<T, BevyError>
    where
        C: Into<Cow<'static, str>>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(error) => {
                let mut error = error.into();
                let message = context().into();
                error.inner.context.push(message);
                Err(error)
            }
        }
    }
}

impl<T> ContextExt<T> for Option<T> {
    fn with_context<C>(self, context: impl FnOnce() -> C) -> Result<T, BevyError>
    where
        C: Into<Cow<'static, str>>,
    {
        match self {
            Some(v) => Ok(v),
            None => {
                let message = context().into();

                Err(message.into())
            }
        }
    }
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
                severity: Severity::Panic,
                context: alloc::vec![],
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
}

impl Display for BevyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.inner.context {
            context if context.is_empty() => writeln!(f, "{}", self.inner.error)?,
            context if context.len() == 1 => {
                writeln!(f, "{}: {}", context[0].trim(), self.inner.error)?;
            }
            context => {
                // The most recent message is the last one in the `Vec`
                // so we need to reverse the iterator
                let mut reversed = context.iter().rev();
                let first = reversed.next().unwrap().trim();

                writeln!(f, "{first}\n\nCaused by:")?;
                for message in reversed {
                    let message = message.trim();
                    writeln!(f, "\t{message}")?;
                }
                writeln!(f, "\t{}", self.inner.error)?;
            }
        }
        self.format_backtrace(f)?;
        Ok(())
    }
}

impl Debug for BevyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{:?}", self.inner.error)?;
        if !self.inner.context.is_empty() {
            writeln!(f, "context: {:?}", self.inner.context)?;
        }
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
            if let Some(payload) = info.payload_as_str() {
                std::println!("{payload}");
            }
            return;
        }

        current_hook(info);
    }
}

/// Creates a new [`BevyError`] from a string.
///
/// Strings can be formatted like the [`format!`](std::format!) macro. Severity
/// can optionally be provided to change it from the default [`Severity::Panic`].
/// This can be done by adding the severity as the fist argument.
///
/// # Example
/// ```
/// use bevy_ecs::{bevy_error, error::{BevyError, Severity}};
///
/// fn this_will_fail(value: u32) -> Result<(), BevyError> {
///     if value == 0 {
///         return Err(bevy_error!(Severity::Debug, "A debug message"));
///     } else {
///         return Err(bevy_error!("We can even do formatting {value}, {}", "hello"));
///     }
/// }
/// ```
#[macro_export]
macro_rules! bevy_error {
    ($fmt:literal) => {
        $crate::error::BevyError::new($crate::error::Severity::Panic, $fmt)
    };
    ($fmt:literal, $($arg:tt)*) => {
        $crate::error::BevyError::new($crate::error::Severity::Panic, $crate::__macro_exports::format!($fmt, $($arg)*))
    };
    ($severity:expr, $fmt:literal) => {
        $crate::error::BevyError::new($severity, $fmt)
    };
    ($severity:expr, $fmt:literal, $($arg:tt)*) => {
        $crate::error::BevyError::new($severity, $crate::__macro_exports::format!($fmt, $($arg)*))
    };
    ($severity:expr) => {
        compile_error!("missing error message")
    };
}

/// Returns early with an error.
///
/// Equivalent to <code>return Err([bevy_error!(\...)](bevy_error!))</code>
/// As a result the returned error defaults to [`Severity::Panic`]. As with
/// `bevy_error!` the severity can be changed by providing a severity as the
/// first argument
///
/// # Example
/// ```
/// use bevy_ecs::{bail, error::{BevyError, Severity}};
///
/// fn do_some_stuff(val: i32) -> Result<(), BevyError> {
///     if val < 0 {
///         bail!(Severity::Warning, "Something is broken: {}", val);
///     } else if val == 0 {
///         bail!("Value really can't be zero");
///     }
///
///     // ...
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! bail {
    ($($args:tt)+) => {
        return core::result::Result::Err($crate::bevy_error!($($args)*))
    };
}

#[cfg(test)]
mod tests {
    use crate::error::BevyError;
    use crate::error::ContextExt;
    use alloc::string::ToString;

    #[test]
    #[cfg(not(miri))] // miri backtraces are weird
    #[cfg(not(windows))] // the windows backtrace in this context is ... unhelpful and not worth testing
    fn filtered_backtrace_test() {
        fn i_fail() -> crate::error::Result {
            let _: usize = "I am not a number".parse()?;
            Ok(())
        }

        let capture_backtrace = std::env::var_os("RUST_BACKTRACE");

        if capture_backtrace.is_none() || capture_backtrace.clone().is_some_and(|s| s == "0") {
            panic!("This test only works if rust backtraces are enabled. Value set was {capture_backtrace:?}. Please set RUST_BACKTRACE to any value other than 0 and run again.")
        }

        let error = i_fail().err().unwrap();
        let debug_message = alloc::format!("{error:?}");
        let mut lines = debug_message.lines().peekable();
        assert_eq!(
            "ParseIntError { kind: InvalidDigit }",
            lines.next().unwrap()
        );

        // On mac backtraces can start with Backtrace::create
        // Rust 1.95 changed the format to use angle brackets: <std::backtrace::Backtrace>::create
        let mut skip = false;
        if let Some(line) = lines.peek()
            && (line[6..] == *"std::backtrace::Backtrace::create"
                || line[6..] == *"<std::backtrace::Backtrace>::create")
        {
            skip = true;
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
            if let Some(line) = lines.peek()
                && line.starts_with("             at")
            {
                skip = true;
            }

            if skip {
                lines.next().unwrap();
            }
        }

        // on linux there is a second call_once
        let mut skip = false;
        if let Some(line) = lines.peek()
            && &line[6..] == "core::ops::function::FnOnce::call_once"
        {
            skip = true;
        }

        if skip {
            lines.next().unwrap();
        }
        let mut skip = false;
        if let Some(line) = lines.peek()
            && line.starts_with("             at")
        {
            skip = true;
        }

        if skip {
            lines.next().unwrap();
        }
        assert_eq!(super::FILTER_MESSAGE, lines.next().unwrap());
        assert!(lines.next().is_none());
    }

    #[test]
    fn downcasting() {
        #[derive(Debug, PartialEq)]
        struct Fun(i32);

        impl core::fmt::Display for Fun {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&self, f)
            }
        }
        impl core::error::Error for Fun {}

        let new_error = BevyError::new(crate::error::Severity::Debug, Fun(1));

        assert!(new_error.is::<Fun>());
        assert_eq!(new_error.downcast_ref::<Fun>(), Some(&Fun(1)));
    }

    /// Testing the functionality would be difficult so we at least check if it
    /// compiles.
    #[test]
    fn bevy_error_macro() {
        bevy_error!("One arg");
        bevy_error!(crate::error::Severity::Debug, "With severity");
        bevy_error!(
            crate::error::Severity::Debug,
            "With severity and args {}",
            4 / 3
        );

        // This is the pain in the ass one since both args are literals but neither is severity
        bevy_error!("Format string {}", 1 + 2);
    }

    #[test]
    fn bevy_bail_macro() {
        // Simplest way to specify the return type
        fn t(f: impl Fn() -> Result<(), BevyError>) {
            let val = f();

            assert!(val.is_err(), "expected error got {:?}", val);
        }

        t(|| bail!("One arg"));
        t(|| bail!(crate::error::Severity::Debug, "With severity"));
        t(|| {
            bail!(
                crate::error::Severity::Debug,
                "With severity and args {}",
                2
            )
        });
        t(|| bail!("Format string {}", 1 + 2));
    }

    #[test]
    fn context() {
        let empty = None::<i32>;
        let as_result = empty.context("Didn't have anything!");
        assert!(as_result
            .unwrap_err()
            .to_string()
            .starts_with("Didn't have anything!\n"));

        let err: Result<i32, BevyError> =
            Err(BevyError::new(crate::error::Severity::Debug, "Oh no!"));
        let mut with_context = err.context("Failed");

        assert!(with_context
            .as_ref()
            .unwrap_err()
            .to_string()
            .starts_with("Failed: Oh no!\n"));

        with_context = with_context.context("Something went wrong");
        assert!(with_context.unwrap_err().to_string().starts_with(
            "Something went wrong

Caused by:
\tFailed
\tOh no!
"
        ));
    }

    #[test]
    fn context_downcasting() {
        #[derive(Debug, PartialEq)]
        struct Fun(i32);

        impl core::fmt::Display for Fun {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&self, f)
            }
        }
        impl core::error::Error for Fun {}

        let fun: Result<i32, Fun> = Err(Fun(1));
        let new_error = fun.context("Hello world!");

        assert!(new_error.as_ref().unwrap_err().is::<Fun>());
        assert_eq!(
            new_error.as_ref().unwrap_err().downcast_ref::<Fun>(),
            Some(&Fun(1))
        );

        let new_new_error = new_error.context("Hey there!");

        assert!(new_new_error.as_ref().unwrap_err().is::<Fun>());
        assert_eq!(
            new_new_error.as_ref().unwrap_err().downcast_ref::<Fun>(),
            Some(&Fun(1))
        );
    }
}
