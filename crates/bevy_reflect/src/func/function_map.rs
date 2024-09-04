use crate::func::signature::ArgumentSignature;
use crate::func::{ArgList, FunctionError, FunctionInfoType, FunctionOverloadError};
use bevy_utils::HashMap;

/// A helper type for storing a mapping of overloaded functions
/// along with the corresponding [function information].
///
/// [function information]: FunctionInfoType
#[derive(Clone, Debug)]
pub(super) struct FunctionMap<F> {
    pub info: FunctionInfoType,
    pub functions: Vec<F>,
    pub indices: HashMap<ArgumentSignature, usize>,
}

impl<F> FunctionMap<F> {
    /// Get a reference to a function in the map.
    ///
    /// If there is only one function in the map, it will be returned.
    /// Otherwise, the function will be selected based on the arguments provided.
    ///
    /// If no overload matches the provided arguments, an error will be returned.
    pub fn get(&self, args: &ArgList) -> Result<&F, FunctionError> {
        if self.functions.len() == 1 {
            Ok(&self.functions[0])
        } else {
            let signature = ArgumentSignature::from(args);
            self.indices
                .get(&signature)
                .map(|index| &self.functions[*index])
                .ok_or_else(|| FunctionError::NoOverload {
                    expected: self.indices.keys().cloned().collect(),
                    received: signature,
                })
        }
    }

    /// Get a mutable reference to a function in the map.
    ///
    /// If there is only one function in the map, it will be returned.
    /// Otherwise, the function will be selected based on the arguments provided.
    ///
    /// If no overload matches the provided arguments, an error will be returned.
    pub fn get_mut(&mut self, args: &ArgList) -> Result<&mut F, FunctionError> {
        if self.functions.len() == 1 {
            Ok(&mut self.functions[0])
        } else {
            let signature = ArgumentSignature::from(args);
            self.indices
                .get(&signature)
                .map(|index| &mut self.functions[*index])
                .ok_or_else(|| FunctionError::NoOverload {
                    expected: self.indices.keys().cloned().collect(),
                    received: signature,
                })
        }
    }

    /// Merge another [`FunctionMap`] into this one.
    ///
    /// If the other map contains any functions with the same signature as this one,
    /// an error will be returned along with the original, unchanged map.
    pub fn merge(mut self, other: Self) -> Result<Self, (Box<Self>, FunctionOverloadError)> {
        // === Function Map === //
        let mut other_indices = HashMap::new();

        for (sig, index) in other.indices {
            if self.indices.contains_key(&sig) {
                return Err((Box::new(self), FunctionOverloadError { signature: sig }));
            }

            other_indices.insert(sig, self.functions.len() + index);
        }

        // === Function Info === //
        let mut other_infos = Vec::new();

        for info in other.info.into_iter() {
            let sig = ArgumentSignature::from(&info);
            if self.indices.contains_key(&sig) {
                return Err((Box::new(self), FunctionOverloadError { signature: sig }));
            }
            other_infos.push(info);
        }

        // === Update === //
        self.indices.extend(other_indices);
        self.functions.extend(other.functions);
        self.info = match self.info {
            FunctionInfoType::Standard(info) => {
                FunctionInfoType::Overloaded(std::iter::once(info).chain(other_infos).collect())
            }
            FunctionInfoType::Overloaded(infos) => FunctionInfoType::Overloaded(
                IntoIterator::into_iter(infos).chain(other_infos).collect(),
            ),
        };

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::FunctionInfo;
    use crate::Type;

    #[test]
    fn should_merge_function_maps() {
        let map_a = FunctionMap {
            info: FunctionInfoType::Overloaded(Box::new([
                FunctionInfo::anonymous().with_arg::<i8>("arg0"),
                FunctionInfo::anonymous().with_arg::<i16>("arg0"),
                FunctionInfo::anonymous().with_arg::<i32>("arg0"),
            ])),
            functions: vec!['a', 'b', 'c'],
            indices: HashMap::from([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<i32>()]), 2),
            ]),
        };

        let map_b = FunctionMap {
            info: FunctionInfoType::Overloaded(Box::new([
                FunctionInfo::anonymous().with_arg::<u8>("arg0"),
                FunctionInfo::anonymous().with_arg::<u16>("arg0"),
                FunctionInfo::anonymous().with_arg::<u32>("arg0"),
            ])),
            functions: vec!['d', 'e', 'f'],
            indices: HashMap::from([
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<u16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<u32>()]), 2),
            ]),
        };

        let map = map_a.merge(map_b).unwrap();

        assert_eq!(map.functions, vec!['a', 'b', 'c', 'd', 'e', 'f']);
        assert_eq!(
            map.indices,
            HashMap::from([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<i32>()]), 2),
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 3),
                (ArgumentSignature::from_iter([Type::of::<u16>()]), 4),
                (ArgumentSignature::from_iter([Type::of::<u32>()]), 5),
            ])
        );
    }

    #[test]
    fn should_return_error_on_duplicate_signature() {
        let map_a = FunctionMap {
            info: FunctionInfoType::Overloaded(Box::new([
                FunctionInfo::anonymous().with_arg::<i8>("arg0"),
                FunctionInfo::anonymous().with_arg::<i16>("arg0"),
                FunctionInfo::anonymous().with_arg::<i32>("arg0"),
            ])),
            functions: vec!['a', 'b', 'c'],
            indices: HashMap::from([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<i32>()]), 2),
            ]),
        };

        let map_b = FunctionMap {
            info: FunctionInfoType::Overloaded(Box::new([
                FunctionInfo::anonymous().with_arg::<u8>("arg0"),
                FunctionInfo::anonymous().with_arg::<i16>("arg0"),
                FunctionInfo::anonymous().with_arg::<u32>("arg0"),
            ])),
            functions: vec!['d', 'e', 'f'],
            indices: HashMap::from([
                (ArgumentSignature::from_iter([Type::of::<u8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<u32>()]), 2),
            ]),
        };

        let Err((map_a, error)) = map_a.merge(map_b) else {
            panic!("expected an error");
        };
        assert_eq!(
            error,
            FunctionOverloadError {
                signature: ArgumentSignature::from_iter([Type::of::<i16>()])
            }
        );

        // Assert that the original map remains unchanged:
        assert_eq!(map_a.functions, vec!['a', 'b', 'c']);
        assert_eq!(
            map_a.indices,
            HashMap::from([
                (ArgumentSignature::from_iter([Type::of::<i8>()]), 0),
                (ArgumentSignature::from_iter([Type::of::<i16>()]), 1),
                (ArgumentSignature::from_iter([Type::of::<i32>()]), 2),
            ])
        );
    }
}
