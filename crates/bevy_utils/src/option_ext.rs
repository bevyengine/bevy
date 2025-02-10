//! Extensions to the [`Option`] type used in Bevy.

use crate::alloc::boxed::Box;
use core::{error::Error, fmt};

/// A custom type which implements [`Error`], used to indicate that an `Option` was `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoneError;

impl fmt::Display for NoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unexpected None value.")
    }
}

impl Error for NoneError {}

/// Extension trait for [`Option`].
pub trait OptionExt<T> {
    /// Convert an `Option<T>` to a `Result<T, NoneError>`.
    fn unpack(self) -> Result<T, NoneError>;

    /// Convert an `Option<T>` to a `Result<T, Box<dyn Error>>`.
    fn assume<E: Into<Box<dyn Error>>>(self, err: E) -> Result<T, Box<dyn Error>>;
}

/// Extensions for [`Option`].
impl<T> OptionExt<T> for Option<T> {
    /// Convert an `Option<T>` to a `Result<T, NoneError>`.
    fn unpack(self) -> Result<T, NoneError> {
        self.ok_or(NoneError)
    }

    /// Convert an `Option<T>` to a `Result<T, Box<dyn Error>>`.
    fn assume<E: Into<Box<dyn Error>>>(self, err: E) -> Result<T, Box<dyn Error>> {
        self.ok_or_else(|| err.into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::string::ToString;

    #[test]
    fn test_unpack_some() {
        let value: Option<i32> = Some(10);
        assert_eq!(value.unpack(), Ok(10));
    }

    #[test]
    fn test_unpack_none() {
        let value: Option<i32> = None;
        let err = value.unpack().unwrap_err();
        assert_eq!(err.to_string(), "Unexpected None value.");
    }

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
