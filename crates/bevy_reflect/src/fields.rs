use crate::attributes::{impl_custom_attribute_methods, CustomAttributes};
use crate::{MaybeTyped, PartialReflect, TypeInfo, TypePath, TypePathTable};
use alloc::borrow::Cow;
use core::fmt::{Display, Formatter};
use std::any::{Any, TypeId};
use std::sync::Arc;

/// A general-purpose field identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldId {
    /// A named field.
    ///
    /// This includes fields of structs and enum struct variants.
    Named(Cow<'static, str>),
    /// An unnamed field.
    ///
    /// This includes fields of tuples and tuple struct variants.
    Unnamed(usize),
}

impl Display for FieldId {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            FieldId::Named(name) => write!(f, "{}", name),
            FieldId::Unnamed(index) => write!(f, "{}", index),
        }
    }
}

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: &'static str,
    type_info: fn() -> Option<&'static TypeInfo>,
    type_path: TypePathTable,
    type_id: TypeId,
    readonly: bool,
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
            type_path: TypePathTable::of::<T>(),
            type_id: TypeId::of::<T>(),
            readonly: false,
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

    /// Set the readonly status of this field.
    pub fn with_readonly(self, readonly: bool) -> Self {
        Self { readonly, ..self }
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

    /// A representation of the type path of the field.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path
    }

    /// The [stable, full type path] of the field.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.type_path_table().path()
    }

    /// The [`TypeId`] of the field.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the field type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The docstring of this field, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    /// Returns `true` if the field is readonly.
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    impl_custom_attribute_methods!(self.custom_attributes, "field");
}

/// The unnamed field of a reflected tuple or tuple struct.
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    type_info: fn() -> Option<&'static TypeInfo>,
    type_path: TypePathTable,
    type_id: TypeId,
    readonly: bool,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl UnnamedField {
    pub fn new<T: PartialReflect + MaybeTyped + TypePath>(index: usize) -> Self {
        Self {
            index,
            type_info: T::maybe_type_info,
            type_path: TypePathTable::of::<T>(),
            type_id: TypeId::of::<T>(),
            readonly: false,
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

    /// Set the readonly status of this field.
    pub fn with_readonly(self, readonly: bool) -> Self {
        Self { readonly, ..self }
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

    /// A representation of the type path of the field.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path
    }

    /// The [stable, full type path] of the field.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.type_path_table().path()
    }

    /// The [`TypeId`] of the field.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the field type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The docstring of this field, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    /// Returns `true` if the field is readonly.
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    impl_custom_attribute_methods!(self.custom_attributes, "field");
}
