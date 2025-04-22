//! Types for handling [`Bundle`]s.
//!
//! This module contains the [`Bundle`] trait and some other helper types.

pub use bevy_ecs_macros::Bundle;

use crate::{
    archetype::{
        Archetype, ArchetypeAfterBundleInsert, ArchetypeId, Archetypes, BundleComponentStatus,
        ComponentStatus, SpawnBundleStatus,
    },
    change_detection::MaybeLocation,
    component::{
        Component, ComponentId, Components, ComponentsRegistrator, RequiredComponentConstructor,
        RequiredComponents, StorageType, Tick,
    },
    entity::{Entities, Entity, EntityLocation},
    observer::Observers,
    prelude::World,
    query::DebugCheckedUnwrap,
    relationship::RelationshipHookMode,
    storage::{SparseSetIndex, SparseSets, Storages, Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, EntityWorldMut, ON_ADD, ON_INSERT, ON_REPLACE},
};
use alloc::{boxed::Box, vec, vec::Vec};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_ptr::{ConstNonNull, OwningPtr};
use bevy_utils::TypeIdMap;
use core::{any::TypeId, ptr::NonNull};
use variadics_please::all_tuples;

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

    /// Registers components that are required by the components in this [`Bundle`].
    fn register_required_components(
        _components: &mut ComponentsRegistrator,
        _required_components: &mut RequiredComponents,
    );
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
pub trait DynamicBundle {
    /// An operation on the entity that happens _after_ inserting this bundle.
    type Effect: BundleEffect;
    // SAFETY:
    // The `StorageType` argument passed into [`Bundle::get_components`] must be correct for the
    // component being fetched.
    //
    /// Calls `func` on each value, in the order of this bundle's [`Component`]s. This passes
    /// ownership of the component values to `func`.
    #[doc(hidden)]
    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) -> Self::Effect;
}

/// An operation on an [`Entity`] that occurs _after_ inserting the [`Bundle`] that defined this bundle effect.
/// The order of operations is:
///
/// 1. The [`Bundle`] is inserted on the entity
/// 2. Relevant Hooks are run for the insert, then Observers
/// 3. The [`BundleEffect`] is run.
///
/// See [`DynamicBundle::Effect`].
pub trait BundleEffect {
    /// Applies this effect to the given `entity`.
    fn apply(self, entity: &mut EntityWorldMut);
}

// SAFETY:
// - `Bundle::component_ids` calls `ids` for C's component id (and nothing else)
// - `Bundle::get_components` is called exactly once for C and passes the component's storage type based on its associated constant.
unsafe impl<C: Component> Bundle for C {
    fn component_ids(components: &mut ComponentsRegistrator, ids: &mut impl FnMut(ComponentId)) {
        ids(components.register_component::<C>());
    }

    fn register_required_components(
        components: &mut ComponentsRegistrator,
        required_components: &mut RequiredComponents,
    ) {
        let component_id = components.register_component::<C>();
        <C as Component>::register_required_components(
            component_id,
            components,
            required_components,
            0,
            &mut Vec::new(),
        );
    }

    fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>)) {
        ids(components.get_id(TypeId::of::<C>()));
    }
}

// SAFETY:
// - `Bundle::from_components` calls `func` exactly once for C, which is the exact value returned by `Bundle::component_ids`.
unsafe impl<C: Component> BundleFromComponents for C {
    unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
    where
        // Ensure that the `OwningPtr` is used correctly
        F: for<'a> FnMut(&'a mut T) -> OwningPtr<'a>,
        Self: Sized,
    {
        let ptr = func(ctx);
        // Safety: The id given in `component_ids` is for `Self`
        unsafe { ptr.read() }
    }
}

impl<C: Component> DynamicBundle for C {
    type Effect = ();
    #[inline]
    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) -> Self::Effect {
        OwningPtr::make(self, |ptr| func(C::STORAGE_TYPE, ptr));
    }
}

macro_rules! tuple_impl {
    ($(#[$meta:meta])* $($name: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        #[allow(
            unused_mut,
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        // SAFETY:
        // - `Bundle::component_ids` calls `ids` for each component type in the
        // bundle, in the exact order that `DynamicBundle::get_components` is called.
        // - `Bundle::from_components` calls `func` exactly once for each `ComponentId` returned by `Bundle::component_ids`.
        // - `Bundle::get_components` is called exactly once for each member. Relies on the above implementation to pass the correct
        //   `StorageType` into the callback.
        unsafe impl<$($name: Bundle),*> Bundle for ($($name,)*) {
            fn component_ids(components: &mut ComponentsRegistrator,  ids: &mut impl FnMut(ComponentId)){
                $(<$name as Bundle>::component_ids(components, ids);)*
            }

            fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>)){
                $(<$name as Bundle>::get_component_ids(components, ids);)*
            }

            fn register_required_components(
                components: &mut ComponentsRegistrator,
                required_components: &mut RequiredComponents,
            ) {
                $(<$name as Bundle>::register_required_components(components, required_components);)*
            }
        }

        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        #[allow(
            unused_mut,
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        // SAFETY:
        // - `Bundle::component_ids` calls `ids` for each component type in the
        // bundle, in the exact order that `DynamicBundle::get_components` is called.
        // - `Bundle::from_components` calls `func` exactly once for each `ComponentId` returned by `Bundle::component_ids`.
        // - `Bundle::get_components` is called exactly once for each member. Relies on the above implementation to pass the correct
        //   `StorageType` into the callback.
        unsafe impl<$($name: BundleFromComponents),*> BundleFromComponents for ($($name,)*) {
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
            where
                F: FnMut(&mut T) -> OwningPtr<'_>
            {
                #[allow(
                    unused_unsafe,
                    reason = "Zero-length tuples will not run anything in the unsafe block. Additionally, rewriting this to move the () outside of the unsafe would require putting the safety comment inside the tuple, hurting readability of the code."
                )]
                // SAFETY: Rust guarantees that tuple calls are evaluated 'left to right'.
                // https://doc.rust-lang.org/reference/expressions.html#evaluation-order-of-operands
                unsafe { ($(<$name as BundleFromComponents>::from_components(ctx, func),)*) }
            }
        }

        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        #[allow(
            unused_mut,
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        impl<$($name: Bundle),*> DynamicBundle for ($($name,)*) {
            type Effect = ($($name::Effect,)*);
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            #[inline(always)]
            fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) -> Self::Effect {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($(mut $name,)*) = self;
                ($(
                    $name.get_components(&mut *func),
                )*)
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    tuple_impl,
    0,
    15,
    B
);

/// A trait implemented for [`BundleEffect`] implementations that do nothing. This is used as a type constraint for
/// [`Bundle`] APIs that do not / cannot run [`DynamicBundle::Effect`], such as "batch spawn" APIs.
pub trait NoBundleEffect {}

macro_rules! after_effect_impl {
    ($($after_effect: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        impl<$($after_effect: BundleEffect),*> BundleEffect for ($($after_effect,)*) {
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case.")
            ]
            fn apply(self, _entity: &mut EntityWorldMut) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($after_effect,)*) = self;
                $($after_effect.apply(_entity);)*
            }
        }

        impl<$($after_effect: NoBundleEffect),*> NoBundleEffect for ($($after_effect,)*) { }
    }
}

all_tuples!(after_effect_impl, 0, 15, P);

/// For a specific [`World`], this stores a unique value identifying a type of a registered [`Bundle`].
///
/// [`World`]: crate::world::World
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct BundleId(usize);

impl BundleId {
    /// Returns the index of the associated [`Bundle`] type.
    ///
    /// Note that this is unique per-world, and should not be reused across them.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

impl SparseSetIndex for BundleId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index()
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

/// What to do on insertion if a component already exists.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InsertMode {
    /// Any existing components of a matching type will be overwritten.
    Replace,
    /// Any existing components of a matching type will be left unchanged.
    Keep,
}

/// Stores metadata associated with a specific type of [`Bundle`] for a given [`World`].
///
/// [`World`]: crate::world::World
pub struct BundleInfo {
    id: BundleId,
    /// The list of all components contributed by the bundle (including Required Components). This is in
    /// the order `[EXPLICIT_COMPONENTS][REQUIRED_COMPONENTS]`
    ///
    /// # Safety
    /// Every ID in this list must be valid within the World that owns the [`BundleInfo`],
    /// must have its storage initialized (i.e. columns created in tables, sparse set created),
    /// and the range (0..`explicit_components_len`) must be in the same order as the source bundle
    /// type writes its components in.
    component_ids: Vec<ComponentId>,
    required_components: Vec<RequiredComponentConstructor>,
    explicit_components_len: usize,
}

impl BundleInfo {
    /// Create a new [`BundleInfo`].
    ///
    /// # Safety
    ///
    /// Every ID in `component_ids` must be valid within the World that owns the `BundleInfo`
    /// and must be in the same order as the source bundle type writes its components in.
    unsafe fn new(
        bundle_type_name: &'static str,
        storages: &mut Storages,
        components: &Components,
        mut component_ids: Vec<ComponentId>,
        id: BundleId,
    ) -> BundleInfo {
        // check for duplicates
        let mut deduped = component_ids.clone();
        deduped.sort_unstable();
        deduped.dedup();
        if deduped.len() != component_ids.len() {
            // TODO: Replace with `Vec::partition_dedup` once https://github.com/rust-lang/rust/issues/54279 is stabilized
            let mut seen = <HashSet<_>>::default();
            let mut dups = Vec::new();
            for id in component_ids {
                if !seen.insert(id) {
                    dups.push(id);
                }
            }

            let names = dups
                .into_iter()
                .map(|id| {
                    // SAFETY: the caller ensures component_id is valid.
                    unsafe { components.get_info_unchecked(id).name() }
                })
                .collect::<Vec<_>>()
                .join(", ");

            panic!("Bundle {bundle_type_name} has duplicate components: {names}");
        }

        // handle explicit components
        let explicit_components_len = component_ids.len();
        let mut required_components = RequiredComponents::default();
        for component_id in component_ids.iter().copied() {
            // SAFETY: caller has verified that all ids are valid
            let info = unsafe { components.get_info_unchecked(component_id) };
            required_components.merge(info.required_components());
            storages.prepare_component(info);
        }
        required_components.remove_explicit_components(&component_ids);

        // handle required components
        let required_components = required_components
            .0
            .into_iter()
            .map(|(component_id, v)| {
                // Safety: These ids came out of the passed `components`, so they must be valid.
                let info = unsafe { components.get_info_unchecked(component_id) };
                storages.prepare_component(info);
                // This adds required components to the component_ids list _after_ using that list to remove explicitly provided
                // components. This ordering is important!
                component_ids.push(component_id);
                v.constructor
            })
            .collect();

        // SAFETY: The caller ensures that component_ids:
        // - is valid for the associated world
        // - has had its storage initialized
        // - is in the same order as the source bundle type
        BundleInfo {
            id,
            component_ids,
            required_components,
            explicit_components_len,
        }
    }

    /// Returns a value identifying the associated [`Bundle`] type.
    #[inline]
    pub const fn id(&self) -> BundleId {
        self.id
    }

    /// Returns the [ID](ComponentId) of each component explicitly defined in this bundle (ex: Required Components are excluded).
    ///
    /// For all components contributed by this bundle (including Required Components), see [`BundleInfo::contributed_components`]
    #[inline]
    pub fn explicit_components(&self) -> &[ComponentId] {
        &self.component_ids[0..self.explicit_components_len]
    }

    /// Returns the [ID](ComponentId) of each Required Component needed by this bundle. This _does not include_ Required Components that are
    /// explicitly provided by the bundle.
    #[inline]
    pub fn required_components(&self) -> &[ComponentId] {
        &self.component_ids[self.explicit_components_len..]
    }

    /// Returns the [ID](ComponentId) of each component contributed by this bundle. This includes Required Components.
    ///
    /// For only components explicitly defined in this bundle, see [`BundleInfo::explicit_components`]
    #[inline]
    pub fn contributed_components(&self) -> &[ComponentId] {
        &self.component_ids
    }

    /// Returns an iterator over the [ID](ComponentId) of each component explicitly defined in this bundle (ex: this excludes Required Components).
    /// To iterate all components contributed by this bundle (including Required Components), see [`BundleInfo::iter_contributed_components`]
    #[inline]
    pub fn iter_explicit_components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.explicit_components().iter().copied()
    }

    /// Returns an iterator over the [ID](ComponentId) of each component contributed by this bundle. This includes Required Components.
    ///
    /// To iterate only components explicitly defined in this bundle, see [`BundleInfo::iter_explicit_components`]
    #[inline]
    pub fn iter_contributed_components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.component_ids.iter().copied()
    }

    /// Returns an iterator over the [ID](ComponentId) of each Required Component needed by this bundle. This _does not include_ Required Components that are
    /// explicitly provided by the bundle.
    pub fn iter_required_components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.required_components().iter().copied()
    }

    /// This writes components from a given [`Bundle`] to the given entity.
    ///
    /// # Safety
    ///
    /// `bundle_component_status` must return the "correct" [`ComponentStatus`] for each component
    /// in the [`Bundle`], with respect to the entity's original archetype (prior to the bundle being added).
    ///
    /// For example, if the original archetype already has `ComponentA` and `T` also has `ComponentA`, the status
    /// should be `Existing`. If the original archetype does not have `ComponentA`, the status should be `Added`.
    ///
    /// When "inserting" a bundle into an existing entity, [`ArchetypeAfterBundleInsert`]
    /// should be used, which will report `Added` vs `Existing` status based on the current archetype's structure.
    ///
    /// When spawning a bundle, [`SpawnBundleStatus`] can be used instead, which removes the need
    /// to look up the [`ArchetypeAfterBundleInsert`] in the archetype graph, which requires
    /// ownership of the entity's current archetype.
    ///
    /// `table` must be the "new" table for `entity`. `table_row` must have space allocated for the
    /// `entity`, `bundle` must match this [`BundleInfo`]'s type
    #[inline]
    unsafe fn write_components<'a, T: DynamicBundle, S: BundleComponentStatus>(
        &self,
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        bundle_component_status: &S,
        required_components: impl Iterator<Item = &'a RequiredComponentConstructor>,
        entity: Entity,
        table_row: TableRow,
        change_tick: Tick,
        bundle: T,
        insert_mode: InsertMode,
        caller: MaybeLocation,
    ) -> T::Effect {
        // NOTE: get_components calls this closure on each component in "bundle order".
        // bundle_info.component_ids are also in "bundle order"
        let mut bundle_component = 0;
        let after_effect = bundle.get_components(&mut |storage_type, component_ptr| {
            let component_id = *self.component_ids.get_unchecked(bundle_component);
            match storage_type {
                StorageType::Table => {
                    // SAFETY: bundle_component is a valid index for this bundle
                    let status = unsafe { bundle_component_status.get_status(bundle_component) };
                    // SAFETY: If component_id is in self.component_ids, BundleInfo::new ensures that
                    // the target table contains the component.
                    let column = table.get_column_mut(component_id).debug_checked_unwrap();
                    match (status, insert_mode) {
                        (ComponentStatus::Added, _) => {
                            column.initialize(table_row, component_ptr, change_tick, caller);
                        }
                        (ComponentStatus::Existing, InsertMode::Replace) => {
                            column.replace(table_row, component_ptr, change_tick, caller);
                        }
                        (ComponentStatus::Existing, InsertMode::Keep) => {
                            if let Some(drop_fn) = table.get_drop_for(component_id) {
                                drop_fn(component_ptr);
                            }
                        }
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set =
                        // SAFETY: If component_id is in self.component_ids, BundleInfo::new ensures that
                        // a sparse set exists for the component.
                        unsafe { sparse_sets.get_mut(component_id).debug_checked_unwrap() };
                    sparse_set.insert(entity, component_ptr, change_tick, caller);
                }
            }
            bundle_component += 1;
        });

        for required_component in required_components {
            required_component.initialize(
                table,
                sparse_sets,
                change_tick,
                table_row,
                entity,
                caller,
            );
        }

        after_effect
    }

    /// Internal method to initialize a required component from an [`OwningPtr`]. This should ultimately be called
    /// in the context of [`BundleInfo::write_components`], via [`RequiredComponentConstructor::initialize`].
    ///
    /// # Safety
    ///
    /// `component_ptr` must point to a required component value that matches the given `component_id`. The `storage_type` must match
    /// the type associated with `component_id`. The `entity` and `table_row` must correspond to an entity with an uninitialized
    /// component matching `component_id`.
    ///
    /// This method _should not_ be called outside of [`BundleInfo::write_components`].
    /// For more information, read the [`BundleInfo::write_components`] safety docs.
    /// This function inherits the safety requirements defined there.
    pub(crate) unsafe fn initialize_required_component(
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        change_tick: Tick,
        table_row: TableRow,
        entity: Entity,
        component_id: ComponentId,
        storage_type: StorageType,
        component_ptr: OwningPtr,
        caller: MaybeLocation,
    ) {
        {
            match storage_type {
                StorageType::Table => {
                    let column =
                        // SAFETY: If component_id is in required_components, BundleInfo::new requires that
                        // the target table contains the component.
                        unsafe { table.get_column_mut(component_id).debug_checked_unwrap() };
                    column.initialize(table_row, component_ptr, change_tick, caller);
                }
                StorageType::SparseSet => {
                    let sparse_set =
                        // SAFETY: If component_id is in required_components, BundleInfo::new requires that
                        // a sparse set exists for the component.
                        unsafe { sparse_sets.get_mut(component_id).debug_checked_unwrap() };
                    sparse_set.insert(entity, component_ptr, change_tick, caller);
                }
            }
        }
    }

    /// Inserts a bundle into the given archetype and returns the resulting archetype.
    /// This could be the same [`ArchetypeId`], in the event that inserting the given bundle
    /// does not result in an [`Archetype`] change.
    ///
    /// Results are cached in the [`Archetype`] graph to avoid redundant work.
    ///
    /// # Safety
    /// `components` must be the same components as passed in [`Self::new`]
    pub(crate) unsafe fn insert_bundle_into_archetype(
        &self,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        components: &Components,
        observers: &Observers,
        archetype_id: ArchetypeId,
    ) -> ArchetypeId {
        if let Some(archetype_after_insert_id) = archetypes[archetype_id]
            .edges()
            .get_archetype_after_bundle_insert(self.id)
        {
            return archetype_after_insert_id;
        }
        let mut new_table_components = Vec::new();
        let mut new_sparse_set_components = Vec::new();
        let mut bundle_status = Vec::with_capacity(self.explicit_components_len);
        let mut added_required_components = Vec::new();
        let mut added = Vec::new();
        let mut existing = Vec::new();

        let current_archetype = &mut archetypes[archetype_id];
        for component_id in self.iter_explicit_components() {
            if current_archetype.contains(component_id) {
                bundle_status.push(ComponentStatus::Existing);
                existing.push(component_id);
            } else {
                bundle_status.push(ComponentStatus::Added);
                added.push(component_id);
                // SAFETY: component_id exists
                let component_info = unsafe { components.get_info_unchecked(component_id) };
                match component_info.storage_type() {
                    StorageType::Table => new_table_components.push(component_id),
                    StorageType::SparseSet => new_sparse_set_components.push(component_id),
                }
            }
        }

        for (index, component_id) in self.iter_required_components().enumerate() {
            if !current_archetype.contains(component_id) {
                added_required_components.push(self.required_components[index].clone());
                added.push(component_id);
                // SAFETY: component_id exists
                let component_info = unsafe { components.get_info_unchecked(component_id) };
                match component_info.storage_type() {
                    StorageType::Table => {
                        new_table_components.push(component_id);
                    }
                    StorageType::SparseSet => {
                        new_sparse_set_components.push(component_id);
                    }
                }
            }
        }

        if new_table_components.is_empty() && new_sparse_set_components.is_empty() {
            let edges = current_archetype.edges_mut();
            // The archetype does not change when we insert this bundle.
            edges.cache_archetype_after_bundle_insert(
                self.id,
                archetype_id,
                bundle_status,
                added_required_components,
                added,
                existing,
            );
            archetype_id
        } else {
            let table_id;
            let table_components;
            let sparse_set_components;
            // The archetype changes when we insert this bundle. Prepare the new archetype and storages.
            {
                let current_archetype = &archetypes[archetype_id];
                table_components = if new_table_components.is_empty() {
                    // If there are no new table components, we can keep using this table.
                    table_id = current_archetype.table_id();
                    current_archetype.table_components().collect()
                } else {
                    new_table_components.extend(current_archetype.table_components());
                    // Sort to ignore order while hashing.
                    new_table_components.sort_unstable();
                    // SAFETY: all component ids in `new_table_components` exist
                    table_id = unsafe {
                        storages
                            .tables
                            .get_id_or_insert(&new_table_components, components)
                    };

                    new_table_components
                };

                sparse_set_components = if new_sparse_set_components.is_empty() {
                    current_archetype.sparse_set_components().collect()
                } else {
                    new_sparse_set_components.extend(current_archetype.sparse_set_components());
                    // Sort to ignore order while hashing.
                    new_sparse_set_components.sort_unstable();
                    new_sparse_set_components
                };
            };
            // SAFETY: ids in self must be valid
            let new_archetype_id = archetypes.get_id_or_insert(
                components,
                observers,
                table_id,
                table_components,
                sparse_set_components,
            );
            // Add an edge from the old archetype to the new archetype.
            archetypes[archetype_id]
                .edges_mut()
                .cache_archetype_after_bundle_insert(
                    self.id,
                    new_archetype_id,
                    bundle_status,
                    added_required_components,
                    added,
                    existing,
                );
            new_archetype_id
        }
    }

    /// Removes a bundle from the given archetype and returns the resulting archetype
    /// (or `None` if the removal was invalid).
    /// This could be the same [`ArchetypeId`], in the event that removing the given bundle
    /// does not result in an [`Archetype`] change.
    ///
    /// Results are cached in the [`Archetype`] graph to avoid redundant work.
    ///
    /// If `intersection` is false, attempting to remove a bundle with components not contained in the
    /// current archetype will fail, returning `None`.
    ///
    /// If `intersection` is true, components in the bundle but not in the current archetype
    /// will be ignored.
    ///
    /// # Safety
    /// `archetype_id` must exist and components in `bundle_info` must exist
    pub(crate) unsafe fn remove_bundle_from_archetype(
        &self,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        components: &Components,
        observers: &Observers,
        archetype_id: ArchetypeId,
        intersection: bool,
    ) -> Option<ArchetypeId> {
        // Check the archetype graph to see if the bundle has been
        // removed from this archetype in the past.
        let archetype_after_remove_result = {
            let edges = archetypes[archetype_id].edges();
            if intersection {
                edges.get_archetype_after_bundle_remove(self.id())
            } else {
                edges.get_archetype_after_bundle_take(self.id())
            }
        };
        let result = if let Some(result) = archetype_after_remove_result {
            // This bundle removal result is cached. Just return that!
            result
        } else {
            let mut next_table_components;
            let mut next_sparse_set_components;
            let next_table_id;
            {
                let current_archetype = &mut archetypes[archetype_id];
                let mut removed_table_components = Vec::new();
                let mut removed_sparse_set_components = Vec::new();
                for component_id in self.iter_explicit_components() {
                    if current_archetype.contains(component_id) {
                        // SAFETY: bundle components were already initialized by bundles.get_info
                        let component_info = unsafe { components.get_info_unchecked(component_id) };
                        match component_info.storage_type() {
                            StorageType::Table => removed_table_components.push(component_id),
                            StorageType::SparseSet => {
                                removed_sparse_set_components.push(component_id);
                            }
                        }
                    } else if !intersection {
                        // A component in the bundle was not present in the entity's archetype, so this
                        // removal is invalid. Cache the result in the archetype graph.
                        current_archetype
                            .edges_mut()
                            .cache_archetype_after_bundle_take(self.id(), None);
                        return None;
                    }
                }

                // Sort removed components so we can do an efficient "sorted remove".
                // Archetype components are already sorted.
                removed_table_components.sort_unstable();
                removed_sparse_set_components.sort_unstable();
                next_table_components = current_archetype.table_components().collect();
                next_sparse_set_components = current_archetype.sparse_set_components().collect();
                sorted_remove(&mut next_table_components, &removed_table_components);
                sorted_remove(
                    &mut next_sparse_set_components,
                    &removed_sparse_set_components,
                );

                next_table_id = if removed_table_components.is_empty() {
                    current_archetype.table_id()
                } else {
                    // SAFETY: all components in next_table_components exist
                    unsafe {
                        storages
                            .tables
                            .get_id_or_insert(&next_table_components, components)
                    }
                };
            }

            let new_archetype_id = archetypes.get_id_or_insert(
                components,
                observers,
                next_table_id,
                next_table_components,
                next_sparse_set_components,
            );
            Some(new_archetype_id)
        };
        let current_archetype = &mut archetypes[archetype_id];
        // Cache the result in an edge.
        if intersection {
            current_archetype
                .edges_mut()
                .cache_archetype_after_bundle_remove(self.id(), result);
        } else {
            current_archetype
                .edges_mut()
                .cache_archetype_after_bundle_take(self.id(), result);
        }
        result
    }
}

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleInserter<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    archetype_after_insert: ConstNonNull<ArchetypeAfterBundleInsert>,
    table: NonNull<Table>,
    archetype: NonNull<Archetype>,
    archetype_move_type: ArchetypeMoveType,
    change_tick: Tick,
}

/// The type of archetype move (or lack thereof) that will result from a bundle
/// being inserted into an entity.
pub(crate) enum ArchetypeMoveType {
    /// If the entity already has all of the components that are being inserted,
    /// its archetype won't change.
    SameArchetype,
    /// If only [`sparse set`](StorageType::SparseSet) components are being added,
    /// the entity's archetype will change while keeping the same table.
    NewArchetypeSameTable { new_archetype: NonNull<Archetype> },
    /// If any [`table-stored`](StorageType::Table) components are being added,
    /// both the entity's archetype and table will change.
    NewArchetypeNewTable {
        new_archetype: NonNull<Archetype>,
        new_table: NonNull<Table>,
    },
}

impl<'w> BundleInserter<'w> {
    #[inline]
    pub(crate) fn new<T: Bundle>(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        change_tick: Tick,
    ) -> Self {
        // SAFETY: These come from the same world. `world.components_registrator` can't be used since we borrow other fields too.
        let mut registrator =
            unsafe { ComponentsRegistrator::new(&mut world.components, &mut world.component_ids) };
        let bundle_id = world
            .bundles
            .register_info::<T>(&mut registrator, &mut world.storages);
        // SAFETY: We just ensured this bundle exists
        unsafe { Self::new_with_id(world, archetype_id, bundle_id, change_tick) }
    }

    /// Creates a new [`BundleInserter`].
    ///
    /// # Safety
    /// - Caller must ensure that `bundle_id` exists in `world.bundles`.
    #[inline]
    pub(crate) unsafe fn new_with_id(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        bundle_id: BundleId,
        change_tick: Tick,
    ) -> Self {
        // SAFETY: We will not make any accesses to the command queue, component or resource data of this world
        let bundle_info = world.bundles.get_unchecked(bundle_id);
        let bundle_id = bundle_info.id();
        let new_archetype_id = bundle_info.insert_bundle_into_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            &world.observers,
            archetype_id,
        );
        if new_archetype_id == archetype_id {
            let archetype = &mut world.archetypes[archetype_id];
            // SAFETY: The edge is assured to be initialized when we called insert_bundle_into_archetype
            let archetype_after_insert = unsafe {
                archetype
                    .edges()
                    .get_archetype_after_bundle_insert_internal(bundle_id)
                    .debug_checked_unwrap()
            };
            let table_id = archetype.table_id();
            let table = &mut world.storages.tables[table_id];
            Self {
                archetype_after_insert: archetype_after_insert.into(),
                archetype: archetype.into(),
                bundle_info: bundle_info.into(),
                table: table.into(),
                archetype_move_type: ArchetypeMoveType::SameArchetype,
                change_tick,
                world: world.as_unsafe_world_cell(),
            }
        } else {
            let (archetype, new_archetype) =
                world.archetypes.get_2_mut(archetype_id, new_archetype_id);
            // SAFETY: The edge is assured to be initialized when we called insert_bundle_into_archetype
            let archetype_after_insert = unsafe {
                archetype
                    .edges()
                    .get_archetype_after_bundle_insert_internal(bundle_id)
                    .debug_checked_unwrap()
            };
            let table_id = archetype.table_id();
            let new_table_id = new_archetype.table_id();
            if table_id == new_table_id {
                let table = &mut world.storages.tables[table_id];
                Self {
                    archetype_after_insert: archetype_after_insert.into(),
                    archetype: archetype.into(),
                    bundle_info: bundle_info.into(),
                    table: table.into(),
                    archetype_move_type: ArchetypeMoveType::NewArchetypeSameTable {
                        new_archetype: new_archetype.into(),
                    },
                    change_tick,
                    world: world.as_unsafe_world_cell(),
                }
            } else {
                let (table, new_table) = world.storages.tables.get_2_mut(table_id, new_table_id);
                Self {
                    archetype_after_insert: archetype_after_insert.into(),
                    archetype: archetype.into(),
                    bundle_info: bundle_info.into(),
                    table: table.into(),
                    archetype_move_type: ArchetypeMoveType::NewArchetypeNewTable {
                        new_archetype: new_archetype.into(),
                        new_table: new_table.into(),
                    },
                    change_tick,
                    world: world.as_unsafe_world_cell(),
                }
            }
        }
    }

    /// # Safety
    /// `entity` must currently exist in the source archetype for this inserter. `location`
    /// must be `entity`'s location in the archetype. `T` must match this [`BundleInfo`]'s type
    #[inline]
    pub(crate) unsafe fn insert<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        location: EntityLocation,
        bundle: T,
        insert_mode: InsertMode,
        caller: MaybeLocation,
        relationship_hook_mode: RelationshipHookMode,
    ) -> (EntityLocation, T::Effect) {
        let bundle_info = self.bundle_info.as_ref();
        let archetype_after_insert = self.archetype_after_insert.as_ref();
        let archetype = self.archetype.as_ref();

        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            // SAFETY: Mutable references do not alias and will be dropped after this block
            let mut deferred_world = self.world.into_deferred();

            if insert_mode == InsertMode::Replace {
                if archetype.has_replace_observer() {
                    deferred_world.trigger_observers(
                        ON_REPLACE,
                        entity,
                        archetype_after_insert.iter_existing(),
                        caller,
                    );
                }
                deferred_world.trigger_on_replace(
                    archetype,
                    entity,
                    archetype_after_insert.iter_existing(),
                    caller,
                    relationship_hook_mode,
                );
            }
        }

        let table = self.table.as_mut();

        // SAFETY: Archetype gets borrowed when running the on_replace observers above,
        // so this reference can only be promoted from shared to &mut down here, after they have been ran
        let archetype = self.archetype.as_mut();

        let (new_archetype, new_location, after_effect) = match &mut self.archetype_move_type {
            ArchetypeMoveType::SameArchetype => {
                // SAFETY: Mutable references do not alias and will be dropped after this block
                let sparse_sets = {
                    let world = self.world.world_mut();
                    &mut world.storages.sparse_sets
                };

                let after_effect = bundle_info.write_components(
                    table,
                    sparse_sets,
                    archetype_after_insert,
                    archetype_after_insert.required_components.iter(),
                    entity,
                    location.table_row,
                    self.change_tick,
                    bundle,
                    insert_mode,
                    caller,
                );

                (archetype, location, after_effect)
            }
            ArchetypeMoveType::NewArchetypeSameTable { new_archetype } => {
                let new_archetype = new_archetype.as_mut();

                // SAFETY: Mutable references do not alias and will be dropped after this block
                let (sparse_sets, entities) = {
                    let world = self.world.world_mut();
                    (&mut world.storages.sparse_sets, &mut world.entities)
                };

                let result = archetype.swap_remove(location.archetype_row);
                if let Some(swapped_entity) = result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { entities.get(swapped_entity).debug_checked_unwrap() };
                    entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        },
                    );
                }
                let new_location = new_archetype.allocate(entity, result.table_row);
                entities.set(entity.index(), new_location);
                let after_effect = bundle_info.write_components(
                    table,
                    sparse_sets,
                    archetype_after_insert,
                    archetype_after_insert.required_components.iter(),
                    entity,
                    result.table_row,
                    self.change_tick,
                    bundle,
                    insert_mode,
                    caller,
                );

                (new_archetype, new_location, after_effect)
            }
            ArchetypeMoveType::NewArchetypeNewTable {
                new_archetype,
                new_table,
            } => {
                let new_table = new_table.as_mut();
                let new_archetype = new_archetype.as_mut();

                // SAFETY: Mutable references do not alias and will be dropped after this block
                let (archetypes_ptr, sparse_sets, entities) = {
                    let world = self.world.world_mut();
                    let archetype_ptr: *mut Archetype = world.archetypes.archetypes.as_mut_ptr();
                    (
                        archetype_ptr,
                        &mut world.storages.sparse_sets,
                        &mut world.entities,
                    )
                };
                let result = archetype.swap_remove(location.archetype_row);
                if let Some(swapped_entity) = result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { entities.get(swapped_entity).debug_checked_unwrap() };
                    entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        },
                    );
                }
                // PERF: store "non bundle" components in edge, then just move those to avoid
                // redundant copies
                let move_result = table.move_to_superset_unchecked(result.table_row, new_table);
                let new_location = new_archetype.allocate(entity, move_result.new_row);
                entities.set(entity.index(), new_location);

                // If an entity was moved into this entity's table spot, update its table row.
                if let Some(swapped_entity) = move_result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { entities.get(swapped_entity).debug_checked_unwrap() };

                    entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: swapped_location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: result.table_row,
                        },
                    );

                    if archetype.id() == swapped_location.archetype_id {
                        archetype
                            .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                    } else if new_archetype.id() == swapped_location.archetype_id {
                        new_archetype
                            .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                    } else {
                        // SAFETY: the only two borrowed archetypes are above and we just did collision checks
                        (*archetypes_ptr.add(swapped_location.archetype_id.index()))
                            .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                    }
                }

                let after_effect = bundle_info.write_components(
                    new_table,
                    sparse_sets,
                    archetype_after_insert,
                    archetype_after_insert.required_components.iter(),
                    entity,
                    move_result.new_row,
                    self.change_tick,
                    bundle,
                    insert_mode,
                    caller,
                );

                (new_archetype, new_location, after_effect)
            }
        };

        let new_archetype = &*new_archetype;
        // SAFETY: We have no outstanding mutable references to world as they were dropped
        let mut deferred_world = unsafe { self.world.into_deferred() };

        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            deferred_world.trigger_on_add(
                new_archetype,
                entity,
                archetype_after_insert.iter_added(),
                caller,
            );
            if new_archetype.has_add_observer() {
                deferred_world.trigger_observers(
                    ON_ADD,
                    entity,
                    archetype_after_insert.iter_added(),
                    caller,
                );
            }
            match insert_mode {
                InsertMode::Replace => {
                    // Insert triggers for both new and existing components if we're replacing them.
                    deferred_world.trigger_on_insert(
                        new_archetype,
                        entity,
                        archetype_after_insert.iter_inserted(),
                        caller,
                        relationship_hook_mode,
                    );
                    if new_archetype.has_insert_observer() {
                        deferred_world.trigger_observers(
                            ON_INSERT,
                            entity,
                            archetype_after_insert.iter_inserted(),
                            caller,
                        );
                    }
                }
                InsertMode::Keep => {
                    // Insert triggers only for new components if we're not replacing them (since
                    // nothing is actually inserted).
                    deferred_world.trigger_on_insert(
                        new_archetype,
                        entity,
                        archetype_after_insert.iter_added(),
                        caller,
                        relationship_hook_mode,
                    );
                    if new_archetype.has_insert_observer() {
                        deferred_world.trigger_observers(
                            ON_INSERT,
                            entity,
                            archetype_after_insert.iter_added(),
                            caller,
                        );
                    }
                }
            }
        }

        (new_location, after_effect)
    }

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        // SAFETY: No outstanding references to self.world, changes to entities cannot invalidate our internal pointers
        unsafe { &mut self.world.world_mut().entities }
    }
}

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleSpawner<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    table: NonNull<Table>,
    archetype: NonNull<Archetype>,
    change_tick: Tick,
}

impl<'w> BundleSpawner<'w> {
    #[inline]
    pub fn new<T: Bundle>(world: &'w mut World, change_tick: Tick) -> Self {
        // SAFETY: These come from the same world. `world.components_registrator` can't be used since we borrow other fields too.
        let mut registrator =
            unsafe { ComponentsRegistrator::new(&mut world.components, &mut world.component_ids) };
        let bundle_id = world
            .bundles
            .register_info::<T>(&mut registrator, &mut world.storages);
        // SAFETY: we initialized this bundle_id in `init_info`
        unsafe { Self::new_with_id(world, bundle_id, change_tick) }
    }

    /// Creates a new [`BundleSpawner`].
    ///
    /// # Safety
    /// Caller must ensure that `bundle_id` exists in `world.bundles`
    #[inline]
    pub(crate) unsafe fn new_with_id(
        world: &'w mut World,
        bundle_id: BundleId,
        change_tick: Tick,
    ) -> Self {
        let bundle_info = world.bundles.get_unchecked(bundle_id);
        let new_archetype_id = bundle_info.insert_bundle_into_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            &world.observers,
            ArchetypeId::EMPTY,
        );
        let archetype = &mut world.archetypes[new_archetype_id];
        let table = &mut world.storages.tables[archetype.table_id()];
        Self {
            bundle_info: bundle_info.into(),
            table: table.into(),
            archetype: archetype.into(),
            change_tick,
            world: world.as_unsafe_world_cell(),
        }
    }

    #[inline]
    pub fn reserve_storage(&mut self, additional: usize) {
        // SAFETY: There are no outstanding world references
        let (archetype, table) = unsafe { (self.archetype.as_mut(), self.table.as_mut()) };
        archetype.reserve(additional);
        table.reserve(additional);
    }

    /// # Safety
    /// `entity` must be allocated (but non-existent), `T` must match this [`BundleInfo`]'s type
    #[inline]
    #[track_caller]
    pub unsafe fn spawn_non_existent<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        bundle: T,
        caller: MaybeLocation,
    ) -> (EntityLocation, T::Effect) {
        // SAFETY: We do not make any structural changes to the archetype graph through self.world so these pointers always remain valid
        let bundle_info = self.bundle_info.as_ref();
        let (location, after_effect) = {
            let table = self.table.as_mut();
            let archetype = self.archetype.as_mut();

            // SAFETY: Mutable references do not alias and will be dropped after this block
            let (sparse_sets, entities) = {
                let world = self.world.world_mut();
                (&mut world.storages.sparse_sets, &mut world.entities)
            };
            let table_row = table.allocate(entity);
            let location = archetype.allocate(entity, table_row);
            let after_effect = bundle_info.write_components(
                table,
                sparse_sets,
                &SpawnBundleStatus,
                bundle_info.required_components.iter(),
                entity,
                table_row,
                self.change_tick,
                bundle,
                InsertMode::Replace,
                caller,
            );
            entities.set(entity.index(), location);
            (location, after_effect)
        };

        // SAFETY: We have no outstanding mutable references to world as they were dropped
        let mut deferred_world = unsafe { self.world.into_deferred() };
        // SAFETY: `DeferredWorld` cannot provide mutable access to `Archetypes`.
        let archetype = self.archetype.as_ref();
        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            deferred_world.trigger_on_add(
                archetype,
                entity,
                bundle_info.iter_contributed_components(),
                caller,
            );
            if archetype.has_add_observer() {
                deferred_world.trigger_observers(
                    ON_ADD,
                    entity,
                    bundle_info.iter_contributed_components(),
                    caller,
                );
            }
            deferred_world.trigger_on_insert(
                archetype,
                entity,
                bundle_info.iter_contributed_components(),
                caller,
                RelationshipHookMode::Run,
            );
            if archetype.has_insert_observer() {
                deferred_world.trigger_observers(
                    ON_INSERT,
                    entity,
                    bundle_info.iter_contributed_components(),
                    caller,
                );
            }
        };

        (location, after_effect)
    }

    /// # Safety
    /// `T` must match this [`BundleInfo`]'s type
    #[inline]
    pub unsafe fn spawn<T: Bundle>(
        &mut self,
        bundle: T,
        caller: MaybeLocation,
    ) -> (Entity, T::Effect) {
        let entity = self.entities().alloc();
        // SAFETY: entity is allocated (but non-existent), `T` matches this BundleInfo's type
        let (_, after_effect) = unsafe { self.spawn_non_existent(entity, bundle, caller) };
        (entity, after_effect)
    }

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        // SAFETY: No outstanding references to self.world, changes to entities cannot invalidate our internal pointers
        unsafe { &mut self.world.world_mut().entities }
    }

    /// # Safety
    /// - `Self` must be dropped after running this function as it may invalidate internal pointers.
    #[inline]
    pub(crate) unsafe fn flush_commands(&mut self) {
        // SAFETY: pointers on self can be invalidated,
        self.world.world_mut().flush();
    }
}

/// Metadata for bundles. Stores a [`BundleInfo`] for each type of [`Bundle`] in a given world.
#[derive(Default)]
pub struct Bundles {
    bundle_infos: Vec<BundleInfo>,
    /// Cache static [`BundleId`]
    bundle_ids: TypeIdMap<BundleId>,
    /// Cache bundles, which contains both explicit and required components of [`Bundle`]
    contributed_bundle_ids: TypeIdMap<BundleId>,
    /// Cache dynamic [`BundleId`] with multiple components
    dynamic_bundle_ids: HashMap<Box<[ComponentId]>, BundleId>,
    dynamic_bundle_storages: HashMap<BundleId, Vec<StorageType>>,
    /// Cache optimized dynamic [`BundleId`] with single component
    dynamic_component_bundle_ids: HashMap<ComponentId, BundleId>,
    dynamic_component_storages: HashMap<BundleId, StorageType>,
}

impl Bundles {
    /// The total number of [`Bundle`] registered in [`Storages`].
    pub fn len(&self) -> usize {
        self.bundle_infos.len()
    }

    /// Returns true if no [`Bundle`] registered in [`Storages`].
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over [`BundleInfo`].
    pub fn iter(&self) -> impl Iterator<Item = &BundleInfo> {
        self.bundle_infos.iter()
    }

    /// Gets the metadata associated with a specific type of bundle.
    /// Returns `None` if the bundle is not registered with the world.
    #[inline]
    pub fn get(&self, bundle_id: BundleId) -> Option<&BundleInfo> {
        self.bundle_infos.get(bundle_id.index())
    }

    /// Gets the value identifying a specific type of bundle.
    /// Returns `None` if the bundle does not exist in the world,
    /// or if `type_id` does not correspond to a type of bundle.
    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<BundleId> {
        self.bundle_ids.get(&type_id).cloned()
    }

    /// Registers a new [`BundleInfo`] for a statically known type.
    ///
    /// Also registers all the components in the bundle.
    pub(crate) fn register_info<T: Bundle>(
        &mut self,
        components: &mut ComponentsRegistrator,
        storages: &mut Storages,
    ) -> BundleId {
        let bundle_infos = &mut self.bundle_infos;
        *self.bundle_ids.entry(TypeId::of::<T>()).or_insert_with(|| {
            let mut component_ids= Vec::new();
            T::component_ids(components, &mut |id| component_ids.push(id));
            let id = BundleId(bundle_infos.len());
            let bundle_info =
                // SAFETY: T::component_id ensures:
                // - its info was created
                // - appropriate storage for it has been initialized.
                // - it was created in the same order as the components in T
                unsafe { BundleInfo::new(core::any::type_name::<T>(), storages, components, component_ids, id) };
            bundle_infos.push(bundle_info);
            id
        })
    }

    /// Registers a new [`BundleInfo`], which contains both explicit and required components for a statically known type.
    ///
    /// Also registers all the components in the bundle.
    pub(crate) fn register_contributed_bundle_info<T: Bundle>(
        &mut self,
        components: &mut ComponentsRegistrator,
        storages: &mut Storages,
    ) -> BundleId {
        if let Some(id) = self.contributed_bundle_ids.get(&TypeId::of::<T>()).cloned() {
            id
        } else {
            let explicit_bundle_id = self.register_info::<T>(components, storages);
            // SAFETY: reading from `explicit_bundle_id` and creating new bundle in same time. Its valid because bundle hashmap allow this
            let id = unsafe {
                let (ptr, len) = {
                    // SAFETY: `explicit_bundle_id` is valid and defined above
                    let contributed = self
                        .get_unchecked(explicit_bundle_id)
                        .contributed_components();
                    (contributed.as_ptr(), contributed.len())
                };
                // SAFETY: this is sound because the contributed_components Vec for explicit_bundle_id will not be accessed mutably as
                // part of init_dynamic_info. No mutable references will be created and the allocation will remain valid.
                self.init_dynamic_info(storages, components, core::slice::from_raw_parts(ptr, len))
            };
            self.contributed_bundle_ids.insert(TypeId::of::<T>(), id);
            id
        }
    }

    /// # Safety
    /// A [`BundleInfo`] with the given [`BundleId`] must have been initialized for this instance of `Bundles`.
    pub(crate) unsafe fn get_unchecked(&self, id: BundleId) -> &BundleInfo {
        self.bundle_infos.get_unchecked(id.0)
    }

    /// # Safety
    /// This [`BundleId`] must have been initialized with a single [`Component`] (via [`init_component_info`](Self::init_dynamic_info))
    pub(crate) unsafe fn get_storage_unchecked(&self, id: BundleId) -> StorageType {
        *self
            .dynamic_component_storages
            .get(&id)
            .debug_checked_unwrap()
    }

    /// # Safety
    /// This [`BundleId`] must have been initialized with multiple [`Component`]s (via [`init_dynamic_info`](Self::init_dynamic_info))
    pub(crate) unsafe fn get_storages_unchecked(&mut self, id: BundleId) -> &mut Vec<StorageType> {
        self.dynamic_bundle_storages
            .get_mut(&id)
            .debug_checked_unwrap()
    }

    /// Initializes a new [`BundleInfo`] for a dynamic [`Bundle`].
    ///
    /// # Panics
    ///
    /// Panics if any of the provided [`ComponentId`]s do not exist in the
    /// provided [`Components`].
    pub(crate) fn init_dynamic_info(
        &mut self,
        storages: &mut Storages,
        components: &Components,
        component_ids: &[ComponentId],
    ) -> BundleId {
        let bundle_infos = &mut self.bundle_infos;

        // Use `raw_entry_mut` to avoid cloning `component_ids` to access `Entry`
        let (_, bundle_id) = self
            .dynamic_bundle_ids
            .raw_entry_mut()
            .from_key(component_ids)
            .or_insert_with(|| {
                let (id, storages) = initialize_dynamic_bundle(
                    bundle_infos,
                    storages,
                    components,
                    Vec::from(component_ids),
                );
                // SAFETY: The ID always increases when new bundles are added, and so, the ID is unique.
                unsafe {
                    self.dynamic_bundle_storages
                        .insert_unique_unchecked(id, storages);
                }
                (component_ids.into(), id)
            });
        *bundle_id
    }

    /// Initializes a new [`BundleInfo`] for a dynamic [`Bundle`] with single component.
    ///
    /// # Panics
    ///
    /// Panics if the provided [`ComponentId`] does not exist in the provided [`Components`].
    pub(crate) fn init_component_info(
        &mut self,
        storages: &mut Storages,
        components: &Components,
        component_id: ComponentId,
    ) -> BundleId {
        let bundle_infos = &mut self.bundle_infos;
        let bundle_id = self
            .dynamic_component_bundle_ids
            .entry(component_id)
            .or_insert_with(|| {
                let (id, storage_type) = initialize_dynamic_bundle(
                    bundle_infos,
                    storages,
                    components,
                    vec![component_id],
                );
                self.dynamic_component_storages.insert(id, storage_type[0]);
                id
            });
        *bundle_id
    }
}

/// Asserts that all components are part of [`Components`]
/// and initializes a [`BundleInfo`].
fn initialize_dynamic_bundle(
    bundle_infos: &mut Vec<BundleInfo>,
    storages: &mut Storages,
    components: &Components,
    component_ids: Vec<ComponentId>,
) -> (BundleId, Vec<StorageType>) {
    // Assert component existence
    let storage_types = component_ids.iter().map(|&id| {
        components.get_info(id).unwrap_or_else(|| {
            panic!(
                "init_dynamic_info called with component id {id:?} which doesn't exist in this world"
            )
        }).storage_type()
    }).collect();

    let id = BundleId(bundle_infos.len());
    let bundle_info =
        // SAFETY: `component_ids` are valid as they were just checked
        unsafe { BundleInfo::new("<dynamic bundle>", storages, components, component_ids, id) };
    bundle_infos.push(bundle_info);

    (id, storage_types)
}

fn sorted_remove<T: Eq + Ord + Copy>(source: &mut Vec<T>, remove: &[T]) {
    let mut remove_index = 0;
    source.retain(|value| {
        while remove_index < remove.len() && *value > remove[remove_index] {
            remove_index += 1;
        }

        if remove_index < remove.len() {
            *value != remove[remove_index]
        } else {
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::{component::HookContext, prelude::*, world::DeferredWorld};
    use alloc::vec;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    #[component(on_add = a_on_add, on_insert = a_on_insert, on_replace = a_on_replace, on_remove = a_on_remove)]
    struct AMacroHooks;

    fn a_on_add(mut world: DeferredWorld, _: HookContext) {
        world.resource_mut::<R>().assert_order(0);
    }

    fn a_on_insert(mut world: DeferredWorld, _: HookContext) {
        world.resource_mut::<R>().assert_order(1);
    }

    fn a_on_replace(mut world: DeferredWorld, _: HookContext) {
        world.resource_mut::<R>().assert_order(2);
    }

    fn a_on_remove(mut world: DeferredWorld, _: HookContext) {
        world.resource_mut::<R>().assert_order(3);
    }

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(Component)]
    struct D;

    #[derive(Component, Eq, PartialEq, Debug)]
    struct V(&'static str); // component with a value

    #[derive(Resource, Default)]
    struct R(usize);

    impl R {
        #[track_caller]
        fn assert_order(&mut self, count: usize) {
            assert_eq!(count, self.0);
            self.0 += 1;
        }
    }

    #[test]
    fn component_hook_order_spawn_despawn() {
        let mut world = World::new();
        world.init_resource::<R>();
        world
            .register_component_hooks::<A>()
            .on_add(|mut world, _| world.resource_mut::<R>().assert_order(0))
            .on_insert(|mut world, _| world.resource_mut::<R>().assert_order(1))
            .on_replace(|mut world, _| world.resource_mut::<R>().assert_order(2))
            .on_remove(|mut world, _| world.resource_mut::<R>().assert_order(3));

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(4, world.resource::<R>().0);
    }

    #[test]
    fn component_hook_order_spawn_despawn_with_macro_hooks() {
        let mut world = World::new();
        world.init_resource::<R>();

        let entity = world.spawn(AMacroHooks).id();
        world.despawn(entity);

        assert_eq!(4, world.resource::<R>().0);
    }

    #[test]
    fn component_hook_order_insert_remove() {
        let mut world = World::new();
        world.init_resource::<R>();
        world
            .register_component_hooks::<A>()
            .on_add(|mut world, _| world.resource_mut::<R>().assert_order(0))
            .on_insert(|mut world, _| world.resource_mut::<R>().assert_order(1))
            .on_replace(|mut world, _| world.resource_mut::<R>().assert_order(2))
            .on_remove(|mut world, _| world.resource_mut::<R>().assert_order(3));

        let mut entity = world.spawn_empty();
        entity.insert(A);
        entity.remove::<A>();
        entity.flush();
        assert_eq!(4, world.resource::<R>().0);
    }

    #[test]
    fn component_hook_order_replace() {
        let mut world = World::new();
        world
            .register_component_hooks::<A>()
            .on_replace(|mut world, _| world.resource_mut::<R>().assert_order(0))
            .on_insert(|mut world, _| {
                if let Some(mut r) = world.get_resource_mut::<R>() {
                    r.assert_order(1);
                }
            });

        let entity = world.spawn(A).id();
        world.init_resource::<R>();
        let mut entity = world.entity_mut(entity);
        entity.insert(A);
        entity.insert_if_new(A); // this will not trigger on_replace or on_insert
        entity.flush();
        assert_eq!(2, world.resource::<R>().0);
    }

    #[test]
    fn component_hook_order_recursive() {
        let mut world = World::new();
        world.init_resource::<R>();
        world
            .register_component_hooks::<A>()
            .on_add(|mut world, context| {
                world.resource_mut::<R>().assert_order(0);
                world.commands().entity(context.entity).insert(B);
            })
            .on_remove(|mut world, context| {
                world.resource_mut::<R>().assert_order(2);
                world.commands().entity(context.entity).remove::<B>();
            });

        world
            .register_component_hooks::<B>()
            .on_add(|mut world, context| {
                world.resource_mut::<R>().assert_order(1);
                world.commands().entity(context.entity).remove::<A>();
            })
            .on_remove(|mut world, _| {
                world.resource_mut::<R>().assert_order(3);
            });

        let entity = world.spawn(A).flush();
        let entity = world.get_entity(entity).unwrap();
        assert!(!entity.contains::<A>());
        assert!(!entity.contains::<B>());
        assert_eq!(4, world.resource::<R>().0);
    }

    #[test]
    fn component_hook_order_recursive_multiple() {
        let mut world = World::new();
        world.init_resource::<R>();
        world
            .register_component_hooks::<A>()
            .on_add(|mut world, context| {
                world.resource_mut::<R>().assert_order(0);
                world.commands().entity(context.entity).insert(B).insert(C);
            });

        world
            .register_component_hooks::<B>()
            .on_add(|mut world, context| {
                world.resource_mut::<R>().assert_order(1);
                world.commands().entity(context.entity).insert(D);
            });

        world
            .register_component_hooks::<C>()
            .on_add(|mut world, _| {
                world.resource_mut::<R>().assert_order(3);
            });

        world
            .register_component_hooks::<D>()
            .on_add(|mut world, _| {
                world.resource_mut::<R>().assert_order(2);
            });

        world.spawn(A).flush();
        assert_eq!(4, world.resource::<R>().0);
    }

    #[test]
    fn insert_if_new() {
        let mut world = World::new();
        let id = world.spawn(V("one")).id();
        let mut entity = world.entity_mut(id);
        entity.insert_if_new(V("two"));
        entity.insert_if_new((A, V("three")));
        entity.flush();
        // should still contain "one"
        let entity = world.entity(id);
        assert!(entity.contains::<A>());
        assert_eq!(entity.get(), Some(&V("one")));
    }

    #[test]
    fn sorted_remove() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7];
        let b = vec![1, 2, 3, 5, 7];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![4, 6]);

        let mut a = vec![1];
        let b = vec![1];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![]);

        let mut a = vec![1];
        let b = vec![2];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![1]);
    }
}
