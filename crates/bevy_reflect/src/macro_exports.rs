use crate::{
    array_debug, enum_debug, list_debug, map_debug, struct_debug, tuple_debug, tuple_struct_debug,
    Reflect, ReflectRef,
};
use std::fmt;

pub use bevy_utils::uuid::generate_composite_uuid;

pub trait DebugSpecialization {
    fn debug_specialization_fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub struct SpecializeDebug<'a, T>(pub &'a T);

// This implementation will take precedence over the subsequent one
// in `reflect_debug` implementations since Rust will check this one
// before autoref coercing to `&SpecializeDebug`.
//
// See https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md.
impl<T: fmt::Debug> DebugSpecialization for SpecializeDebug<'_, T> {
    fn debug_specialization_fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.0, fmt)
    }
}

impl<T: Reflect> DebugSpecialization for &SpecializeDebug<'_, T> {
    fn debug_specialization_fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.reflect_ref() {
            ReflectRef::Struct(x) => struct_debug(x, fmt),
            ReflectRef::TupleStruct(x) => tuple_struct_debug(x, fmt),
            ReflectRef::Tuple(x) => tuple_debug(x, fmt),
            ReflectRef::List(x) => list_debug(x, fmt),
            ReflectRef::Array(x) => array_debug(x, fmt),
            ReflectRef::Map(x) => map_debug(x, fmt),
            ReflectRef::Enum(x) => enum_debug(x, fmt),
            ReflectRef::Value(_) => unreachable!(
                "reflected value types should define their own `Debug` implementations"
            ),
        }
    }
}
