use crate::TypeRegistry;
use core::fmt::{Debug, Formatter};
use downcast_rs::{impl_downcast, Downcast};

/// Type-erased [`TypeData`].
///
/// Type data can be registered to the [`TypeRegistry`] and stored on a type's [`TypeRegistration`].
///
/// While type data is often generated using the [`#[reflect_trait]`](crate::reflect_trait) macro,
/// almost any type that implements [`Clone`] can be considered "type data".
/// This is because it has a blanket implementation over all `T` where `T: Clone + Send + Sync + 'static`.
///
/// See the [crate-level documentation] for more information on type data and type registration.
///
/// [`TypeRegistration`]: crate::TypeRegistration
/// [crate-level documentation]: crate
pub trait BaseTypeData: Downcast + Send + Sync {
    fn type_name(&self) -> &'static str;
    fn clone_type_data(&self) -> Box<dyn BaseTypeData>;
}
impl_downcast!(BaseTypeData);

impl<T: 'static + Send + Sync> BaseTypeData for T
where
    T: Clone,
{
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn clone_type_data(&self) -> Box<dyn BaseTypeData> {
        Box::new(self.clone())
    }
}

impl Debug for dyn BaseTypeData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.type_name())
    }
}

/// Trait used for generating data for a type to be stored in a [`TypeRegistry`].
///
/// This is used to provide additional information about a type that can be used
/// dynamically at runtime.
/// Most often, this data is tied to a trait so that the trait may be used dynamically.
///
/// # Naming Convention
///
/// The [derive macro] requires that type data be prefixed with `Reflect`.
/// This is done so that the macro's registrations are less visually noisy.
/// For example, the type data for [`Default`] is called [`ReflectDefault`],
/// and would be registered via the derive macro like `#[reflect(Default)]`.
///
/// # Example
///
/// Let's say we have the following code and want to be able to use the trait dynamically:
///
/// ```
/// # use bevy_reflect::{Reflect};
/// trait Animal {
///   fn speak(&self) -> &'static str;
/// }
///
/// #[derive(Reflect)]
/// struct Dog(String);
///
/// impl Animal for Dog {
///   fn speak(&self) -> &'static str {
///     "woof"
///   }
/// }
/// ```
///
/// To do this, we can create a type data struct that implements [`TypeData`]:
///
/// ```
/// # use bevy_reflect::{TypeData, Reflect, FromReflect};
/// # trait Animal {
/// #   fn speak(&self) -> &'static str;
/// # }
/// #
/// # #[derive(Reflect)]
/// # struct Dog(String);
/// #
/// # impl Animal for Dog {
/// #   fn speak(&self) -> &'static str {
/// #     "woof"
/// #   }
/// # }
/// #
/// #[derive(Clone)]
/// struct ReflectAnimal {
///   speak: fn(&dyn Reflect) -> &'static str,
/// }
///
/// impl ReflectAnimal {
///   pub fn speak(&self, animal: &dyn Reflect) -> &'static str {
///     (self.speak)(animal)
///   }
/// }
///
/// impl<T: Animal + FromReflect> TypeData<T> for ReflectAnimal {
///   fn create_type_data() -> Self {
///     Self {
///       speak: |animal| {
///         T::from_reflect(animal).unwrap().speak()
///       }
///     }
///   }
/// }
///
/// // Usage
/// let dog = Dog("Fido".to_string());
/// let data = <ReflectAnimal as TypeData<Dog>>::create_type_data();
/// assert_eq!(data.speak(&dog), "woof");
/// ```
///
/// Alternatively, we can use the [`reflect_trait`] macro to generate a type data struct for us.
/// Note that this only works with [object-safe] traits.
///
/// ```
/// # use bevy_reflect::{TypeData, Reflect, FromReflect};
/// # use bevy_reflect_derive::reflect_trait;
/// #[reflect_trait]
/// trait Animal {
///   fn speak(&self) -> &'static str;
/// }
///
/// # #[derive(Reflect)]
/// # struct Dog(String);
/// #
/// # impl Animal for Dog {
/// #   fn speak(&self) -> &'static str {
/// #     "woof"
/// #   }
/// # }
/// #
/// // Usage
/// let dog = Dog("Fido".to_string());
/// let data = <ReflectAnimal as TypeData<Dog>>::create_type_data();
/// assert_eq!(data.get(&dog).unwrap().speak(), "woof");
/// ```
///
/// [derive macro]: bevy_reflect_derive::Reflect
/// [`ReflectDefault`]: crate::std_traits::ReflectDefault
/// [`reflect_trait`]: crate::reflect_trait
/// [object-safe]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
pub trait TypeData<T>: BaseTypeData + Clone {
    /// Create a new instance of this type data.
    fn create_type_data() -> Self;

    /// Callback for when the type data is fully registered.
    ///
    /// Type data becomes fully registered in one of the following ways:
    /// * The containing [`TypeRegistration`] is inserted into a [`TypeRegistry`]
    /// * This type data is inserted with [`TypeRegistry::register_type_data`]
    ///
    /// This can be used to register additional type data when this type data is registered.
    /// For example, to register the same data for related types or to register it
    /// for container types (e.g. `Vec<T>`, `Option<T>`, etc).
    ///
    /// [`TypeRegistration`]: crate::TypeRegistration
    #[allow(unused_variables)]
    fn on_register(registry: &mut TypeRegistry) {}
}
