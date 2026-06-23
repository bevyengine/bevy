use crate::{
    generics::impl_generic_info_methods, ty::impl_type_methods, Generics, Reflect, Type, TypePath,
};

/// A container for compile-time info related to reflection-opaque types, including primitives.
///
/// This typically represents a type which cannot be broken down any further. This is often
/// due to technical reasons (or by definition), but it can also be a purposeful choice.
///
/// For example, [`i32`] cannot be broken down any further, so it is represented by an [`OpaqueInfo`].
/// And while [`String`] itself is a struct, its fields are private, so we don't really treat
/// it _as_ a struct. It therefore makes more sense to represent it as an [`OpaqueInfo`].
///
/// [`String`]: alloc::string::String
#[derive(Debug, Clone)]
pub struct OpaqueInfo {
    ty: Type,
    generics: Generics,
    #[cfg(feature = "reflect_documentation")]
    docs: Option<&'static str>,
}

impl OpaqueInfo {
    /// Creates a new [`OpaqueInfo`].
    pub fn new<T: Reflect + TypePath + ?Sized>() -> Self {
        Self {
            ty: Type::of::<T>(),
            generics: Generics::new(),
            #[cfg(feature = "reflect_documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this type.
    #[cfg(feature = "reflect_documentation")]
    pub fn with_docs(self, doc: Option<&'static str>) -> Self {
        Self { docs: doc, ..self }
    }

    impl_type_methods!(ty);

    /// The docstring of this dynamic type, if any.
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}
