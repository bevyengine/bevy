use crate::{kind::KindRef, Reflect, Type};
use core::cmp::Ordering;
use thiserror::Error;

impl dyn Reflect {
    /// Returns an ordering between this value and `other` (`None` if they are unordered, like
    /// [`PartialOrd::partial_cmp`]), or an error describing the (possibly nested) value that failed
    /// if they cannot be compared.
    ///
    /// This will use the reflected [`PartialOrd`] implementation, if one is present. If not, it will rely
    /// on a "structural" partial ordering using the reflected type information.
    ///
    /// Values of different types are unordered.
    pub fn reflect_partial_cmp(
        &self,
        other: &dyn Reflect,
    ) -> Result<Option<Ordering>, ReflectPartialCmpError> {
        let ty = self.ty();
        if ty.type_id() != other.ty().type_id() {
            return Ok(None);
        }
        if let Some(partial_cmp) = ty.vtable().partial_cmp {
            // SAFETY: `Reflect::ty` accurately describes both values, which have the same type
            return Ok(unsafe { partial_cmp(self.ptr(), other.ptr()) });
        }
        match (self.kind_ref(), other.kind_ref()) {
            (KindRef::Struct(a), KindRef::Struct(b)) => {
                lexicographic_cmp(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
            }
            (KindRef::Enum(a), KindRef::Enum(b)) => match a.variant().cmp(&b.variant()) {
                Ordering::Equal => {
                    lexicographic_cmp(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
                }
                ordering => Ok(Some(ordering)),
            },
            (KindRef::List(a), KindRef::List(b)) => lexicographic_cmp(a.iter(), b.iter()),
            (KindRef::Array(a), KindRef::Array(b)) => lexicographic_cmp(a.iter(), b.iter()),
            _ => Err(ReflectPartialCmpError { ty }),
        }
    }
}

/// An error returned by
/// [`<dyn Reflect>::reflect_partial_cmp`](trait.Reflect.html#method.reflect_partial_cmp) when part
/// of a value has no reflected [`PartialOrd`] implementation and cannot be ordered structurally.
#[derive(Error, Debug, Clone, Copy)]
#[error(
    "cannot order `{}`: it has no reflected `PartialOrd` implementation and cannot be ordered structurally",
    .ty.type_path().full()
)]
pub struct ReflectPartialCmpError {
    /// The type of the (possibly nested) value that could not be ordered.
    pub ty: &'static Type,
}

/// Compares the values in order, using the first non-equal ordering. If one sequence is a prefix of
/// the other, the shorter sequence is ordered first. If a pair of values is unordered, the sequences
/// are unordered.
fn lexicographic_cmp<'a>(
    mut a: impl Iterator<Item = &'a dyn Reflect>,
    mut b: impl Iterator<Item = &'a dyn Reflect>,
) -> Result<Option<Ordering>, ReflectPartialCmpError> {
    loop {
        match (a.next(), b.next()) {
            (Some(a), Some(b)) => match a.reflect_partial_cmp(b)? {
                Some(Ordering::Equal) => {}
                ordering => return Ok(ordering),
            },
            (Some(_), None) => return Ok(Some(Ordering::Greater)),
            (None, Some(_)) => return Ok(Some(Ordering::Less)),
            (None, None) => return Ok(Some(Ordering::Equal)),
        }
    }
}
