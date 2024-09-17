use crate::attributes::{impl_custom_attribute_methods, CustomAttributes};
use crate::type_info::impl_type_methods;
use crate::{MaybeTyped, PartialReflect, Type, TypeInfo, TypePath};
use std::sync::Arc;

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: &'static str,
    type_info: fn() -> Option<&'static TypeInfo>,
    ty: Type,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl NamedField {
    /// Create a new [`NamedField`].
    pub fn new<T: PartialReflect + MaybeTyped + TypePath>(name: &'static str) -> Self {
        Self {
            name,
            type_info: T::maybe_type_info,
            ty: Type::of::<T>(),
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this field.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this field.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
    }

    /// The name of the field.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The [`TypeInfo`] of the field.
    ///
    ///
    /// Returns `None` if the field does not contain static type information,
    /// such as for dynamic types.
    pub fn type_info(&self) -> Option<&'static TypeInfo> {
        (self.type_info)()
    }

    impl_type_methods!(ty);

    /// The docstring of this field, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "field");
}

/// The unnamed field of a reflected tuple or tuple struct.
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    type_info: fn() -> Option<&'static TypeInfo>,
    ty: Type,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl UnnamedField {
    pub fn new<T: PartialReflect + MaybeTyped + TypePath>(index: usize) -> Self {
        Self {
            index,
            type_info: T::maybe_type_info,
            ty: Type::of::<T>(),
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this field.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this field.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
    }

    /// Returns the index of the field.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The [`TypeInfo`] of the field.
    ///
    ///
    /// Returns `None` if the field does not contain static type information,
    /// such as for dynamic types.
    pub fn type_info(&self) -> Option<&'static TypeInfo> {
        (self.type_info)()
    }

    impl_type_methods!(ty);

    /// The docstring of this field, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "field");
}
