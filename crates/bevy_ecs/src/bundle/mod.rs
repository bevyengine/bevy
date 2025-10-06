//! Types for handling [`Bundle`]s.
//!
//! This module contains the [`Bundle`] trait and some other helper types.

mod impls;
mod info;
mod insert;
mod remove;
mod spawner;
#[cfg(test)]
mod tests;

pub(crate) use insert::BundleInserter;
pub(crate) use remove::BundleRemover;
pub(crate) use spawner::BundleSpawner;

use bevy_ptr::MovingPtr;
use core::mem::MaybeUninit;
pub use info::*;

/// Derive the [`Bundle`] trait
///
/// You can apply this derive macro to structs that are
/// composed of [`Component`](crate::component::Component)s or
/// other [`Bundle`]s.
///
/// ## Attributes
///
/// Sometimes parts of the Bundle should not be inserted.
/// Those can be marked with `#[bundle(ignore)]`, and they will be skipped.
/// In that case, the field needs to implement [`Default`] unless you also ignore
/// the [`BundleFromComponents`] implementation.
///
/// ```rust
/// # use bevy_ecs::prelude::{Component, Bundle};
/// # #[derive(Component)]
/// # struct Hitpoint;
/// #
/// #[derive(Bundle)]
/// struct HitpointMarker {
///     hitpoints: Hitpoint,
///
///     #[bundle(ignore)]
///     creator: Option<String>
/// }
/// ```
///
/// Some fields may be bundles that do not implement
/// [`BundleFromComponents`]. This happens for bundles that cannot be extracted.
/// For example with [`SpawnRelatedBundle`](bevy_ecs::spawn::SpawnRelatedBundle), see below for an
/// example usage.
/// In those cases you can either ignore it as above,
/// or you can opt out the whole Struct by marking it as ignored with
/// `#[bundle(ignore_from_components)]`.
///
/// ```rust
/// # use bevy_ecs::prelude::{Component, Bundle, ChildOf, Spawn};
/// # #[derive(Component)]
/// # struct Hitpoint;
/// # #[derive(Component)]
/// # struct Marker;
/// #
/// use bevy_ecs::spawn::SpawnRelatedBundle;
///
/// #[derive(Bundle)]
/// #[bundle(ignore_from_components)]
/// struct HitpointMarker {
///     hitpoints: Hitpoint,
///     related_spawner: SpawnRelatedBundle<ChildOf, Spawn<Marker>>,
/// }
/// ```
pub use bevy_ecs_macros::Bundle;

use crate::{
    component::{ComponentId, Components, ComponentsRegistrator, StorageType},
    world::EntityWorldMut,
};
use bevy_ptr::OwningPtr;

/// The `Bundle` trait enables insertion and removal of [`Component`]s from an entity.
///
/// Implementers of the `Bundle` trait are called 'bundles'.
///
/// Each bundle represents a static set of [`Component`] types.
/// Currently, bundles can only contain one of each [`Component`], and will
/// panic once initialized if this is not met.
///
/// ## Insertion
///
/// The primary use for bundles is to add a useful collection of components to an entity.
///
/// Adding a value of bundle to an entity will add the components from the set it
/// represents to the entity.
/// The values of these components are taken from the bundle.
/// If an entity already had one of these components, the entity's original component value
/// will be overwritten.
///
/// Importantly, bundles are only their constituent set of components.
/// You **should not** use bundles as a unit of behavior.
/// The behavior of your app can only be considered in terms of components, as systems,
/// which drive the behavior of a `bevy` application, operate on combinations of
/// components.
///
/// This rule is also important because multiple bundles may contain the same component type,
/// calculated in different ways &mdash; adding both of these bundles to one entity
/// would create incoherent behavior.
/// This would be unexpected if bundles were treated as an abstraction boundary, as
/// the abstraction would be unmaintainable for these cases.
///
/// For this reason, there is intentionally no [`Query`] to match whether an entity
/// contains the components of a bundle.
/// Queries should instead only select the components they logically operate on.
///
/// ## Removal
///
/// Bundles are also used when removing components from an entity.
///
/// Removing a bundle from an entity will remove any of its components attached
/// to the entity from the entity.
/// That is, if the entity does not have all the components of the bundle, those
/// which are present will be removed.
///
/// # Implementers
///
/// Every type which implements [`Component`] also implements `Bundle`, since
/// [`Component`] types can be added to or removed from an entity.
///
/// Additionally, [Tuples](`tuple`) of bundles are also [`Bundle`] (with up to 15 bundles).
/// These bundles contain the items of the 'inner' bundles.
/// This is a convenient shorthand which is primarily used when spawning entities.
///
/// [`unit`], otherwise known as [`()`](`unit`), is a [`Bundle`] containing no components (since it
/// can also be considered as the empty tuple).
/// This can be useful for spawning large numbers of empty entities using
/// [`World::spawn_batch`](crate::world::World::spawn_batch).
///
/// Tuple bundles can be nested, which can be used to create an anonymous bundle with more than
/// 15 items.
/// However, in most cases where this is required, the derive macro [`derive@Bundle`] should be
/// used instead.
/// The derived `Bundle` implementation contains the items of its fields, which all must
/// implement `Bundle`.
/// As explained above, this includes any [`Component`] type, and other derived bundles.
///
/// If you want to add `PhantomData` to your `Bundle` you have to mark it with `#[bundle(ignore)]`.
/// ```
/// # use std::marker::PhantomData;
/// use bevy_ecs::{component::Component, bundle::Bundle};
///
/// #[derive(Component)]
/// struct XPosition(i32);
/// #[derive(Component)]
/// struct YPosition(i32);
///
/// #[derive(Bundle)]
/// struct PositionBundle {
///     // A bundle can contain components
///     x: XPosition,
///     y: YPosition,
/// }
///
/// // You have to implement `Default` for ignored field types in bundle structs.
/// #[derive(Default)]
/// struct Other(f32);
///
/// #[derive(Bundle)]
/// struct NamedPointBundle<T: Send + Sync + 'static> {
///     // Or other bundles
///     a: PositionBundle,
///     // In addition to more components
///     z: PointName,
///
///     // when you need to use `PhantomData` you have to mark it as ignored
///     #[bundle(ignore)]
///     _phantom_data: PhantomData<T>
/// }
///
/// #[derive(Component)]
/// struct PointName(String);
/// ```
///
/// # Safety
///
/// Manual implementations of this trait are unsupported.
/// That is, there is no safe way to implement this trait, and you must not do so.
/// If you want a type to implement [`Bundle`], you must use [`derive@Bundle`](derive@Bundle).
///
/// [`Component`]: crate::component::Component
/// [`Query`]: crate::system::Query
// Some safety points:
// - [`Bundle::component_ids`] must return the [`ComponentId`] for each component type in the
// bundle, in the _exact_ order that [`DynamicBundle::get_components`] is called.
// - [`Bundle::from_components`] must call `func` exactly once for each [`ComponentId`] returned by
//   [`Bundle::component_ids`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Bundle`",
    label = "invalid `Bundle`",
    note = "consider annotating `{Self}` with `#[derive(Component)]` or `#[derive(Bundle)]`"
)]
pub unsafe trait Bundle: DynamicBundle + Send + Sync + 'static {
    /// Gets this [`Bundle`]'s component ids, in the order of this bundle's [`Component`]s
    #[doc(hidden)]
    fn component_ids(components: &mut ComponentsRegistrator, ids: &mut impl FnMut(ComponentId));

    /// Gets this [`Bundle`]'s component ids. This will be [`None`] if the component has not been registered.
    fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>));
}

/// Creates a [`Bundle`] by taking it from internal storage.
///
/// # Safety
///
/// Manual implementations of this trait are unsupported.
/// That is, there is no safe way to implement this trait, and you must not do so.
/// If you want a type to implement [`Bundle`], you must use [`derive@Bundle`](derive@Bundle).
///
/// [`Query`]: crate::system::Query
// Some safety points:
// - [`Bundle::component_ids`] must return the [`ComponentId`] for each component type in the
// bundle, in the _exact_ order that [`DynamicBundle::get_components`] is called.
// - [`Bundle::from_components`] must call `func` exactly once for each [`ComponentId`] returned by
//   [`Bundle::component_ids`].
pub unsafe trait BundleFromComponents {
    /// Calls `func`, which should return data for each component in the bundle, in the order of
    /// this bundle's [`Component`]s
    ///
    /// # Safety
    /// Caller must return data for each component in the bundle, in the order of this bundle's
    /// [`Component`]s
    #[doc(hidden)]
    unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
    where
        // Ensure that the `OwningPtr` is used correctly
        F: for<'a> FnMut(&'a mut T) -> OwningPtr<'a>,
        Self: Sized;
}

/// The parts from [`Bundle`] that don't require statically knowing the components of the bundle.
pub trait DynamicBundle: Sized {
    /// An operation on the entity that happens _after_ inserting this bundle.
    type Effect;

    /// Moves the components out of the bundle.
    ///
    /// # Safety
    /// For callers:
    /// - Must be called exactly once before `apply_effect`
    /// - The `StorageType` argument passed into `func` must be correct for the component being fetched.
    /// - `apply_effect` must be called exactly once after this has been called if `Effect: !NoBundleEffect`
    ///
    /// For implementors:
    ///  - Implementors of this function must convert `ptr` into pointers to individual components stored within
    ///    `Self` and call `func` on each of them in exactly the same order as [`Bundle::get_component_ids`] and
    ///    [`BundleFromComponents::from_components`].
    ///  - If any part of `ptr` is to be accessed in `apply_effect`, it must *not* be dropped at any point in this
    ///    function. Calling [`bevy_ptr::deconstruct_moving_ptr`] in this function automatically ensures this.
    ///
    /// [`Component`]: crate::component::Component
    // This function explicitly uses `MovingPtr` to avoid potentially large stack copies of the bundle
    // when inserting into ECS storage. See https://github.com/bevyengine/bevy/issues/20571 for more
    // information.
    unsafe fn get_components(
        ptr: MovingPtr<'_, Self>,
        func: &mut impl FnMut(StorageType, OwningPtr<'_>),
    );

    /// Applies the after-effects of spawning this bundle.
    ///
    /// This is applied after all residual changes to the [`World`], including flushing the internal command
    /// queue.
    ///
    /// # Safety
    /// For callers:
    /// - Must be called exactly once after `get_components` has been called.
    /// - `ptr` must point to the instance of `Self` that `get_components` was called on,
    ///   all of fields that were moved out of in `get_components` will not be valid anymore.
    ///
    /// For implementors:
    ///  - If any part of `ptr` is to be accessed in this function, it must *not* be dropped at any point in
    ///    `get_components`. Calling [`bevy_ptr::deconstruct_moving_ptr`] in `get_components` automatically
    ///    ensures this is the case.
    ///
    /// [`World`]: crate::world::World
    // This function explicitly uses `MovingPtr` to avoid potentially large stack copies of the bundle
    // when inserting into ECS storage. See https://github.com/bevyengine/bevy/issues/20571 for more
    // information.
    unsafe fn apply_effect(ptr: MovingPtr<'_, MaybeUninit<Self>>, entity: &mut EntityWorldMut);
}

/// A trait implemented for [`DynamicBundle::Effect`] implementations that do nothing. This is used as a type constraint for
/// [`Bundle`] APIs that do not / cannot run [`DynamicBundle::Effect`], such as "batch spawn" APIs.
pub trait NoBundleEffect {}
