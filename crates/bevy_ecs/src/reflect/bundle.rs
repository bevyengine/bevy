//! Definitions for [`Bundle`] reflection.
//!
//! This module exports two types: [`ReflectBundleFns`] and [`ReflectBundle`].
//!
//! # Architecture
//!
//! [`ReflectBundle`] wraps a [`ReflectBundleFns`]. In fact, each method on
//! [`ReflectBundle`] wraps a call to a function pointer field in `ReflectBundleFns`.
//!
//! ## Who creates `ReflectBundle`s?
//!
//! When a user adds the `#[reflect(Bundle)]` attribute to their `#[derive(Reflect)]`
//! type, it tells the derive macro for `Reflect` to add the following single line to its
//! [`get_type_registration`] method (see the relevant code[^1]).
//!
//! ```ignore
//! registration.insert::<ReflectBundle>(FromType::<Self>::from_type());
//! ```
//!
//! This line adds a `ReflectBundle` to the registration data for the type in question.
//! The user can access the `ReflectBundle` for type `T` through the type registry,
//! as per the `trait_reflection.rs` example.
//!
//! The `FromType::<Self>::from_type()` in the previous line calls the `FromType<C>`
//! implementation of `ReflectBundle`.
//!
//! The `FromType<C>` impl creates a function per field of [`ReflectBundleFns`].
//! In those functions, we call generic methods on [`World`] and [`EntityMut`].
//!
//! The result is a `ReflectBundle` completely independent of `C`, yet capable
//! of using generic ECS methods such as `entity.remove::<C>()` to insert `&dyn Reflect`
//! with underlying type `C`, without the `C` appearing in the type signature.
//!
//! ## A note on code generation
//!
//! A downside of this approach is that monomorphized code (ie: concrete code
//! for generics) is generated **unconditionally**, regardless of whether it ends
//! up used or not.
//!
//! Adding `N` fields on `ReflectBundleFns` will generate `N × M` additional
//! functions, where `M` is how many types derive `#[reflect(Bundle)]`.
//!
//! Those functions will increase the size of the final app binary.
//!
//! [^1]: `crates/bevy_reflect/bevy_reflect_derive/src/registration.rs`
//!
//! [`get_type_registration`]: bevy_reflect::GetTypeRegistration::get_type_registration

use std::any::TypeId;

use crate::{
    prelude::Bundle,
    world::{EntityMut, FromWorld, World},
};
use bevy_reflect::{FromType, Reflect, ReflectRef, TypeRegistry};

use super::ReflectComponent;

/// A struct used to operate on reflected [`Bundle`] of a type.
///
/// A [`ReflectBundle`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectBundle(ReflectBundleFns);

/// The raw function pointers needed to make up a [`ReflectBundle`].
///
/// This is used when creating custom implementations of [`ReflectBundle`] with
/// [`ReflectBundle::new()`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectBundle`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectBundle`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Bundle)]` attribute.
/// > After adding the bundle to the [`TypeRegistry`][bevy_reflect::TypeRegistry],
/// > its [`ReflectBundle`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectBundle`] may be useful if you need to create new bundle types
/// at runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectBundle`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected bundles of that type will be inserted into the Bevy
/// world.
#[derive(Clone)]
pub struct ReflectBundleFns {
    /// Function pointer implementing [`ReflectBundle::from_world()`].
    pub from_world: fn(&mut World) -> Box<dyn Reflect>,
    /// Function pointer implementing [`ReflectBundle::insert()`].
    pub insert: fn(&mut EntityMut, &dyn Reflect),
    /// Function pointer implementing [`ReflectBundle::apply()`].
    pub apply: fn(&mut EntityMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::apply_or_insert()`].
    pub apply_or_insert: fn(&mut EntityMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::remove()`].
    pub remove: fn(&mut EntityMut),
}

impl ReflectBundleFns {
    /// Get the default set of [`ReflectBundleFns`] for a specific bundle type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Bundle + Reflect + FromWorld>() -> Self {
        <ReflectBundle as FromType<T>>::from_type().0
    }
}

impl ReflectBundle {
    /// Constructs default reflected [`Bundle`] from world using [`from_world()`](FromWorld::from_world).
    pub fn from_world(&self, world: &mut World) -> Box<dyn Reflect> {
        (self.0.from_world)(world)
    }

    /// Insert a reflected [`Bundle`] into the entity like [`insert()`](crate::world::EntityMut::insert).
    pub fn insert(&self, entity: &mut EntityMut, bundle: &dyn Reflect) {
        (self.0.insert)(entity, bundle);
    }

    /// Uses reflection to set the value of this [`Bundle`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Bundle`] of the given type.
    pub fn apply(&self, entity: &mut EntityMut, bundle: &dyn Reflect, registry: &TypeRegistry) {
        (self.0.apply)(entity, bundle, registry);
    }

    /// Uses reflection to set the value of this [`Bundle`] type in the entity to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(
        &self,
        entity: &mut EntityMut,
        bundle: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply_or_insert)(entity, bundle, registry);
    }

    /// Removes this [`Bundle`] type from the entity. Does nothing if it doesn't exist.
    pub fn remove(&self, entity: &mut EntityMut) {
        (self.0.remove)(entity);
    }

    /// Create a custom implementation of [`ReflectBundle`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Bundle)]` bundle
    /// to generate a [`ReflectBundle`] implementation automatically.
    ///
    /// See [`ReflectBundleFns`] for more information.
    pub fn new(fns: ReflectBundleFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectBundle`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectBundle>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectBundle`] and keeping it
    /// between frames, cloning a `ReflectBundle` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectBundle`,
    /// use `fn_pointers` to get the underlying [`ReflectBundleFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectBundle>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectBundleFns {
        &self.0
    }
}

impl<C: Bundle + Reflect + FromWorld> FromType<C> for ReflectBundle {
    fn from_type() -> Self {
        ReflectBundle(ReflectBundleFns {
            from_world: |world| Box::new(C::from_world(world)),
            insert: |entity, reflected_bundle| {
                let mut bundle = entity.world_scope(|world| C::from_world(world));
                bundle.apply(reflected_bundle);
                entity.insert(bundle);
            },
            apply: |entity, reflected_bundle, registry| {
                let mut bundle = entity.world_scope(|world| C::from_world(world));
                bundle.apply(reflected_bundle);

                if let ReflectRef::Struct(bundle) = bundle.reflect_ref() {
                    for field in bundle.iter_fields() {
                        if let Some(reflect_component) =
                            registry.get_type_data::<ReflectComponent>(field.type_id())
                        {
                            reflect_component.apply(entity, field);
                        } else if let Some(reflect_bundle) =
                            registry.get_type_data::<ReflectBundle>(field.type_id())
                        {
                            reflect_bundle.apply(entity, field, registry);
                        } else {
                            entity.world_scope(|world| {
                                if let Some(id) = world.bundles().get_id(TypeId::of::<C>()) {
                                    let info = world.bundles().get(id).unwrap();
                                    if info.components().is_empty() {
                                        panic!(
                                            "no `ReflectComponent` registration found for `{}`",
                                            field.type_name()
                                        );
                                    }
                                };
                            });

                            panic!(
                                "no `ReflectBundle` registration found for `{}`",
                                field.type_name()
                            )
                        }
                    }
                } else {
                    panic!(
                        "expected bundle `{}` to be named struct",
                        std::any::type_name::<C>()
                    );
                }
            },
            apply_or_insert: |entity, reflected_bundle, registry| {
                let mut bundle = entity.world_scope(|world| C::from_world(world));
                bundle.apply(reflected_bundle);

                if let ReflectRef::Struct(bundle) = bundle.reflect_ref() {
                    for field in bundle.iter_fields() {
                        if let Some(reflect_component) =
                            registry.get_type_data::<ReflectComponent>(field.type_id())
                        {
                            reflect_component.apply_or_insert(entity, field);
                        } else if let Some(reflect_bundle) =
                            registry.get_type_data::<ReflectBundle>(field.type_id())
                        {
                            reflect_bundle.apply_or_insert(entity, field, registry);
                        } else {
                            entity.world_scope(|world| {
                                if let Some(id) = world.bundles().get_id(TypeId::of::<C>()) {
                                    let info = world.bundles().get(id).unwrap();
                                    if info.components().is_empty() {
                                        panic!(
                                            "no `ReflectComponent` registration found for `{}`",
                                            field.type_name()
                                        );
                                    }
                                };
                            });

                            panic!(
                                "no `ReflectBundle` registration found for `{}`",
                                field.type_name()
                            )
                        }
                    }
                } else {
                    panic!(
                        "expected bundle `{}` to be named struct",
                        std::any::type_name::<C>()
                    );
                }
            },
            remove: |entity| {
                entity.remove::<C>();
            },
        })
    }
}
