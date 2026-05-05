//! Function signature types.
//!
//! Function signatures differ from [`FunctionInfo`] and [`SignatureInfo`] in that they
//! are only concerned about the types and order of the arguments and return type of a function.
//!
//! The names of arguments do not matter,
//! nor does any other information about the function such as its name or other attributes.
//!
//! This makes signatures useful for comparing or hashing functions strictly based on their
//! arguments and return type.
//!
//! [`FunctionInfo`]: crate::func::info::FunctionInfo

use crate::func::args::ArgInfo;
use crate::func::{ArgList, SignatureInfo};
use crate::Type;
use alloc::boxed::Box;
use bevy_platform::collections::Equivalent;
use core::borrow::Borrow;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::ops::{Deref, DerefMut};

/// The signature of a function.
///
/// This can be used as a way to compare or hash functions based on their arguments and return type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    args: ArgumentSignature,
    ret: Type,
}

impl Signature {
    /// Create a new function signature with the given argument signature and return type.
    pub fn new(args: ArgumentSignature, ret: Type) -> Self {
        Self { args, ret }
    }

    /// Get the argument signature of the function.
    pub fn args(&self) -> &ArgumentSignature {
        &self.args
    }

    /// Get the return type of the function.
    pub fn return_type(&self) -> &Type {
        &self.ret
    }
}

impl Debug for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?} -> {:?}", self.args, self.ret)
    }
}

impl<T: Borrow<SignatureInfo>> From<T> for Signature {
    fn from(info: T) -> Self {
        let info = info.borrow();
        Self::new(ArgumentSignature::from(info), *info.return_info().ty())
    }
}

/// A wrapper around a borrowed [`ArgList`] that can be used as an
/// [equivalent] of an [`ArgumentSignature`].
///
/// [equivalent]: Equivalent
pub(super) struct ArgListSignature<'a, 'b>(&'a ArgList<'b>);

impl Equivalent<ArgumentSignature> for ArgListSignature<'_, '_> {
    fn equivalent(&self, key: &ArgumentSignature) -> bool {
        self.len() == key.len() && self.iter().eq(key.iter())
    }
}

impl<'a, 'b> ArgListSignature<'a, 'b> {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Type> {
        self.0.iter().map(|arg| {
            arg.value()
                .get_represented_type_info()
                .unwrap_or_else(|| {
                    panic!("no `TypeInfo` found for argument: {:?}", arg);
                })
                .ty()
        })
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Eq for ArgListSignature<'_, '_> {}

impl PartialEq for ArgListSignature<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl Hash for ArgListSignature<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.iter().for_each(|arg| {
            arg.value()
                .get_represented_type_info()
                .unwrap_or_else(|| {
                    panic!("no `TypeInfo` found for argument: {:?}", arg);
                })
                .ty()
                .hash(state);
        });
    }
}

impl<'a, 'b> From<&'a ArgList<'b>> for ArgListSignature<'a, 'b> {
    fn from(args: &'a ArgList<'b>) -> Self {
        Self(args)
    }
}

/// The argument-portion of a function signature.
///
/// For example, given a function signature `(a: i32, b: f32) -> u32`,
/// the argument signature would be `(i32, f32)`.
///
/// This can be used as a way to compare or hash functions based on their arguments.
#[derive(Clone)]
pub struct ArgumentSignature(Box<[Type]>);

impl Debug for ArgumentSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut tuple = f.debug_tuple("");
        for ty in self.0.iter() {
            tuple.field(ty);
        }
        tuple.finish()
    }
}

impl Deref for ArgumentSignature {
    type Target = [Type];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ArgumentSignature {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Eq for ArgumentSignature {}

impl PartialEq for ArgumentSignature {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().eq(other.0.iter())
    }
}

impl Hash for ArgumentSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.iter().for_each(|ty| ty.hash(state));
    }
}

impl FromIterator<Type> for ArgumentSignature {
    fn from_iter<T: IntoIterator<Item = Type>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: Borrow<SignatureInfo>> From<T> for ArgumentSignature {
    fn from(info: T) -> Self {
        Self(
            info.borrow()
                .args()
                .iter()
                .map(ArgInfo::ty)
                .copied()
                .collect(),
        )
    }
}

impl From<&ArgList<'_>> for ArgumentSignature {
    fn from(args: &ArgList) -> Self {
        Self(
            args.iter()
                .map(|arg| {
                    arg.value()
                        .get_represented_type_info()
                        .unwrap_or_else(|| {
                            panic!("no `TypeInfo` found for argument: {:?}", arg);
                        })
                        .ty()
                })
                .copied()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::TypedFunction;
    use alloc::{format, string::String, vec};

    #[test]
    fn should_generate_signature_from_function_info() {
        fn add(a: i32, b: f32) -> u32 {
            (a as f32 + b).round() as u32
        }

        let info = add.get_function_info();
        let signature = Signature::from(info.base());

        assert_eq!(signature.args().0.len(), 2);
        assert_eq!(signature.args().0[0], Type::of::<i32>());
        assert_eq!(signature.args().0[1], Type::of::<f32>());
        assert_eq!(*signature.return_type(), Type::of::<u32>());
    }

    #[test]
    fn should_debug_signature() {
        let signature = Signature::new(
            ArgumentSignature::from_iter(vec![Type::of::<&mut String>(), Type::of::<i32>()]),
            Type::of::<()>(),
        );

        assert_eq!(
            format!("{signature:?}"),
            "(&mut alloc::string::String, i32) -> ()"
        );
    }
}
