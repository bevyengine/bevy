use super::Error;

/// Assume that `Self<T>` is `T`, otherwise return the provided error.
///
/// This can be a drop-in replacement for `expect`, combined with the question mark operator and
/// [`Result`](super::Result) return type, to get the same ergonomics as `expect` but without the
/// panicking behavior (when using a non-panicking error handler).
pub trait Assume<T> {
    /// The error type returned by [`Assume::assume`].
    ///
    /// Typically implements the [`Error`] trait, allowing it to match Bevy's fallible system
    /// [`Result`](super::Result) return type.
    type Error;

    /// Convert `Self<T>` to a `Result<T, Self::Error>`.
    fn assume<E: Into<Self::Error>>(self, err: E) -> Result<T, Self::Error>;
}

impl<T> Assume<T> for Option<T> {
    type Error = Error;

    fn assume<E: Into<Self::Error>>(self, err: E) -> Result<T, Self::Error> {
        self.ok_or_else(|| err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::string::ToString;
    use core::{error::Error, fmt};

    #[test]
    fn test_assume_some() {
        let value: Option<i32> = Some(20);

        match value.assume("Error message") {
            Ok(value) => assert_eq!(value, 20),
            Err(err) => panic!("Unexpected error: {err}"),
        }
    }

    #[test]
    fn test_assume_none_with_str() {
        let value: Option<i32> = None;
        let err = value.assume("index 1 should exist").unwrap_err();
        assert_eq!(err.to_string(), "index 1 should exist");
    }

    #[test]
    fn test_assume_none_with_custom_error() {
        #[derive(Debug)]
        struct MyError;

        impl fmt::Display for MyError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "My custom error")
            }
        }
        impl Error for MyError {}

        let value: Option<i32> = None;
        let err = value.assume(MyError).unwrap_err();
        assert_eq!(err.to_string(), "My custom error");
    }
}
