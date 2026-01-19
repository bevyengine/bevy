//! Definitions for [`Resource`] reflection.
//!
//! # Architecture
//!
//! See the module doc for [`reflect::component`](`crate::reflect::component`).

use crate::{
    change_detection::Mut,
    component::ComponentId,
    resource::Resource,
    world::{
        error::ResourceFetchError, unsafe_world_cell::UnsafeWorldCell, FilteredResources,
        FilteredResourcesMut, World,
    },
};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect, TypePath, TypeRegistry};

use super::from_reflect_with_fallback;

/// A struct used to operate on reflected [`Resource`] of a type.
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource(ReflectResourceFns);

/// The raw function pointers needed to make up a [`ReflectResource`].
///
/// This is used when creating custom implementations of [`ReflectResource`] with
/// [`ReflectResource::new()`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectResource`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectResource`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Resource)]` attribute.
/// > After adding the component to the [`TypeRegistry`],
/// > its [`ReflectResource`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectResource`] may be useful if you need to create new resource types at
/// runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectResource`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected resources of that type will be inserted into the bevy
/// world.
#[derive(Clone)]
pub struct ReflectResourceFns {
    /// Function pointer implementing [`ReflectResource::insert()`].
    pub insert: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectResource::apply()`].
    pub apply: fn(&mut World, &dyn PartialReflect),
    /// Function pointer implementing [`ReflectResource::apply_or_insert()`].
    pub apply_or_insert: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectResource::remove()`].
    pub remove: fn(&mut World),
    /// Function pointer implementing [`ReflectResource::reflect()`].
    pub reflect:
        for<'w> fn(FilteredResources<'w, '_>) -> Result<&'w dyn Reflect, ResourceFetchError>,
    /// Function pointer implementing [`ReflectResource::reflect_mut()`].
    pub reflect_mut: for<'w> fn(
        FilteredResourcesMut<'w, '_>,
    ) -> Result<Mut<'w, dyn Reflect>, ResourceFetchError>,
    /// Function pointer implementing [`ReflectResource::reflect_unchecked_mut()`].
    ///
    /// # Safety
    /// The function may only be called with an [`UnsafeWorldCell`] that can be used to mutably access the relevant resource.
    pub reflect_unchecked_mut: unsafe fn(UnsafeWorldCell<'_>) -> Option<Mut<'_, dyn Reflect>>,
    /// Function pointer implementing [`ReflectResource::copy()`].
    pub copy: fn(&World, &mut World, &TypeRegistry),
    /// Function pointer implementing [`ReflectResource::register_resource()`].
    pub register_resource: fn(&mut World) -> ComponentId,
}

impl ReflectResourceFns {
    /// Get the default set of [`ReflectResourceFns`] for a specific resource type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Resource + FromReflect + TypePath>() -> Self {
        <ReflectResource as FromType<T>>::from_type().0
    }
}

impl ReflectResource {
    /// Insert a reflected [`Resource`] into the world like [`insert()`](World::insert_resource).
    pub fn insert(
        &self,
        world: &mut World,
        resource: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) {
        (self.0.insert)(world, resource, registry);
    }

    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn apply(&self, world: &mut World, resource: &dyn PartialReflect) {
        (self.0.apply)(world, resource);
    }

    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(
        &self,
        world: &mut World,
        resource: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply_or_insert)(world, resource, registry);
    }

    /// Removes this [`Resource`] type from the world. Does nothing if it doesn't exist.
    pub fn remove(&self, world: &mut World) {
        (self.0.remove)(world);
    }

    /// Gets the value of this [`Resource`] type from the world as a reflected reference.
    ///
    /// Note that [`&World`](World) is a valid type for `resources`.
    pub fn reflect<'w, 's>(
        &self,
        resources: impl Into<FilteredResources<'w, 's>>,
    ) -> Result<&'w dyn Reflect, ResourceFetchError> {
        (self.0.reflect)(resources.into())
    }

    /// Gets the value of this [`Resource`] type from the world as a mutable reflected reference.
    ///
    /// Note that [`&mut World`](World) is a valid type for `resources`.
    pub fn reflect_mut<'w, 's>(
        &self,
        resources: impl Into<FilteredResourcesMut<'w, 's>>,
    ) -> Result<Mut<'w, dyn Reflect>, ResourceFetchError> {
        (self.0.reflect_mut)(resources.into())
    }

    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method with an [`UnsafeWorldCell`] which can be used to mutably access the resource.
    /// * Don't call this method more than once in the same scope for a given [`Resource`].
    pub unsafe fn reflect_unchecked_mut<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
    ) -> Option<Mut<'w, dyn Reflect>> {
        // SAFETY: caller promises to uphold uniqueness guarantees
        unsafe { (self.0.reflect_unchecked_mut)(world) }
    }

    /// Gets the value of this [`Resource`] type from `source_world` and [applies](Self::apply()) it to the value of this [`Resource`] type in `destination_world`.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn copy(
        &self,
        source_world: &World,
        destination_world: &mut World,
        registry: &TypeRegistry,
    ) {
        (self.0.copy)(source_world, destination_world, registry);
    }

    /// Register the type of this [`Resource`] in [`World`], returning the [`ComponentId`]
    pub fn register_resource(&self, world: &mut World) -> ComponentId {
        (self.0.register_resource)(world)
    }

    /// Create a custom implementation of [`ReflectResource`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Resource)]` component
    /// to generate a [`ReflectResource`] implementation automatically.
    ///
    /// See [`ReflectResourceFns`] for more information.
    pub fn new(&self, fns: ReflectResourceFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectResource`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectResource>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectResource`] and keeping it
    /// between frames, cloning a `ReflectResource` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectResource`,
    /// use `fn_pointers` to get the underlying [`ReflectResourceFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectResource>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectResourceFns {
        &self.0
    }
}

impl<R: Resource + FromReflect + TypePath> FromType<R> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource(ReflectResourceFns {
            insert: |world, reflected_resource, registry| {
                let resource = from_reflect_with_fallback::<R>(reflected_resource, world, registry);
                world.insert_resource(resource);
            },
            apply: |world, reflected_resource| {
                let mut resource = world.resource_mut::<R>();
                resource.apply(reflected_resource);
            },
            apply_or_insert: |world, reflected_resource, registry| {
                if let Some(mut resource) = world.get_resource_mut::<R>() {
                    resource.apply(reflected_resource);
                } else {
                    let resource =
                        from_reflect_with_fallback::<R>(reflected_resource, world, registry);
                    world.insert_resource(resource);
                }
            },
            remove: |world| {
                world.remove_resource::<R>();
            },
            reflect: |world| world.get::<R>().map(|res| res.into_inner() as &dyn Reflect),
            reflect_mut: |world| {
                world
                    .into_mut::<R>()
                    .map(|res| res.map_unchanged(|value| value as &mut dyn Reflect))
            },
            reflect_unchecked_mut: |world| {
                // SAFETY: all usages of `reflect_unchecked_mut` guarantee that there is either a single mutable
                // reference or multiple immutable ones alive at any given point
                let res = unsafe { world.get_resource_mut::<R>() };
                res.map(|res| res.map_unchanged(|value| value as &mut dyn Reflect))
            },
            copy: |source_world, destination_world, registry| {
                let source_resource = source_world.resource::<R>();
                let destination_resource =
                    from_reflect_with_fallback::<R>(source_resource, destination_world, registry);
                destination_world.insert_resource(destination_resource);
            },

            register_resource: |world: &mut World| -> ComponentId {
                world.register_resource::<R>()
            },
        })
    }
}
