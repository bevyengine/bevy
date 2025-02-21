use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};
use serde::{
    de::{Error, Visitor},
    Deserialize,
};

/// A debug struct used for error messages that displays a list of expected values.
///
/// # Example
///
/// ```ignore (Can't import private struct from doctest)
/// let expected = vec!["foo", "bar", "baz"];
/// assert_eq!("`foo`, `bar`, `baz`", format!("{}", ExpectedValues(expected)));
/// ```
pub(super) struct ExpectedValues<T: Display>(pub Vec<T>);

impl<T: Display> FromIterator<T> for ExpectedValues<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: Display> Debug for ExpectedValues<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let len = self.0.len();
        for (index, item) in self.0.iter().enumerate() {
            write!(f, "`{item}`")?;
            if index < len - 1 {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

/// Represents a simple reflected identifier.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct Ident(pub String);

impl<'de> Deserialize<'de> for Ident {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct IdentVisitor;

        impl<'de> Visitor<'de> for IdentVisitor {
            type Value = Ident;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Ident(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Ident(value))
            }
        }

        deserializer.deserialize_identifier(IdentVisitor)
    }
}
