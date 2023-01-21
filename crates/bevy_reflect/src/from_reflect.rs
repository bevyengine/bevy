use crate::{FromType, Reflect, ReflectKind, TypeInfo};
use std::borrow::Cow;
use thiserror::Error;

/// A trait for types which can be constructed from a reflected type.
///
/// This trait can be derived on types which implement [`Reflect`]. Some complex
/// types (such as `Vec<T>`) may only be reflected if their element types
/// implement this trait.
///
/// For structs and tuple structs, fields marked with the `#[reflect(ignore)]`
/// attribute will be constructed using the `Default` implementation of the
/// field type, rather than the corresponding field value (if any) of the
/// reflected value.
pub trait FromReflect: Reflect + Sized {
    /// Constructs a concrete instance of `Self` from a reflected value.
    fn from_reflect(reflect: &dyn Reflect) -> Result<Self, FromReflectError>;

    /// Attempts to downcast the given value to `Self` using,
    /// constructing the value using [`from_reflect`] if that fails.
    ///
    /// This method is more efficient than using [`from_reflect`] for cases where
    /// the given value is likely a boxed instance of `Self` (i.e. `Box<Self>`)
    /// rather than a boxed dynamic type (e.g. [`DynamicStruct`], [`DynamicList`], etc.).
    ///
    /// [`from_reflect`]: Self::from_reflect
    /// [`DynamicStruct`]: crate::DynamicStruct
    /// [`DynamicList`]: crate::DynamicList
    fn take_from_reflect(
        reflect: Box<dyn Reflect>,
    ) -> Result<Self, (Box<dyn Reflect>, FromReflectError)> {
        reflect
            .take::<Self>()
            .or_else(|value| Self::from_reflect(value.as_ref()).map_err(|err| (value, err)))
    }
}

/// Type data that represents the [`FromReflect`] trait and allows it to be used dynamically.
///
/// `FromReflect` allows dynamic types (e.g. [`DynamicStruct`], [`DynamicEnum`], etc.) to be converted
/// to their full, concrete types. This is most important when it comes to deserialization where it isn't
/// guaranteed that every field exists when trying to construct the final output.
///
/// However, to do this, you normally need to specify the exact concrete type:
///
/// ```
/// # use bevy_reflect::{DynamicTupleStruct, FromReflect, Reflect};
/// #[derive(Reflect, FromReflect, PartialEq, Eq, Debug)]
/// struct Foo(#[reflect(default = "default_value")] usize);
///
/// fn default_value() -> usize { 123 }
///
/// let reflected = DynamicTupleStruct::default();
///
/// let concrete: Foo = <Foo as FromReflect>::from_reflect(&reflected).unwrap();
///
/// assert_eq!(Foo(123), concrete);
/// ```
///
/// In a dynamic context where the type might not be known at compile-time, this is nearly impossible to do.
/// That is why this type data struct exists— it allows us to construct the full type without knowing
/// what the actual type is.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{DynamicTupleStruct, FromReflect, Reflect, ReflectFromReflect, TypeRegistry};
/// # #[derive(Reflect, FromReflect, PartialEq, Eq, Debug)]
/// # #[reflect(FromReflect)]
/// # struct Foo(#[reflect(default = "default_value")] usize);
/// # fn default_value() -> usize { 123 }
/// # let mut registry = TypeRegistry::new();
/// # registry.register::<Foo>();
///
/// let mut reflected = DynamicTupleStruct::default();
/// reflected.set_name(std::any::type_name::<Foo>().to_string());
///
/// let registration = registry.get_with_name(reflected.type_name()).unwrap();
/// let rfr = registration.data::<ReflectFromReflect>().unwrap();
///
/// let concrete: Box<dyn Reflect> = rfr.from_reflect(&reflected).unwrap();
///
/// assert_eq!(Foo(123), concrete.take::<Foo>().unwrap());
/// ```
///
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicEnum`]: crate::DynamicEnum
#[derive(Clone)]
pub struct ReflectFromReflect {
    from_reflect: fn(&dyn Reflect) -> Result<Box<dyn Reflect>, FromReflectError>,
}

impl ReflectFromReflect {
    /// Perform a [`FromReflect::from_reflect`] conversion on the given reflection object.
    ///
    /// This will convert the object to a concrete type if it wasn't already, and return
    /// the value as `Box<dyn Reflect>`.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_reflect<'a>(
        &'a self,
        reflect_value: &'a dyn Reflect,
    ) -> Result<Box<dyn Reflect>, FromReflectError> {
        (self.from_reflect)(reflect_value)
    }
}

impl<T: FromReflect> FromType<T> for ReflectFromReflect {
    fn from_type() -> Self {
        Self {
            from_reflect: |reflect_value| {
                T::from_reflect(reflect_value).map(|value| Box::new(value) as Box<dyn Reflect>)
            },
        }
    }
}

/// An Error for failed conversion of reflected type to original type in [`FromReflect::from_reflect`].
///
/// In the error message, the kind of the source type may have a prefix "(Dynamic)" indicating that the
/// source is dynamic, i.e., [`DynamicStruct`], [`DynamicList`], etc.
///
/// Within variants `NamedFieldError`, `UnnamedFieldError`, `IndexError`, `VariantError`, `KeyError` and
/// `ValueError`; [`Error::source`] must be used to trace the underlying error.
///
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicList`]: crate::DynamicList
/// [`Error::source`]: std::error::Error::source
#[derive(Error, Debug)]
pub enum FromReflectError {
    /// The source and target types are of different types or [kinds](ReflectKind).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` of kind {} due to mismatched types or kinds", 
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), self.display_to_kind())]
    InvalidType {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,
    },

    /// The source and target types have different lengths.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Array`](crate::Array).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to source type having length of {} and target type having length of {}",
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .from_len, .to_len)]
    InvalidLength {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Length of the source type.
        from_len: usize,

        /// Length of the target type.
        to_len: usize,
    },

    /// The source type did not have a field with name given by the parameter `field`.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Struct`](crate::Struct) and
    /// [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to a missing field `{}`", 
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .field)]
    MissingNamedField {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Name of missing field in source type.
        field: &'static str,
    },

    /// The source type did not have a field at index given by the parameter `index`.
    ///
    /// This error is given by types of [kind](ReflectKind) [`TupleStruct`](crate::TupleStruct) and
    /// [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to a missing field at index {}", 
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .index)]
    MissingUnnamedField {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Index of missing field in source type.
        index: usize,
    },

    /// The source type did not have a value at index given by the parameter `index`.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Tuple`](crate::Tuple).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to a missing value at index {}",
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .index)]
    MissingIndex {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Index of missing value in source type.
        index: usize,
    },

    /// The target type did not have a variant with name given by the parameter `variant`.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to a missing variant `{}`",
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .variant)]
    MissingVariant {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Name of missing variant in target type.
        variant: Cow<'static, str>,
    },

    /// An error has occurred in conversion of a field with name given by the parameter `field`.
    ///
    /// Use [`Error::source`](std::error::Error::source) to get the underlying error.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Struct`](crate::Struct) and
    /// [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to an error in the field `{}`", 
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .field)]
    NamedFieldError {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Name of field where error occurred.
        field: &'static str,

        /// Underlying error in conversion of field.
        source: Box<FromReflectError>,
    },

    /// An error has occurred in conversion of a field at index given by the parameter `index`.
    ///
    /// Use [`Error::source`](std::error::Error::source) to get the underlying error.
    ///
    /// This error is given by types of [kind](ReflectKind) [`TupleStruct`](crate::TupleStruct)
    /// and [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to an error in the field at index {}", 
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .index)]
    UnnamedFieldError {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Index of field where error occurred.
        index: usize,

        /// Underlying error in conversion of field.
        source: Box<FromReflectError>,
    },

    /// An error has occurred in conversion of a value at index given by the parameter `index`.
    ///
    /// Use [`Error::source`](std::error::Error::source) to get the underlying error.
    ///
    /// This error is given by types of [kind](ReflectKind) [`List`](crate::List) and
    /// [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to an error in the value at index `{}`",
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .index)]
    IndexError {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Index of value where error occurred.
        index: usize,

        /// Underlying error in conversion of value at the index.
        source: Box<FromReflectError>,
    },

    /// An error has occurred in conversion of a variant with name given by the parameter `variant`.
    ///
    /// Use [`Error::source`](std::error::Error::source) to get the underlying error.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Enum`](crate::Enum).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to an error in the variant `{}`", 
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name(), .variant)]
    VariantError {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Name of variant where error occurred.
        variant: Cow<'static, str>,

        /// Underlying error in conversion of variant.
        source: Box<FromReflectError>,
    },

    /// An error has occurred in conversion of a key of Map.
    ///
    /// Use [`Error::source`](std::error::Error::source) to get the underlying error.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Map`](crate::Map).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to an error in a key of the Map",
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name())]
    KeyError {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Underlying error in conversion of a key of Map.
        source: Box<FromReflectError>,
    },

    /// An error has occurred in conversion of a value of Map.
    ///
    /// Use [`Error::source`](std::error::Error::source) to get the underlying error.
    ///
    /// This error is given by types of [kind](ReflectKind) [`Map`](crate::Map).
    #[error("The reflected type `{}` of kind {} cannot be converted to type `{}` due to an error in a value of the Map",
            .from_type.type_name(), self.display_from_kind(), .to_type.type_name())]
    ValueError {
        /// [`TypeInfo`] of the source type.
        from_type: &'static TypeInfo,

        /// [`ReflectKind`] of the source type.
        from_kind: ReflectKind,

        /// [`TypeInfo`] of the target type.
        to_type: &'static TypeInfo,

        /// Underlying error in conversion of a value of Map.
        source: Box<FromReflectError>,
    },
}

impl FromReflectError {
    /// Returns the [`TypeInfo`] of the source type.
    pub fn from_type(&self) -> &'static TypeInfo {
        match self {
            Self::InvalidType { from_type, .. }
            | Self::InvalidLength { from_type, .. }
            | Self::MissingNamedField { from_type, .. }
            | Self::MissingUnnamedField { from_type, .. }
            | Self::MissingIndex { from_type, .. }
            | Self::MissingVariant { from_type, .. }
            | Self::NamedFieldError { from_type, .. }
            | Self::UnnamedFieldError { from_type, .. }
            | Self::IndexError { from_type, .. }
            | Self::VariantError { from_type, .. }
            | Self::KeyError { from_type, .. }
            | Self::ValueError { from_type, .. } => from_type,
        }
    }

    /// Returns the [`TypeInfo`] of the target type.
    pub fn to_type(&self) -> &'static TypeInfo {
        match self {
            Self::InvalidType { to_type, .. }
            | Self::InvalidLength { to_type, .. }
            | Self::MissingNamedField { to_type, .. }
            | Self::MissingUnnamedField { to_type, .. }
            | Self::MissingIndex { to_type, .. }
            | Self::MissingVariant { to_type, .. }
            | Self::NamedFieldError { to_type, .. }
            | Self::UnnamedFieldError { to_type, .. }
            | Self::IndexError { to_type, .. }
            | Self::VariantError { to_type, .. }
            | Self::KeyError { to_type, .. }
            | Self::ValueError { to_type, .. } => to_type,
        }
    }

    /// Returns the [`ReflectKind`] of the source type.
    pub fn from_kind(&self) -> ReflectKind {
        *match self {
            Self::InvalidType { from_kind, .. }
            | Self::InvalidLength { from_kind, .. }
            | Self::MissingNamedField { from_kind, .. }
            | Self::MissingUnnamedField { from_kind, .. }
            | Self::MissingIndex { from_kind, .. }
            | Self::MissingVariant { from_kind, .. }
            | Self::NamedFieldError { from_kind, .. }
            | Self::UnnamedFieldError { from_kind, .. }
            | Self::IndexError { from_kind, .. }
            | Self::VariantError { from_kind, .. }
            | Self::KeyError { from_kind, .. }
            | Self::ValueError { from_kind, .. } => from_kind,
        }
    }

    /// Returns the [kind](ReflectKind) of source type for display purposes.
    fn display_from_kind(&self) -> String {
        let prefix = if let TypeInfo::Dynamic(_) = self.from_type() {
            "(Dynamic)"
        } else {
            ""
        };

        format!("{}{:?}", prefix, self.from_kind())
    }

    /// Returns the [kind](ReflectKind) of target type for display purposes.
    fn display_to_kind(&self) -> &str {
        match self.to_type() {
            TypeInfo::Struct(_) => "Struct",
            TypeInfo::TupleStruct(_) => "TupleStruct",
            TypeInfo::Tuple(_) => "Tuple",
            TypeInfo::List(_) => "List",
            TypeInfo::Array(_) => "Array",
            TypeInfo::Map(_) => "Map",
            TypeInfo::Enum(_) => "Enum",
            TypeInfo::Value(_) => "Value",
            TypeInfo::Dynamic(_) => "Dynamic",
        }
    }
}
