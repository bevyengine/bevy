//! Type data for type registrations.
//!
//! Type data is extra metadata that can be added to a type's [`TypeRegistration`].
//! Most often this is used to provide information about a type's trait implementation dynamically,
//! but it may be used for other things such as specifying configuration values, opt-ins, and more.
//!
//! # Manual Type Data Creation
//!
//! Any type can be used as type data so long as it implements [`TypeData`].
//! The following code demonstrates how we might define type data that marks a type as "debuggable"
//! (note that this would probably be better handled through [custom attributes],
//! so this is just for demonstration purposes).
//!
//! It's customary to prefix all type data with `Reflect` as this allows it to be used with
//! the [derive macro](derive@crate::Reflect).
//!
//! ```rust
//! # use bevy_reflect::TypeData;
//! struct ReflectDebuggable;
//! impl TypeData for ReflectDebuggable {}
//!
//! ```
//!
//! We can then register our type data on any type we want:
//!
//! ```
//! # use bevy_reflect::{TypeData, TypeRegistration, TypeRegistry};
//! # struct ReflectDebuggable;
//! # impl TypeData for ReflectDebuggable {}
//! # let mut registry = TypeRegistry::empty();
//! let registration = TypeRegistration::of::<String>()
//!     .insert_data(ReflectDebuggable);
//! registry.add_registration(registration);
//! ```
//!
//! ## Using `CreateTypeData`
//!
//! In order to be compatible with [`TypeRegistry::register_type_data`],
//! we will also want to implement [`CreateTypeData`] for our struct.
//! This is usually done through a blanket implementation,
//! and allows us to add other requirements for usage.
//! For example, we can adjust out `ReflectDebuggable` to actually make use of [`Debug`].
//!
//! ```
//! # use core::fmt::Debug;
//! # use core::any::TypeId;
//! # use bevy_reflect::{CreateTypeData, Reflect, TypeData, TypeRegistration, TypeRegistry};
//! struct ReflectDebuggable {
//!   debug: fn(&dyn Reflect) -> String,
//! }
//! impl TypeData for ReflectDebuggable {}
//!
//! impl ReflectDebuggable {
//!   // Helper method for calling our `debug` function.
//!   fn debug<T: Reflect>(&self, value: &T) -> String {
//!     (self.debug)(value)
//!   }
//! }
//!
//! impl<T: Reflect + Debug> CreateTypeData<T> for ReflectDebuggable {
//!   fn create_type_data(input: ()) -> Self {
//!     Self {
//!       debug: |value| {
//!         let value = value .downcast_ref::<T>().unwrap();
//!         format!("{:?}", value)
//!       }
//!     }
//!   }
//! }
//!
//! # let mut registry = TypeRegistry::empty();
//! # registry.register::<Vec<i32>>();
//! // ...
//!
//! registry.register_type_data::<Vec<i32>, ReflectDebuggable>();
//! let data = registry.get_type_data::<ReflectDebuggable>(TypeId::of::<Vec<i32>>()).unwrap();
//! assert_eq!(data.debug(&vec![1, 2, 3]), "[1, 2, 3]");
//! ```
//!
//! [`CreateTypeData`] can also be created with input if needed.
//! Here, we didn't need input so we used the default input type of `()`.
//!
//! # Automatic Trait Reflection
//!
//! Because it's so commonplace to want to transform trait implementations into type data,
//! this crate provides a macro for doing just that called [`reflect_trait`].
//!
//! It can be used on any [dyn-compatible] trait and generates a `Reflect`-prefixed type data struct
//! that allows a reflected type to be cast into its trait object.
//!
//! ```
//! # use core::any::TypeId;
//! # use bevy_reflect::{Reflect, TypeRegistry, reflect_trait};
//! // This will generate a type data struct called `ReflectShout`.
//! #[reflect_trait]
//! trait Shout {
//!   fn shout(&self) -> String;
//! }
//!
//! impl Shout for String {
//!   fn shout(&self) -> String {
//!     format!("{}!!!", self)
//!   }
//! }
//!
//! # let mut registry = TypeRegistry::new();
//! // ...
//!  registry.register_type_data::<String, ReflectShout>();
//! let data = registry.get_type_data::<ReflectShout>(TypeId::of::<String>()).unwrap();
//!
//! let value: Box<dyn Reflect> = Box::new(String::from("Hello, world"));
//! let obj: &dyn Shout = data.get(&*value).unwrap();
//! assert_eq!(obj.shout(), "Hello, world!!!");
//! ```
//!
//! # Callbacks
//!
//! Both [`TypeData`] and [`CreateTypeData`] provide mechanisms for specifying registration callbacks.
//! The possible callbacks are:
//!
//! - `on_insert`: Triggered when the type data is inserted into a [`TypeRegistration`].
//! - `on_register`: Triggered when a [`TypeRegistration`] containing the type data is registered into a [`TypeRegistry`].
//!   If the [`TypeRegistration`] is already registered, then it will be triggered immediately.
//!
//! Callbacks are defined on both traits to provide as much flexibility as possible.
//! The callbacks on [`TypeData`] are always run and are intrinsic to the type itself.
//! Define your callback here if it doesn't rely on knowledge of the type it's being registered on.
//! The callbacks also have access to `&self`, which allows function pointers to be stored on the type data
//! and returned by these methods.
//!
//! The callbacks on [`CreateTypeData`] are associated functions that have access to the type they're
//! being registered on.
//! Note that these callbacks are only invoked when registered directly on a [`TypeRegistration`] or [`TypeRegistry`].
//! Calling [`CreateTypeData::create_type_data`] and inserting the return value will result in these
//! callbacks not being triggered.
//!
//! [`reflect_trait`]: bevy_reflect_derive::reflect_trait
//! [dyn-compatible]: https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility

use crate::{TypeRegistration, TypeRegistrationMut, TypeRegistry};
use downcast_rs::{impl_downcast, Downcast};

/// Type alias representing the callback function for when [`TypeData`] is inserted into a [`TypeRegistration`].
pub type OnInsertTypeData = fn(registration: TypeRegistrationMut<'_>);

/// Type alias representing the callback function for when a [`TypeRegistration`] is registered into a [`TypeRegistry`].
pub type OnRegisterTypeData = fn(registry: &mut TypeRegistry);

/// A trait for representing type metadata.
///
/// Type data can be registered to the [`TypeRegistry`] and stored on a type's [`TypeRegistration`].
///
/// While type data is often generated using the [`#[reflect_trait]`](crate::reflect_trait) macro,
/// any type that implements this trait can be considered "type data".
///
/// For creating your own type data generically or based on a specific type,
/// see the [`CreateTypeData`] trait.
///
/// See the [module-level documentation] for more information on type data.
///
/// [`TypeRegistry`]: crate::TypeRegistry
/// [`TypeRegistration`]: crate::TypeRegistration
/// [module-level documentation]: crate::type_data
pub trait TypeData: Downcast + Send + Sync {
    /// Optional callback for when this type data is inserted into a [`TypeRegistration`].
    fn on_insert(&self) -> Option<OnInsertTypeData> {
        None
    }

    /// Optional callback for when the [`TypeRegistration`] this type data belongs to is
    /// registered into a [`TypeRegistry`].
    ///
    /// Note that if this type data is inserted into a [`TypeRegistration`] that already belongs to a [`TypeRegistry`],
    /// then this should trigger immediately.
    fn on_register(&self) -> Option<OnRegisterTypeData> {
        None
    }
}
impl_downcast!(TypeData);

/// A trait for creating [`TypeData`].
///
/// Normally any type that implements [`TypeData`] can be inserted into a [`TypeRegistration`] using [`TypeRegistration::insert_data`].
/// However, only types that implement this trait may be registered using [`TypeRegistration::register_type_data`],
/// [`TypeRegistration::register_type_data_with`], or via the `#[reflect(MyTrait)]` attribute with the [`Reflect` derive macro].
///
/// Note that in order to work with the `#[reflect(MyTrait)]` attribute,
/// implementors must be named with the `Reflect` prefix (e.g., `ReflectMyTrait`).
///
/// # Input
///
/// By default, this trait expects no input for creating the type data
/// (the `Input` type parameter defaults to `()`).
///
/// However, implementors may choose to implement this trait with other input types.
/// As long as the implementations don't conflict, multiple different input types may be specified.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{CreateTypeData, Reflect};
/// trait Combine {
///   fn combine(a: f32, b: f32) -> f32;
/// }
///
/// #[derive(Clone)]
/// struct ReflectCombine {
///   multiplier: f32,
///   additional: f32,
///   combine: fn(f32, f32) -> f32,
/// }
///
/// impl ReflectCombine {
///   pub fn combine(&self, a: f32, b: f32) -> f32 {
///     let combined = (self.combine)(a, b);
///     let multiplied = self.multiplier * combined;
///     multiplied + self.additional
///   }
/// }
///
/// // A default implementation for when no input is given
/// impl<T: Combine + Reflect> CreateTypeData<T> for ReflectCombine {
///   fn create_type_data(_: ()) -> Self {
///     Self {
///       multiplier: 1.0,
///       additional: 0.0,
///       combine: T::combine,
///     }
///   }
/// }
///
/// // A custom implementation for when a multiplier is given
/// impl<T: Combine + Reflect> CreateTypeData<T, (f32, f32)> for ReflectCombine {
///   fn create_type_data(input: (f32, f32)) -> Self {
///     Self {
///       multiplier: input.0,
///       additional: input.1,
///       combine: T::combine,
///     }
///   }
/// }
///
/// #[derive(Reflect)]
/// // We can have the `Reflect` derive automatically register `ReflectCombine`:
/// #[reflect(Combine)]
/// struct WithoutMultiplier;
///
/// impl Combine for WithoutMultiplier {
///   fn combine(a: f32, b: f32) -> f32 {
///     a + b
///   }
/// }
///
/// #[derive(Reflect)]
/// // We can also given it some input:
/// #[reflect(Combine(2.0, 4.0))]
/// struct WithMultiplier;
///
/// impl Combine for WithMultiplier {
///   fn combine(a: f32, b: f32) -> f32 {
///     a + b
///   }
/// }
///
/// // Or we can simply create the data manually:
/// let without_multiplier = <ReflectCombine as CreateTypeData<WithoutMultiplier>>::create_type_data(());
/// let with_multiplier = <ReflectCombine as CreateTypeData<WithMultiplier, _>>::create_type_data((2.0, 4.0));
///
/// assert_eq!(without_multiplier.combine(1.0, 2.0), 3.0);
/// assert_eq!(with_multiplier.combine(1.0, 2.0), 10.0);
/// ```
///
/// [`TypeRegistration`]: crate::TypeRegistration
/// [`TypeRegistry::register_type_data`]: crate::TypeRegistry::register_type_data
/// [`TypeRegistry::register_type_data_with`]: crate::TypeRegistry::register_type_data_with
/// [`Reflect` derive macro]: derive@crate::Reflect
pub trait CreateTypeData<T, Input = ()>: TypeData {
    /// Create this type data using the given input.
    fn create_type_data(input: Input) -> Self;

    /// Inserts [`TypeData`] dependencies of this [`TypeData`].
    /// This is especially useful for trait [`TypeData`] that has a supertrait (ex: `A: B`).
    /// When the [`TypeData`] for `A` is inserted, the `B` [`TypeData`] will also be inserted.
    #[expect(
        unused_variables,
        reason = "default implementation does not have any dependencies"
    )]
    #[deprecated(
        since = "0.21.0",
        note = "This function will be removed in a future release. Use either `CreateTypeData::on_insert` or `TypeData::on_insert` instead."
    )]
    fn insert_dependencies(type_registration: &mut TypeRegistration) {}

    /// Optional callback for when this type data is created and inserted into a [`TypeRegistration`].
    fn on_insert() -> Option<OnInsertTypeData> {
        None
    }

    /// Optional callback for when the [`TypeRegistration`] the created type data belongs to is
    /// registered into a [`TypeRegistry`].
    ///
    /// Note that if this type data is inserted into a [`TypeRegistration`] that already belongs to a [`TypeRegistry`],
    /// then this should trigger immediately.
    fn on_register() -> Option<OnRegisterTypeData> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::{Reflect, TypeData, TypeRegistration};
    use core::any::TypeId;
    use core::marker::PhantomData;

    #[test]
    fn should_register_dependencies_on_insert() {
        #[derive(Reflect)]
        struct Foo;

        struct ReflectA;
        impl TypeData for ReflectA {
            fn on_insert(&self) -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.insert_data(ReflectB);
                })
            }
        }

        #[derive(TypeData)]
        struct ReflectB;

        let registration = TypeRegistration::of::<Foo>().insert_data(ReflectA);
        assert!(registration.contains::<ReflectA>());
        assert!(registration.contains::<ReflectB>());
    }

    #[test]
    fn should_register_dependencies_on_create_type_data_insert() {
        #[derive(Reflect)]
        struct Foo;

        struct ReflectA<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectA<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectA<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }

            fn on_insert() -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.register_type_data::<ReflectB<T>, _>();
                })
            }
        }

        struct ReflectB<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectB<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectB<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }
        }

        let registration = TypeRegistration::of::<Foo>().register_type_data::<ReflectA<Foo>, _>();
        assert_eq!(registration.len(), 2);
        assert!(registration.contains::<ReflectA<Foo>>());
        assert!(registration.contains::<ReflectB<Foo>>());
    }

    #[test]
    fn should_register_dependencies_on_insert_from_registry() {
        #[derive(Reflect)]
        struct Foo;

        struct ReflectA;
        impl TypeData for ReflectA {
            fn on_insert(&self) -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.insert_data(ReflectB);
                })
            }
        }

        impl<T> CreateTypeData<T> for ReflectA {
            fn create_type_data(_: ()) -> Self {
                Self
            }
        }

        #[derive(TypeData)]
        struct ReflectB;

        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();
        registry.register_type_data::<Foo, ReflectA>();

        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        assert!(registration.contains::<ReflectA>());
        assert!(registration.contains::<ReflectB>());
    }

    #[test]
    fn should_register_dependencies_on_create_type_data_insert_from_registry() {
        #[derive(Reflect)]
        struct Foo;

        struct ReflectA<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectA<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectA<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }

            fn on_insert() -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.register_type_data::<ReflectB<T>, _>();
                })
            }
        }

        struct ReflectB<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectB<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectB<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }
        }

        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();
        registry.register_type_data::<Foo, ReflectA<Foo>>();

        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        assert!(registration.contains::<ReflectA<Foo>>());
        assert!(registration.contains::<ReflectB<Foo>>());
    }

    #[test]
    fn should_handle_dependency_cycles_on_insert() {
        #[derive(Reflect)]
        struct Foo;

        struct ReflectA;
        impl TypeData for ReflectA {
            fn on_insert(&self) -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.insert_data(ReflectB);
                })
            }
        }

        struct ReflectB;
        impl TypeData for ReflectB {
            fn on_insert(&self) -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.insert_data(ReflectC);
                })
            }
        }

        struct ReflectC;
        impl TypeData for ReflectC {
            fn on_insert(&self) -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.insert_data(ReflectA);
                })
            }
        }

        let registration = TypeRegistration::of::<Foo>().insert_data(ReflectA);
        assert!(registration.contains::<ReflectA>());
        assert!(registration.contains::<ReflectB>());
        assert!(registration.contains::<ReflectC>());
    }

    #[test]
    fn should_handle_dependency_cycles_on_create_type_data_insert() {
        #[derive(Reflect)]
        struct Foo;

        struct ReflectA<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectA<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectA<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }

            fn on_insert() -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.register_type_data::<ReflectB<T>, _>();
                })
            }
        }

        struct ReflectB<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectB<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectB<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }

            fn on_insert() -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.register_type_data::<ReflectC<T>, _>();
                })
            }
        }

        struct ReflectC<T>(PhantomData<T>);
        impl<T: Send + Sync + 'static> TypeData for ReflectC<T> {}

        impl<T: Send + Sync + 'static> CreateTypeData<T> for ReflectC<T> {
            fn create_type_data(_: ()) -> Self {
                Self(PhantomData)
            }

            fn on_insert() -> Option<OnInsertTypeData> {
                Some(|mut registration| {
                    registration.register_type_data::<ReflectA<T>, _>();
                })
            }
        }

        let registration = TypeRegistration::of::<Foo>().register_type_data::<ReflectA<Foo>, _>();
        assert!(registration.contains::<ReflectA<Foo>>());
        assert!(registration.contains::<ReflectB<Foo>>());
        assert!(registration.contains::<ReflectC<Foo>>());
    }

    #[test]
    fn should_register_dependencies_on_register() {
        #[derive(Reflect)]
        struct Foo;

        #[derive(Reflect)]
        struct Bar;

        struct ReflectA;
        impl TypeData for ReflectA {
            fn on_register(&self) -> Option<OnRegisterTypeData> {
                Some(|registry| {
                    registry.register::<Bar>();
                })
            }
        }

        impl<T> CreateTypeData<T> for ReflectA {
            fn create_type_data(_: ()) -> Self {
                Self
            }
        }

        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();
        registry.register_type_data::<Foo, ReflectA>();

        assert!(registry.contains(TypeId::of::<Foo>()));
        assert!(registry.contains(TypeId::of::<Bar>()));
    }

    #[test]
    fn should_register_dependencies_on_create_type_data_register() {
        #[derive(Reflect)]
        struct Foo;

        #[derive(Reflect)]
        struct Bar;

        #[derive(TypeData)]
        struct ReflectA;

        impl<T> CreateTypeData<T> for ReflectA {
            fn create_type_data(_: ()) -> Self {
                Self
            }

            fn on_register() -> Option<OnRegisterTypeData> {
                Some(|registry| {
                    registry.register::<Bar>();
                })
            }
        }

        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();
        registry.register_type_data::<Foo, ReflectA>();

        assert!(registry.contains(TypeId::of::<Foo>()));
        assert!(registry.contains(TypeId::of::<Bar>()));
    }

    #[test]
    fn should_register_dependencies_on_deferred_register() {
        #[derive(Reflect)]
        struct Foo;

        #[derive(Reflect)]
        struct Bar;

        struct ReflectA;
        impl TypeData for ReflectA {
            fn on_register(&self) -> Option<OnRegisterTypeData> {
                Some(|registry| {
                    registry.register::<Bar>();
                })
            }
        }

        let registration = TypeRegistration::of::<Foo>().insert_data(ReflectA);

        let mut registry = TypeRegistry::empty();
        registry.add_registration(registration);

        assert!(registry.contains(TypeId::of::<Foo>()));
        assert!(registry.contains(TypeId::of::<Bar>()));
    }

    #[test]
    fn should_register_dependencies_on_deferred_create_type_data_register() {
        #[derive(Reflect)]
        struct Foo;

        #[derive(Reflect)]
        struct Bar;

        #[derive(TypeData)]
        struct ReflectA;

        impl<T> CreateTypeData<T> for ReflectA {
            fn create_type_data(_: ()) -> Self {
                Self
            }

            fn on_register() -> Option<OnRegisterTypeData> {
                Some(|registry| {
                    registry.register::<Bar>();
                })
            }
        }

        let registration = TypeRegistration::of::<Foo>().register_type_data::<ReflectA, Foo>();

        let mut registry = TypeRegistry::empty();
        registry.add_registration(registration);

        assert!(registry.contains(TypeId::of::<Foo>()));
        assert!(registry.contains(TypeId::of::<Bar>()));
    }
}
