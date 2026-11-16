use crate::{kind::KindRef, Reflect, Type};
use thiserror::Error;

impl dyn Reflect {
    /// Returns whether this value is equal to `other`, or an error describing the (possibly nested)
    /// value that failed if they cannot be compared.
    ///
    /// This will use the reflected [`PartialEq`] implementation, if one is present. If not, it will rely
    /// on a "structural" partial equality using the reflected type information.
    ///
    /// Values of different types are never equal.
    pub fn reflect_partial_eq(&self, other: &dyn Reflect) -> Result<bool, ReflectPartialEqError> {
        let ty = self.ty();
        if ty.type_id() != other.ty().type_id() {
            return Ok(false);
        }
        if let Some(partial_eq) = ty.vtable().partial_eq {
            // SAFETY: `Reflect::ty` accurately describes both values, which have the same type
            return Ok(unsafe { partial_eq(self.ptr(), other.ptr()) });
        }
        match (self.kind_ref(), other.kind_ref()) {
            (KindRef::Struct(a), KindRef::Struct(b)) => {
                all_eq(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
            }
            (KindRef::Enum(a), KindRef::Enum(b)) => {
                if a.variant() != b.variant() {
                    return Ok(false);
                }
                all_eq(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
            }
            (KindRef::List(a), KindRef::List(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                all_eq(a.iter(), b.iter())
            }
            (KindRef::Array(a), KindRef::Array(b)) => all_eq(a.iter(), b.iter()),
            (KindRef::Map(a), KindRef::Map(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (key, a_value) in a.iter() {
                    let Some(b_value) = b.get(key) else {
                        return Ok(false);
                    };
                    if !a_value.reflect_partial_eq(b_value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (KindRef::Set(a), KindRef::Set(b)) => {
                Ok(a.len() == b.len() && a.iter().all(|value| b.contains(value)))
            }
            _ => Err(ReflectPartialEqError { ty }),
        }
    }
}

/// An error returned by
/// [`<dyn Reflect>::reflect_partial_eq`](trait.Reflect.html#method.reflect_partial_eq) when part
/// of a value has no reflected [`PartialEq`] implementation and cannot be compared structurally.
#[derive(Error, Debug, Clone, Copy)]
#[error(
    "cannot compare `{}`: it has no reflected `PartialEq` implementation and cannot be compared structurally",
    .ty.type_path().full()
)]
pub struct ReflectPartialEqError {
    /// The type of the (possibly nested) value that could not be compared.
    pub ty: &'static Type,
}

/// Returns whether every pair of values is equal. Both iterators must have the same length.
fn all_eq<'a>(
    a: impl Iterator<Item = &'a dyn Reflect>,
    b: impl Iterator<Item = &'a dyn Reflect>,
) -> Result<bool, ReflectPartialEqError> {
    for (a, b) in a.zip(b) {
        if !a.reflect_partial_eq(b)? {
            return Ok(false);
        }
    }
    Ok(true)
}
