use crate::{FromType, PartialReflect, Reflect, ReflectKindMismatchError};
use alloc::string::{String, ToString};
use alloc::{boxed::Box, vec::Vec};
use thiserror::Error;

/// A trait that enables types to be dynamically constructed from reflected data.
///
/// It's recommended to use the [derive macro] rather than manually implementing this trait.
///
/// `FromReflect` allows dynamic proxy types, like [`DynamicStruct`], to be used to generate
/// their concrete counterparts.
/// It can also be used to partially or fully clone a type (depending on whether it has
/// ignored fields or not).
///
/// In some cases, this trait may even be required.
/// Deriving [`Reflect`] on an enum requires all its fields to implement `FromReflect`.
/// Additionally, some complex types like `Vec<T>` require that their element types
/// implement this trait.
/// The reason for such requirements is that some operations require new data to be constructed,
/// such as swapping to a new variant or pushing data to a homogeneous list.
///
/// See the [crate-level documentation] to see how this trait can be used.
///
/// [derive macro]: bevy_reflect_derive::FromReflect
/// [`DynamicStruct`]: crate::DynamicStruct
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `FromReflect` so cannot be created through reflection",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait FromReflect: Reflect + Sized {
    /// Constructs a concrete instance of `Self` from a reflected value.
    fn from_reflect(reflect: &dyn PartialReflect) -> Result<Self, FromReflectError>;

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
        reflect: Box<dyn PartialReflect>,
    ) -> Result<Self, (FromReflectError, Box<dyn PartialReflect>)> {
        match reflect.try_take::<Self>() {
            Ok(value) => Ok(value),
            Err(value) => match Self::from_reflect(value.as_ref()) {
                Err(err) => Err((err, value)),
                Ok(value) => Ok(value),
            },
        }
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
/// #[derive(Reflect, PartialEq, Eq, Debug)]
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
/// # use bevy_reflect::{DynamicTupleStruct, Reflect, ReflectFromReflect, Typed, TypeRegistry, TypePath};
/// # #[derive(Reflect, PartialEq, Eq, Debug)]
/// # struct Foo(#[reflect(default = "default_value")] usize);
/// # fn default_value() -> usize { 123 }
/// # let mut registry = TypeRegistry::new();
/// # registry.register::<Foo>();
///
/// let mut reflected = DynamicTupleStruct::default();
/// reflected.set_represented_type(Some(<Foo as Typed>::type_info()));
///
/// let registration = registry.get_with_type_path(<Foo as TypePath>::type_path()).unwrap();
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
    from_reflect: fn(&dyn PartialReflect) -> Result<Box<dyn Reflect>, FromReflectError>,
}

impl ReflectFromReflect {
    /// Perform a [`FromReflect::from_reflect`] conversion on the given reflection object.
    ///
    /// This will convert the object to a concrete type if it wasn't already, and return
    /// the value as `Box<dyn Reflect>`.
    pub fn from_reflect(
        &self,
        reflect_value: &dyn PartialReflect,
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

#[derive(Error, Debug, PartialEq, Eq)]
pub enum FromReflectError {
    #[error("attempted to convert `{}` to `{}`", .0.received, .0.expected)]
    /// Attempted to convert the wrong [kind](crate::ReflectKind) to a type, e.g. a struct to a enum.
    MismatchedKinds(#[from] ReflectKindMismatchError),

    #[error("`{from_type}` is not `{to_type}`")]
    /// Tried to convert incompatible types.
    MismatchedTypes {
        from_type: Box<str>,
        to_type: Box<str>,
    },

    #[error("attempted to convert type with {from_size} size to a type with {to_size} size")]
    /// Attempted to convert to types with mismatched sizes, e.g. a [u8; 4] to [u8; 3].
    DifferentSize { from_size: usize, to_size: usize },

    #[error("attempted to convert missing tuple index `{0}`")]
    MissingTupleIndex(usize),

    #[error("attempted to convert missing field `{0}`")]
    MissingField(Box<str>),

    #[error("attempted to convert missing enum variant `{0}`")]
    MissingEnumVariant(Box<str>),

    #[error("{} at path `{}`", self.leaf_error(), self.path())]
    FieldError(Box<str>, Box<FromReflectError>),

    #[error("{} at path `{}`", self.leaf_error(), self.path())]
    TupleIndexError(usize, Box<FromReflectError>),

    #[error("{} at path `{}`", self.leaf_error(), self.path())]
    VariantError(Box<str>, Box<FromReflectError>),
}

impl FromReflectError {
    fn leaf_error(&self) -> String {
        match self {
            FromReflectError::FieldError(_, error)
            | FromReflectError::TupleIndexError(_, error)
            | FromReflectError::VariantError(_, error) => error.leaf_error(),
            other => other.to_string(),
        }
    }

    fn path(&self) -> String {
        self.reverse_path()
            .iter()
            .rev()
            .fold(String::new(), |acc, x| acc + x)
    }

    fn reverse_path(&self) -> Vec<String> {
        match self {
            FromReflectError::FieldError(field, error) => {
                let mut path = error.reverse_path();
                path.push(".".to_string() + field);
                path
            }
            FromReflectError::TupleIndexError(index, error) => {
                let mut path = error.reverse_path();
                path.push(".".to_string() + &index.to_string());
                path
            }
            FromReflectError::VariantError(variant, error) => {
                let mut path = error.reverse_path();
                path.push("::".to_string() + variant);
                path
            }
            _other => Vec::new(),
        }
    }
}
