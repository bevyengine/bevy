use crate::func::signature::ArgumentSignature;
use crate::func::FunctionInfoType;
use bevy_utils::HashMap;

/// A helper type for storing either a single function or a mapping of overloaded functions.
#[derive(Clone)]
pub(super) enum FunctionMap<F> {
    Standard(F),
    Overloaded(HashMap<ArgumentSignature, F>),
}

/// Merges the given [`FunctionMap`]s and [`FunctionInfoType`]s into a new [`FunctionMap`] and [`FunctionInfoType`].
///
/// # Panics
///
/// Panics if a [`FunctionMap`]'s corresponding [`FunctionInfoType`] does not match its overload status.
pub(super) fn merge_function_map<F>(
    map_a: FunctionMap<F>,
    info_a: FunctionInfoType,
    map_b: FunctionMap<F>,
    info_b: FunctionInfoType,
) -> (FunctionMap<F>, FunctionInfoType) {
    match (map_a, info_a, map_b, info_b) {
        (
            FunctionMap::Standard(old),
            FunctionInfoType::Standard(info_a),
            FunctionMap::Standard(new),
            FunctionInfoType::Standard(info_b),
        ) => {
            let sig_a = ArgumentSignature::from(&info_a);
            let sig_b = ArgumentSignature::from(&info_b);

            if sig_a == sig_b {
                (
                    FunctionMap::Standard(new),
                    FunctionInfoType::Standard(info_b),
                )
            } else {
                (
                    FunctionMap::Overloaded(HashMap::from([(sig_a, old), (sig_b, new)])),
                    FunctionInfoType::Overloaded(Box::new([info_a, info_b])),
                )
            }
        }
        (
            FunctionMap::Overloaded(old),
            FunctionInfoType::Overloaded(info_a),
            FunctionMap::Standard(new),
            FunctionInfoType::Standard(info_b),
        ) => {
            let sig_b = ArgumentSignature::from(&info_b);
            let mut map = old;
            map.insert(sig_b, new);

            let mut info = Vec::from_iter(info_a);
            info.push(info_b);

            (
                FunctionMap::Overloaded(map),
                FunctionInfoType::Overloaded(info.into_boxed_slice()),
            )
        }
        (
            FunctionMap::Standard(old),
            FunctionInfoType::Standard(info_a),
            FunctionMap::Overloaded(new),
            FunctionInfoType::Overloaded(info_b),
        ) => {
            let sig_a = ArgumentSignature::from(&info_a);
            let mut map = new;
            map.insert(sig_a, old);

            let mut info = vec![info_a];
            info.extend(info_b);

            (
                FunctionMap::Overloaded(map),
                FunctionInfoType::Overloaded(info.into_boxed_slice()),
            )
        }
        (
            FunctionMap::Overloaded(map1),
            FunctionInfoType::Overloaded(info_a),
            FunctionMap::Overloaded(map2),
            FunctionInfoType::Overloaded(info_b),
        ) => {
            let mut map = map1;
            map.extend(map2);

            let mut info = Vec::from_iter(info_a);
            info.extend(info_b);

            (
                FunctionMap::Overloaded(map),
                FunctionInfoType::Overloaded(info.into_boxed_slice()),
            )
        }
        _ => {
            panic!("`FunctionMap` and `FunctionInfoType` mismatch");
        }
    }
}
