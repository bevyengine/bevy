use crate::{FromType, Reflect};

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
    message = "`{Self}` can not be created through reflection",
    note = "consider annotating `{Self}` with `#[derive(FromReflect)]`"
)]
pub trait FromReflect: Reflect + Sized {
    /// Constructs a concrete instance of `Self` from a reflected value.
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self>;

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
    fn take_from_reflect(reflect: Box<dyn Reflect>) -> Result<Self, Box<dyn Reflect>> {
        match reflect.take::<Self>() {
            Ok(value) => Ok(value),
            Err(value) => match Self::from_reflect(value.as_ref()) {
                None => Err(value),
                Some(value) => Ok(value),
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
    from_reflect: fn(&dyn Reflect) -> Option<Box<dyn Reflect>>,
}

impl ReflectFromReflect {
    /// Perform a [`FromReflect::from_reflect`] conversion on the given reflection object.
    ///
    /// This will convert the object to a concrete type if it wasn't already, and return
    /// the value as `Box<dyn Reflect>`.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_reflect(&self, reflect_value: &dyn Reflect) -> Option<Box<dyn Reflect>> {
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
