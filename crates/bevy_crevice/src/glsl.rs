//! Defines traits and types for generating GLSL code from Rust definitions.

pub use bevy_crevice_derive::GlslStruct;
use std::marker::PhantomData;

/// Type-level linked list of array dimensions
pub struct Dimension<A, const N: usize> {
    _marker: PhantomData<A>,
}

/// Type-level linked list terminator for array dimensions.
pub struct DimensionNil;

/// Trait for type-level array dimensions. Probably shouldn't be implemented outside this crate.
pub unsafe trait DimensionList {
    /// Write dimensions in square brackets to a string, list tail to list head.
    fn push_to_string(s: &mut String);
}

unsafe impl DimensionList for DimensionNil {
    fn push_to_string(_: &mut String) {}
}

unsafe impl<A: DimensionList, const N: usize> DimensionList for Dimension<A, N> {
    fn push_to_string(s: &mut String) {
        use std::fmt::Write;
        A::push_to_string(s);
        write!(s, "[{}]", N).unwrap();
    }
}

/// Trait for types that have a GLSL equivalent. Useful for generating GLSL code
/// from Rust structs.
pub unsafe trait Glsl {
    /// The name of this type in GLSL, like `vec2` or `mat4`.
    const NAME: &'static str;
}

/// Trait for types that can be represented as a struct in GLSL.
///
/// This trait should not generally be implemented by hand, but can be derived.
pub unsafe trait GlslStruct: Glsl {
    /// The fields contained in this struct.
    fn enumerate_fields(s: &mut String);

    /// Generates GLSL code that represents this struct and its fields.
    fn glsl_definition() -> String {
        let mut output = String::new();
        output.push_str("struct ");
        output.push_str(Self::NAME);
        output.push_str(" {\n");

        Self::enumerate_fields(&mut output);

        output.push_str("};");
        output
    }
}

/// Trait for types that are expressible as a GLSL type with (possibly zero) array dimensions.
pub unsafe trait GlslArray {
    /// Base type name.
    const NAME: &'static str;
    /// Type-level linked list of array dimensions, ordered outer to inner.
    type ArraySize: DimensionList;
}

unsafe impl<T: Glsl> GlslArray for T {
    const NAME: &'static str = <T as Glsl>::NAME;
    type ArraySize = DimensionNil;
}

unsafe impl Glsl for f32 {
    const NAME: &'static str = "float";
}

unsafe impl Glsl for f64 {
    const NAME: &'static str = "double";
}

unsafe impl Glsl for i32 {
    const NAME: &'static str = "int";
}

unsafe impl Glsl for u32 {
    const NAME: &'static str = "uint";
}

unsafe impl<T: GlslArray, const N: usize> GlslArray for [T; N] {
    const NAME: &'static str = T::NAME;

    type ArraySize = Dimension<T::ArraySize, N>;
}
