use crate::func::args::ArgCount;
use crate::func::signature::{ArgListSignature, ArgumentSignature};
use crate::func::{ArgList, FunctionError, FunctionInfo, FunctionOverloadError};
use alloc::{borrow::Cow, vec, vec::Vec};
use bevy_platform::collections::HashMap;
use core::fmt::{Debug, Formatter};

/// An internal structure for storing a function and its corresponding [function information].
///
/// This is used to facilitate the sharing of functionality between [`DynamicFunction`]
/// and [`DynamicFunctionMut`].
///
/// [function information]: FunctionInfo
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Clone)]
pub(super) struct DynamicFunctionInternal<F> {
    functions: Vec<F>,
    info: FunctionInfo,
    arg_map: HashMap<ArgumentSignature, usize>,
}

impl<F> DynamicFunctionInternal<F> {
    /// Create a new instance of [`DynamicFunctionInternal`] with the given function
    /// and its corresponding information.
    pub fn new(func: F, info: FunctionInfo) -> Self {
        let arg_map = info
            .signatures()
            .iter()
            .map(|sig| (ArgumentSignature::from(sig), 0))
            .collect();

        Self {
            functions: vec![func],
            info,
            arg_map,
        }
    }
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.info = self.info.with_name(Some(name.into()));
        self
    }

    /// The name of the function.
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.info.name()
    }

    /// Returns `true` if the function is overloaded.
    pub fn is_overloaded(&self) -> bool {
        self.info.is_overloaded()
    }

    /// Get an immutable reference to the function.
    ///
    /// If the function is not overloaded, it will always be returned regardless of the arguments.
    /// Otherwise, the function will be selected based on the arguments provided.
    ///
    /// If no overload matches the provided arguments, returns [`FunctionError::NoOverload`].
    pub fn get(&self, args: &ArgList) -> Result<&F, FunctionError> {
        if !self.info.is_overloaded() {
            return Ok(&self.functions[0]);
        }

        let signature = ArgListSignature::from(args);
        self.arg_map
            .get(&signature)
            .map(|index| &self.functions[*index])
            .ok_or_else(|| FunctionError::NoOverload {
                expected: self.arg_map.keys().cloned().collect(),
                received: ArgumentSignature::from(args),
            })
    }

    /// Get a mutable reference to the function.
    ///
    /// If the function is not overloaded, it will always be returned regardless of the arguments.
    /// Otherwise, the function will be selected based on the arguments provided.
    ///
    /// If no overload matches the provided arguments, returns [`FunctionError::NoOverload`].
    pub fn get_mut(&mut self, args: &ArgList) -> Result<&mut F, FunctionError> {
        if !self.info.is_overloaded() {
            return Ok(&mut self.functions[0]);
        }

        let signature = ArgListSignature::from(args);
        self.arg_map
            .get(&signature)
            .map(|index| &mut self.functions[*index])
            .ok_or_else(|| FunctionError::NoOverload {
                expected: self.arg_map.keys().cloned().collect(),
                received: ArgumentSignature::from(args),
            })
    }

    /// Returns the function information contained in the map.
    #[inline]
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }

    /// Returns the number of arguments the function expects.
    ///
    /// For overloaded functions that can have a variable number of arguments,
    /// this will contain the full set of counts for all signatures.
    pub fn arg_count(&self) -> ArgCount {
        self.info.arg_count()
    }

    /// Helper method for validating that a given set of arguments are _potentially_ valid for this function.
    ///
    /// Currently, this validates:
    /// - The number of arguments is within the expected range
    pub fn validate_args(&self, args: &ArgList) -> Result<(), FunctionError> {
        let expected_arg_count = self.arg_count();
        let received_arg_count = args.len();

        if !expected_arg_count.contains(received_arg_count) {
            Err(FunctionError::ArgCountMismatch {
                expected: expected_arg_count,
                received: received_arg_count,
            })
        } else {
            Ok(())
        }
    }

    /// Merge another [`DynamicFunctionInternal`] into this one.
    ///
    /// If `other` contains any functions with the same signature as this one,
    /// an error will be returned along with the original, unchanged instance.
    ///
    /// Therefore, this method should always return an overloaded function if the merge is successful.
    ///
    /// Additionally, if the merge succeeds, it should be guaranteed that the order
    /// of the functions in the map will be preserved.
    /// For example, merging `[func_a, func_b]` (self) with `[func_c, func_d]` (other) should result in
    /// `[func_a, func_b, func_c, func_d]`.
    /// And merging `[func_c, func_d]` (self) with `[func_a, func_b]` (other) should result in
    /// `[func_c, func_d, func_a, func_b]`.
    pub fn merge(&mut self, mut other: Self) -> Result<(), FunctionOverloadError> {
        // Keep a separate map of the new indices to avoid mutating the existing one
        // until we can be sure the merge will be successful.
        let mut new_signatures = <HashMap<_, _>>::default();

        for (sig, index) in other.arg_map {
            if self.arg_map.contains_key(&sig) {
                return Err(FunctionOverloadError::DuplicateSignature(sig));
            }

            new_signatures.insert(sig, self.functions.len() + index);
        }

        self.arg_map.reserve(new_signatures.len());
        for (sig, index) in new_signatures {
            self.arg_map.insert(sig, index);
        }

        self.functions.append(&mut other.functions);
        self.info.extend_unchecked(other.info);

        Ok(())
    }

    /// Maps the internally stored function(s) from type `F` to type `G`.
    pub fn map_functions<G>(self, f: fn(F) -> G) -> DynamicFunctionInternal<G> {
        DynamicFunctionInternal {
            functions: self.functions.into_iter().map(f).collect(),
            info: self.info,
            arg_map: self.arg_map,
        }
    }
}

impl<F> Debug for DynamicFunctionInternal<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.info
            .pretty_printer()
            .include_fn_token()
            .include_name()
            .fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::{FunctionInfo, SignatureInfo};
    use crate::Type;

    #[test]
    fn should_merge_single_into_single() {
        let mut func_a = DynamicFunctionInternal::new(
            'a',
            FunctionInfo::new(SignatureInfo::anonymous().with_arg::<i8>("arg0")),
        );

        let func_b = DynamicFunctionInternal::new(
            'b',
            FunctionInfo::new(SignatureInfo::anonymous().with_arg::<u8>("arg0")),
        );

        func_a.merge(func_b).unwrap();

        assert_eq!(func_a.functions, vec!['a', 'b']);
        assert_eq!(func_a.info.signatures().len(), 2);
        assert_eq!(
            func_a.arg_map,
            HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 1),
            ])
        );
    }

    #[test]
    fn should_merge_single_into_overloaded() {
        let mut func_a = DynamicFunctionInternal::new(
            'a',
            FunctionInfo::new(SignatureInfo::anonymous().with_arg::<i8>("arg0")),
        );

        let func_b = DynamicFunctionInternal {
            functions: vec!['b', 'c'],
            info: FunctionInfo::new(SignatureInfo::anonymous().with_arg::<u8>("arg0"))
                .with_overload(SignatureInfo::anonymous().with_arg::<u16>("arg0"))
                .unwrap(),
            arg_map: HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<u16>()]), 1),
            ]),
        };

        func_a.merge(func_b).unwrap();

        assert_eq!(func_a.functions, vec!['a', 'b', 'c']);
        assert_eq!(func_a.info.signatures().len(), 3);
        assert_eq!(
            func_a.arg_map,
            HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<u16>()]), 2),
            ])
        );
    }

    #[test]
    fn should_merge_overload_into_single() {
        let mut func_a = DynamicFunctionInternal {
            functions: vec!['a', 'b'],
            info: FunctionInfo::new(SignatureInfo::anonymous().with_arg::<i8>("arg0"))
                .with_overload(SignatureInfo::anonymous().with_arg::<i16>("arg0"))
                .unwrap(),
            arg_map: HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
            ]),
        };

        let func_b = DynamicFunctionInternal::new(
            'c',
            FunctionInfo::new(SignatureInfo::anonymous().with_arg::<u8>("arg0")),
        );

        func_a.merge(func_b).unwrap();

        assert_eq!(func_a.functions, vec!['a', 'b', 'c']);
        assert_eq!(func_a.info.signatures().len(), 3);
        assert_eq!(
            func_a.arg_map,
            HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 2),
            ])
        );
    }

    #[test]
    fn should_merge_overloaded_into_overloaded() {
        let mut func_a = DynamicFunctionInternal {
            functions: vec!['a', 'b'],
            info: FunctionInfo::new(SignatureInfo::anonymous().with_arg::<i8>("arg0"))
                .with_overload(SignatureInfo::anonymous().with_arg::<i16>("arg0"))
                .unwrap(),
            arg_map: HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
            ]),
        };

        let func_b = DynamicFunctionInternal {
            functions: vec!['c', 'd'],
            info: FunctionInfo::new(SignatureInfo::anonymous().with_arg::<u8>("arg0"))
                .with_overload(SignatureInfo::anonymous().with_arg::<u16>("arg0"))
                .unwrap(),
            arg_map: HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<u16>()]), 1),
            ]),
        };

        func_a.merge(func_b).unwrap();

        assert_eq!(func_a.functions, vec!['a', 'b', 'c', 'd']);
        assert_eq!(func_a.info.signatures().len(), 4);
        assert_eq!(
            func_a.arg_map,
            HashMap::from_iter([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 2),
                (ArgumentSignature::from_iter([Type::of::<u16>()]), 3),
            ])
        );
    }

    #[test]
    fn should_return_error_on_duplicate_signature() {
        let mut func_a = DynamicFunctionInternal::new(
            'a',
            FunctionInfo::new(
                SignatureInfo::anonymous()
                    .with_arg::<i8>("arg0")
                    .with_arg::<i16>("arg1"),
            ),
        );

        let func_b = DynamicFunctionInternal {
            functions: vec!['b', 'c'],
            info: FunctionInfo::new(
                SignatureInfo::anonymous()
                    .with_arg::<u8>("arg0")
                    .with_arg::<u16>("arg1"),
            )
            .with_overload(
                SignatureInfo::anonymous()
                    .with_arg::<i8>("arg0")
                    .with_arg::<i16>("arg1"),
            )
            .unwrap(),
            arg_map: HashMap::from_iter([
                (
                    ArgumentSignature::from_iter([Type::of::<u8>(), Type::of::<u16>()]),
                    0,
                ),
                (
                    ArgumentSignature::from_iter([Type::of::<i8>(), Type::of::<i16>()]),
                    1,
                ),
            ]),
        };

        let FunctionOverloadError::DuplicateSignature(duplicate) =
            func_a.merge(func_b).unwrap_err()
        else {
            panic!("Expected `FunctionOverloadError::DuplicateSignature`");
        };

        assert_eq!(
            duplicate,
            ArgumentSignature::from_iter([Type::of::<i8>(), Type::of::<i16>()])
        );

        // Assert the original remains unchanged:
        assert!(!func_a.is_overloaded());
        assert_eq!(func_a.functions, vec!['a']);
        assert_eq!(func_a.info.signatures().len(), 1);
        assert_eq!(
            func_a.arg_map,
            HashMap::from_iter([(
                ArgumentSignature::from_iter([Type::of::<i8>(), Type::of::<i16>()]),
                0
            ),])
        );
    }
}
