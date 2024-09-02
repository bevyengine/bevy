//! Function signature types.
//!
//! Function signatures differ from [`FunctionInfo`] in that they are only concerned
//! about the types and order of the arguments and return type of a function.
//!
//! The names of arguments do not matter,
//! nor does any other information about the function such as its name or other attributes.
//!
//! This makes signatures useful for comparing or hashing functions strictly based on their
//! arguments and return type.

use crate::func::args::ArgInfo;
use crate::func::FunctionInfo;
use crate::Type;
use core::borrow::Borrow;
use core::fmt::{Debug, Formatter};
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

impl<T: Borrow<FunctionInfo>> From<T> for Signature {
    fn from(info: T) -> Self {
        let info = info.borrow();
        Self::new(ArgumentSignature::from(info), *info.return_info().ty())
    }
}

/// The argument-portion of a function signature.
///
/// For example, given a function signature `(a: i32, b: f32) -> u32`,
/// the argument signature would be `(i32, f32)`.
///
/// This can be used as a way to compare or hash functions based on their arguments.
#[derive(Clone, PartialEq, Eq, Hash)]
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

impl FromIterator<Type> for ArgumentSignature {
    fn from_iter<T: IntoIterator<Item = Type>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: Borrow<FunctionInfo>> From<T> for ArgumentSignature {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::TypedFunction;

    #[test]
    fn should_generate_signature_from_function_info() {
        fn add(a: i32, b: f32) -> u32 {
            (a as f32 + b).round() as u32
        }

        let info = add.get_function_info();
        let signature = Signature::from(&info);

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
            format!("{:?}", signature),
            "(&mut alloc::string::String, i32) -> ()"
        );
    }
}
