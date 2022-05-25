use crate::Reflect;
use std::any::{Any, TypeId};
use std::borrow::Cow;

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: Cow<'static, str>,
    type_name: &'static str,
    type_id: TypeId,
}

impl NamedField {
    /// Create a new [`NamedField`].
    pub fn new<T: Reflect>(name: &str) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Create a new [`NamedField`] using a static string.
    ///
    /// This helps save an allocation when the string has a static lifetime, such
    /// as when using [`std::any::type_name`].
    pub fn static_new<T: Reflect>(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// The name of the field.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// The [name] of the underlying type of the field.
    ///
    /// [name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        &self.type_name
    }

    /// The [`TypeId`] of the underlying type of the field.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the underlying type of the field.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// The unnamed field of a reflected tuple or tuple struct.
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    type_name: &'static str,
    type_id: TypeId,
}

impl UnnamedField {
    pub fn new<T: Reflect>(index: usize) -> Self {
        Self {
            index,
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Returns the index of the field.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The [name] of the underlying type of the field.
    ///
    /// [name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        &self.type_name
    }

    /// The [`TypeId`] of the underlying type of the field.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the underlying type of the field.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}
