use core::{error::Error, fmt};

/// Unpack `Self<T>` to `T`, otherwise return [`Unpack::Error`].
pub trait Unpack<T> {
    /// The error type returned by [`Unpack::unpack`].
    type Error;

    /// Convert `Self<T>` to a `Result<T, Self::Error>`.
    fn unpack(self) -> Result<T, Self::Error>;
}

impl<T> Unpack<T> for Option<T> {
    type Error = NoneError;

    fn unpack(self) -> Result<T, Self::Error> {
        self.ok_or(NoneError)
    }
}

/// A custom type which implements [`Error`], used to indicate that an [`Option`] was [`None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoneError;

impl fmt::Display for NoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unexpected None value.")
    }
}

impl Error for NoneError {}

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
}
