use crate::{KindRef, Reflect, Type};
use bevy_platform::hash::FixedHasher;
use core::hash::{BuildHasher, Hash, Hasher};
use thiserror::Error;

impl dyn Reflect {
    /// Returns a hash of this value, or an error describing the (possibly nested) value that failed
    /// if any part of it cannot be hashed.
    ///
    /// See [`<dyn Reflect>::reflect_hash_into`](trait.Reflect.html#method.reflect_hash_into).
    pub fn reflect_hash(&self) -> Result<u64, ReflectHashError> {
        let mut hasher = FixedHasher.build_hasher();
        self.reflect_hash_into(&mut hasher)?;
        Ok(hasher.finish())
    }

    /// Feeds this value into `hasher`. Returns an error describing the (possibly nested) value that
    /// failed if any part of it cannot be hashed, in which case `hasher` may have been partially
    /// written to.
    ///
    /// This will use the reflected [`Hash`] implementation, if one is present. If not, it will rely
    /// on a "structural" hash using the reflected type information, which hashes the same values
    /// that the structural
    /// [`<dyn Reflect>::reflect_partial_eq`](trait.Reflect.html#method.reflect_partial_eq)
    /// compares. Values with a reflected [`PartialEq`] implementation but no reflected [`Hash`]
    /// implementation cannot be hashed, because a structural hash could disagree with their
    /// equality.
    ///
    /// Values of different types produce different hashes, even if their contents are the same.
    pub fn reflect_hash_into(&self, mut hasher: &mut dyn Hasher) -> Result<(), ReflectHashError> {
        let ty = self.ty();
        let error = |reason| ReflectHashError { ty, reason };
        ty.type_id().hash(&mut hasher);
        if let Some(hash) = ty.vtable().hash {
            // SAFETY: `Reflect::ty` accurately describes this value
            unsafe { hash(self.ptr(), hasher) };
            return Ok(());
        }
        // Values that are equal according to a custom `PartialEq` (ex: case-insensitive strings) could
        // have different structural hashes
        if ty.vtable().partial_eq.is_some() {
            return Err(error(ReflectHashErrorReason::PartialEqWithoutHash));
        }
        match self.kind_ref() {
            // Values of the same struct type have the same number of fields, so it isn't hashed.
            // Ignored fields are not hashed, because they are not compared.
            KindRef::Struct(value) => {
                for (_, field) in value.iter() {
                    field.reflect_hash_into(hasher)?;
                }
            }
            KindRef::Enum(value) => {
                hasher.write_usize(value.variant());
                for (_, field) in value.iter() {
                    field.reflect_hash_into(hasher)?;
                }
            }
            KindRef::List(value) => {
                hasher.write_usize(value.len());
                for item in value.iter() {
                    item.reflect_hash_into(hasher)?;
                }
            }
            KindRef::Array(value) => {
                // Values of the same array type have the same length, so it isn't hashed
                for item in value.iter() {
                    item.reflect_hash_into(hasher)?;
                }
            }
            KindRef::Map(_) | KindRef::Set(_) | KindRef::Primitive(_) => {
                // Primitives, maps, and sets are either handled above by the custom hash impl, or
                // they fail to hash
                return Err(error(ReflectHashErrorReason::NotHashable));
            }
        }
        Ok(())
    }
}

/// An error returned by [`<dyn Reflect>::reflect_hash`](trait.Reflect.html#method.reflect_hash)
/// when part of a value cannot be hashed.
#[derive(Error, Debug, Clone, Copy)]
#[error("cannot hash `{}`: {reason}", .ty.type_path().full())]
pub struct ReflectHashError {
    /// The type of the (possibly nested) value that could not be hashed.
    pub ty: &'static Type,
    /// Why the value could not be hashed.
    pub reason: ReflectHashErrorReason,
}

/// Why a value could not be hashed. See [`ReflectHashError`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectHashErrorReason {
    /// The type has no reflected [`Hash`] implementation, and is a map, set, or
    /// [`Primitive`](crate::Primitive), which cannot be hashed structurally.
    #[error("it has no reflected `Hash` implementation and cannot be hashed structurally")]
    NotHashable,
    /// The type has a reflected [`PartialEq`] implementation but no reflected [`Hash`]
    /// implementation. A structural hash could disagree with its equality.
    #[error(
        "it has a reflected `PartialEq` implementation but no reflected `Hash` implementation"
    )]
    PartialEqWithoutHash,
}
