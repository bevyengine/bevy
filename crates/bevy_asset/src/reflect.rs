use alloc::boxed::Box;
use bevy_platform::collections::hash_map::Keys;
use core::{
    any::{Any, TypeId},
    iter::empty,
};

use bevy_ecs::{
    archetype::{ArchetypeEntity, ArchetypeId, ArchetypeRecord},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect};

use crate::{
    Asset, AssetData, AssetEntity, AssetUuidMap, DirectAssetAccessExt, Handle, InsertAssetError,
    UntypedAssetId, UntypedHandle,
};

/// Type data for the [`TypeRegistry`](bevy_reflect::TypeRegistry) used to operate on reflected [`Asset`]s.
///
/// This type provides similar methods to [`Assets`](crate::Assets),
/// [`AssetsMut`](crate::AssetsMut), and [`AssetCommands`](crate::AssetCommands), like
/// [`get`](ReflectAsset::get),
/// [`spawn`](ReflectAsset::spawn), and [`remove`](ReflectAsset::remove), but can be used in
/// situations where you don't know which asset type `T` you want until runtime.
///
/// [`ReflectAsset`] can be obtained via
/// [`TypeRegistration::data`](bevy_reflect::TypeRegistration::data) if the asset was registered
/// using [`register_asset_reflect`](crate::AssetApp::register_asset_reflect).
#[derive(Clone)]
pub struct ReflectAsset {
    handle_type_id: TypeId,
    asset_data_type_id: TypeId,

    get: fn(&World, UntypedAssetId) -> Option<&dyn Reflect>,
    // SAFETY:
    // - may only be called with an [`UnsafeWorldCell`] which can be used to read `AssetUuidMap`
    //   resource, and the `AssetData` component mutably
    // - may only be used to access **at most one** access at once
    get_unchecked_mut: unsafe fn(UnsafeWorldCell<'_>, UntypedAssetId) -> Option<&mut dyn Reflect>,
    spawn: fn(&mut World, &dyn PartialReflect) -> UntypedHandle,
    insert: fn(&mut World, UntypedAssetId, &dyn PartialReflect) -> Result<(), InsertAssetError>,
    count: fn(&World) -> usize,
    ids: for<'w> fn(&'w World) -> Box<dyn Iterator<Item = UntypedAssetId> + 'w>,
    remove: fn(&mut World, UntypedAssetId) -> Option<Box<dyn Reflect>>,
}

impl ReflectAsset {
    /// The [`TypeId`] of the [`Handle<T>`] for this asset
    pub fn handle_type_id(&self) -> TypeId {
        self.handle_type_id
    }

    /// The [`TypeId`] of the [`AssetData<T>`] component.
    pub fn asset_data_type_id(&self) -> TypeId {
        self.asset_data_type_id
    }

    /// Equivalent of [`Assets::get`](crate::Assets::get)
    pub fn get<'w>(
        &self,
        world: &'w World,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<&'w dyn Reflect> {
        (self.get)(world, asset_id.into())
    }

    /// Equivalent of [`AssetsMut::get_mut`](crate::AssetsMut::get)
    pub fn get_mut<'w>(
        &self,
        world: &'w mut World,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<&'w mut dyn Reflect> {
        #[expect(
            unsafe_code,
            reason = "Use of unsafe `Self::get_unchecked_mut()` function."
        )]
        // SAFETY: We have exclusive access to the whole world, which includes all
        unsafe {
            (self.get_unchecked_mut)(world.as_unsafe_world_cell(), asset_id.into())
        }
    }

    /// Equivalent of [`AssetsMut::get_mut`](crate::AssetsMut::get_mut), but works with an
    /// [`UnsafeWorldCell`].
    ///
    /// Only use this method when you have ensured that you are the *only* one with access to the
    /// [`AssetData`] of the asset type. Furthermore, this does *not* allow you to have look up two
    /// distinct handles, you can only have at most one alive at the same time. This means that this
    /// is *not allowed*:
    /// ```no_run
    /// # use bevy_asset::{ReflectAsset, UntypedHandle};
    /// # use bevy_ecs::prelude::World;
    /// # let reflect_asset: ReflectAsset = unimplemented!();
    /// # let mut world: World = unimplemented!();
    /// # let handle_1: UntypedHandle = unimplemented!();
    /// # let handle_2: UntypedHandle = unimplemented!();
    /// let unsafe_world_cell = world.as_unsafe_world_cell();
    /// let a = unsafe { reflect_asset.get_unchecked_mut(unsafe_world_cell, &handle_1).unwrap() };
    /// let b = unsafe { reflect_asset.get_unchecked_mut(unsafe_world_cell, &handle_2).unwrap() };
    /// // ^ not allowed, two mutable references through the same asset resource, even though the
    /// // handles are distinct
    ///
    /// println!("a = {a:?}, b = {b:?}");
    /// ```
    ///
    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method if you know that the [`UnsafeWorldCell`] may be used to access the
    ///   corresponding [`AssetData`].
    /// * Don't call this method more than once in the same scope.
    #[expect(
        unsafe_code,
        reason = "This function calls unsafe code and has safety requirements."
    )]
    pub unsafe fn get_unchecked_mut<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<&'w mut dyn Reflect> {
        // SAFETY: requirements are deferred to the caller
        unsafe { (self.get_unchecked_mut)(world, asset_id.into()) }
    }

    /// Equivalent of [`DirectAssetAccessExt::spawn_asset`]
    pub fn spawn(&self, world: &mut World, value: &dyn PartialReflect) -> UntypedHandle {
        (self.spawn)(world, value)
    }
    /// Equivalent of [`DirectAssetAccessExt::insert_asset`]
    pub fn insert(
        &self,
        world: &mut World,
        asset_id: impl Into<UntypedAssetId>,
        value: &dyn PartialReflect,
    ) -> Result<(), InsertAssetError> {
        (self.insert)(world, asset_id.into(), value)
    }

    /// Equivalent of [`DirectAssetAccessExt::remove_asset`]
    pub fn remove(
        &self,
        world: &mut World,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<Box<dyn Reflect>> {
        (self.remove)(world, asset_id.into())
    }

    /// Equivalent of [`Assets::count`](crate::Assets::count)
    pub fn count(&self, world: &World) -> usize {
        (self.count)(world)
    }

    /// Equivalent of [`Assets::is_empty`](crate::Assets::is_empty)
    pub fn is_empty(&self, world: &World) -> bool {
        self.count(world) == 0
    }

    /// Similar to [`Assets::iter`](crate::Assets::iter).
    pub fn ids<'w>(&self, world: &'w World) -> impl Iterator<Item = UntypedAssetId> + 'w {
        (self.ids)(world)
    }
}

impl<A: Asset + FromReflect> FromType<A> for ReflectAsset {
    fn from_type() -> Self {
        ReflectAsset {
            handle_type_id: TypeId::of::<Handle<A>>(),
            asset_data_type_id: TypeId::of::<AssetData<A>>(),
            get: |world, asset_id| {
                world
                    .get_asset::<A>(asset_id.typed_debug_checked())
                    .map(|asset| asset as &dyn Reflect)
            },
            get_unchecked_mut: |world, asset_id| {
                #[expect(
                    unsafe_code,
                    reason = "We are providing an abstraction over UnsafeWorldCell methods"
                )]
                // SAFETY: Caller ensures we have access to `AssetUuidMap`.
                let entity = unsafe { world.get_resource::<AssetUuidMap>() }
                    .unwrap()
                    .resolve_entity(asset_id)
                    .ok()?;
                let entity = world.get_entity(entity.raw_entity()).ok()?;
                #[expect(
                    unsafe_code,
                    reason = "We are providing an abstraction over UnsafeWorldCell methods"
                )]
                // SAFETY: Caller ensures we have access to the asset data on this entity, and
                // ensures we only have at most one reference to this asset.
                let data = unsafe { entity.get_mut::<AssetData<A>>() }?.into_inner();
                Some(&mut data.0 as _)
            },
            spawn: |world, value| {
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::add`");
                world.spawn_asset(value).untyped()
            },
            insert: |world, asset_id, value| {
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::set`");
                world.insert_asset(asset_id.typed_debug_checked(), value)
            },
            count: |world| {
                let Some(component_id) = world.components().get_id(TypeId::of::<AssetData<A>>())
                else {
                    return 0;
                };
                let Some(archetypes) = world.archetypes().component_index().get(&component_id)
                else {
                    return 0;
                };
                archetypes
                    .keys()
                    .map(|id| world.archetypes().get(*id).unwrap())
                    .map(|archetype| archetype.entities().len())
                    .sum()
            },
            ids: |world| {
                let Some(component_id) = world.components().get_id(TypeId::of::<AssetData<A>>())
                else {
                    return Box::new(empty::<UntypedAssetId>());
                };
                let Some(archetypes) = world.archetypes().component_index().get(&component_id)
                else {
                    return Box::new(empty::<UntypedAssetId>());
                };
                let mut archetype_ids = archetypes.keys();
                let Some(first_id) = archetype_ids.next() else {
                    return Box::new(empty::<UntypedAssetId>());
                };
                let archetype = world.archetypes().get(*first_id).unwrap();
                let entities = archetype.entities().iter();

                struct AssetIdIter<'w> {
                    world: &'w World,
                    archetype_ids: Keys<'w, ArchetypeId, ArchetypeRecord>,
                    entities: core::slice::Iter<'w, ArchetypeEntity>,
                    type_id: TypeId,
                }

                impl Iterator for AssetIdIter<'_> {
                    type Item = UntypedAssetId;

                    fn next(&mut self) -> Option<Self::Item> {
                        // Loop until we either get an entity, or we run out of archetypes.
                        loop {
                            if let Some(archetype_entity) = self.entities.next() {
                                return Some(UntypedAssetId::Entity {
                                    type_id: self.type_id,
                                    entity: AssetEntity::new_unchecked(archetype_entity.id()),
                                });
                            }

                            // We ran out of entities in this archetype, so move on to the next
                            // archetype.
                            let archetype_id = self.archetype_ids.next()?;
                            let archetype = self.world.archetypes().get(*archetype_id).unwrap();
                            self.entities = archetype.entities().iter();
                        }
                    }
                }

                Box::new(AssetIdIter {
                    world,
                    archetype_ids,
                    entities,
                    type_id: TypeId::of::<A>(),
                })
            },
            remove: |world, asset_id| {
                world
                    .remove_asset::<A>(asset_id.typed_debug_checked())
                    .ok()
                    .map(|asset| Box::new(asset) as _)
            },
        }
    }
}

/// Reflect type data struct relating a [`Handle<T>`] back to the `T` asset type.
///
/// Say you want to look up the asset values of a list of handles when you have access to their `&dyn Reflect` form.
/// Assets can be looked up in the world using [`ReflectAsset`], but how do you determine which [`ReflectAsset`] to use when
/// only looking at the handle? [`ReflectHandle`] is stored in the type registry on each `Handle<T>` type, so you can use [`ReflectHandle::asset_type_id`] to look up
/// the [`ReflectAsset`] type data on the corresponding `T` asset type:
///
///
/// ```no_run
/// # use bevy_reflect::{TypeRegistry, prelude::*};
/// # use bevy_ecs::prelude::*;
/// use bevy_asset::{ReflectHandle, ReflectAsset};
///
/// # let world: &World = unimplemented!();
/// # let type_registry: TypeRegistry = unimplemented!();
/// let handles: Vec<&dyn Reflect> = unimplemented!();
/// for handle in handles {
///     let reflect_handle = type_registry.get_type_data::<ReflectHandle>(handle.type_id()).unwrap();
///     let reflect_asset = type_registry.get_type_data::<ReflectAsset>(reflect_handle.asset_type_id()).unwrap();
///
///     let handle = reflect_handle.downcast_handle_untyped(handle.as_any()).unwrap();
///     let value = reflect_asset.get(world, &handle).unwrap();
///     println!("{value:?}");
/// }
/// ```
#[derive(Clone)]
pub struct ReflectHandle {
    asset_type_id: TypeId,
    downcast_handle_untyped: fn(&dyn Any) -> Option<UntypedHandle>,
    typed: fn(UntypedHandle) -> Box<dyn Reflect>,
}

impl ReflectHandle {
    /// The [`TypeId`] of the asset
    pub fn asset_type_id(&self) -> TypeId {
        self.asset_type_id
    }

    /// A way to go from a [`Handle<T>`] in a `dyn Any` to a [`UntypedHandle`]
    pub fn downcast_handle_untyped(&self, handle: &dyn Any) -> Option<UntypedHandle> {
        (self.downcast_handle_untyped)(handle)
    }

    /// A way to go from a [`UntypedHandle`] to a [`Handle<T>`] in a `Box<dyn Reflect>`.
    /// Equivalent of [`UntypedHandle::typed`].
    pub fn typed(&self, handle: UntypedHandle) -> Box<dyn Reflect> {
        (self.typed)(handle)
    }
}

impl<A: Asset> FromType<Handle<A>> for ReflectHandle {
    fn from_type() -> Self {
        ReflectHandle {
            asset_type_id: TypeId::of::<A>(),
            downcast_handle_untyped: |handle: &dyn Any| {
                handle
                    .downcast_ref::<Handle<A>>()
                    .map(|h| h.clone().untyped())
            },
            typed: |handle: UntypedHandle| Box::new(handle.typed_debug_checked::<A>()),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec::Vec};
    use core::any::TypeId;

    use crate::{tests::create_app, Asset, AssetApp, ReflectAsset};
    use bevy_ecs::reflect::AppTypeRegistry;
    use bevy_reflect::Reflect;

    #[derive(Asset, Reflect)]
    struct AssetType {
        field: String,
    }

    #[test]
    fn test_reflect_asset_operations() {
        let mut app = create_app().0;
        app.init_asset::<AssetType>()
            .register_asset_reflect::<AssetType>();

        let reflect_asset = {
            let type_registry = app.world().resource::<AppTypeRegistry>();
            let type_registry = type_registry.read();

            type_registry
                .get_type_data::<ReflectAsset>(TypeId::of::<AssetType>())
                .unwrap()
                .clone()
        };

        let value = AssetType {
            field: "test".into(),
        };

        let handle = reflect_asset.spawn(app.world_mut(), &value);
        // struct is a reserved keyword, so we can't use it here
        let strukt = reflect_asset
            .get_mut(app.world_mut(), &handle)
            .unwrap()
            .reflect_mut()
            .as_struct()
            .unwrap();
        strukt
            .field_mut("field")
            .unwrap()
            .apply(&String::from("edited"));

        assert_eq!(reflect_asset.count(app.world()), 1);
        let ids: Vec<_> = reflect_asset.ids(app.world()).collect();
        assert_eq!(ids.len(), 1);
        let id = ids[0];

        let asset = reflect_asset.get(app.world(), id).unwrap();
        assert_eq!(asset.downcast_ref::<AssetType>().unwrap().field, "edited");

        reflect_asset.remove(app.world_mut(), id).unwrap();
        assert_eq!(reflect_asset.count(app.world()), 0);
    }
}
