use crate::{dynamic::DynamicValue, CreateTypeData, Reflect};
use alloc::boxed::Box;

/// A trait that enables types to be dynamically constructed from reflected data.
///
/// It's recommended to use the [derive macro] rather than manually implementing this trait.
///
/// `FromDynamic` allows dynamic proxy types, like [`DynamicStruct`], to be used to generate
/// their concrete counterparts.
/// It can also be used to partially or fully clone a type (depending on whether it has
/// ignored fields or not).
///
/// In some cases, this trait may even be required.
/// Deriving [`Reflect`] on an enum requires all its fields to implement `FromDynamic`.
/// Additionally, some complex types like `Vec<T>` require that their element types
/// implement this trait.
/// The reason for such requirements is that some operations require new data to be constructed,
/// such as swapping to a new variant or pushing data to a homogeneous list.
///
/// See the [crate-level documentation] to see how this trait can be used.
///
/// [derive macro]: bevy_reflect_derive::FromDynamic
/// [`DynamicStruct`]: crate::dynamic::DynamicStruct
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `FromDynamic` so cannot be created through reflection",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait FromDynamic: Sized + 'static {
    /// Constructs a concrete instance of `Self` from a reflected value.
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self>;
}

/// Type data that represents the [`FromDynamic`] trait and allows it to be used dynamically.
///
/// `FromDynamic` allows dynamic types (e.g. [`DynamicStruct`], [`DynamicEnum`], etc.) to be converted
/// to their full, concrete types. This is most important when it comes to deserialization where it isn't
/// guaranteed that every field exists when trying to construct the final output.
///
/// However, to do this, you normally need to specify the exact concrete type:
///
/// ```
/// # use bevy_reflect::{dynamic::DynamicTupleStruct, FromDynamic, Reflect};
/// #[derive(Reflect, PartialEq, Eq, Debug)]
/// struct Foo(#[reflect(default = "default_value")] usize);
///
/// fn default_value() -> usize { 123 }
///
/// let reflected = DynamicTupleStruct::default();
///
/// let concrete: Foo = <Foo as FromDynamic>::from_dynamic(&reflected).unwrap();
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
/// # use bevy_reflect::{dynamic::DynamicTupleStruct, Reflect, ReflectFromDynamic, Typed, TypeRegistry, TypePath};
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
/// let rfr = registration.data::<ReflectFromDynamic>().unwrap();
///
/// let concrete: Box<dyn Reflect> = rfr.from_dynamic(&reflected).unwrap();
///
/// assert_eq!(Foo(123), concrete.take::<Foo>().unwrap());
/// ```
///
/// [`DynamicStruct`]: crate::dynamic::DynamicStruct
/// [`DynamicEnum`]: crate::enums::DynamicEnum
#[derive(Clone)]
pub struct ReflectFromDynamic {
    from_dynamic: fn(DynamicValue) -> Option<Box<dyn Reflect>>,
}

impl ReflectFromDynamic {
    /// Perform a [`FromDynamic::from_dynamic`] conversion on the given dynamic value.
    ///
    /// This will convert the dynamic value to a concrete type if it wasn't already, and return
    /// the value as `Box<dyn Reflect>`.
    pub fn from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>> {
        (self.from_dynamic)(dynamic)
    }
}

impl<T: Reflect + FromDynamic> CreateTypeData<T> for ReflectFromDynamic {
    fn create_type_data(_input: ()) -> Self {
        Self {
            from_dynamic: |dynamic| {
                T::from_dynamic(dynamic).map(|value| Box::new(value) as Box<dyn Reflect>)
            },
        }
    }
}
