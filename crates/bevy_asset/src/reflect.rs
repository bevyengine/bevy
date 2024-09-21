use alloc::sync::Arc;
use core::any::{Any, TypeId};

use bevy_ecs::world::{unsafe_world_cell::UnsafeWorldCell, World};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect};

use crate::{Asset, AssetId, Assets, Handle, MutableAssetError, UntypedAssetId, UntypedHandle};

/// Type data for the [`TypeRegistry`](bevy_reflect::TypeRegistry) used to operate on reflected [`Asset`]s.
///
/// This type provides similar methods to [`Assets<T>`] like [`get`](ReflectAsset::get),
/// [`add`](ReflectAsset::add) and [`remove`](ReflectAsset::remove), but can be used in situations where you don't know which asset type `T` you want
/// until runtime.
///
/// [`ReflectAsset`] can be obtained via [`TypeRegistration::data`](bevy_reflect::TypeRegistration::data) if the asset was registered using [`register_asset_reflect`](crate::AssetApp::register_asset_reflect).
#[derive(Clone)]
pub struct ReflectAsset {
    handle_type_id: TypeId,
    assets_resource_type_id: TypeId,

    get: fn(&World, UntypedHandle) -> Option<&dyn Reflect>,
    // SAFETY:
    // - may only be called with an [`UnsafeWorldCell`] which can be used to access the corresponding `Assets<T>` resource mutably
    // - may only be used to access **at most one** access at once
    get_unchecked_mut: unsafe fn(
        UnsafeWorldCell<'_>,
        UntypedHandle,
    ) -> Result<&mut dyn Reflect, MutableAssetError>,
    add: fn(&mut World, &dyn PartialReflect) -> UntypedHandle,
    insert: fn(&mut World, UntypedHandle, &dyn PartialReflect),
    len: fn(&World) -> usize,
    ids: for<'w> fn(&'w World) -> Box<dyn Iterator<Item = UntypedAssetId> + 'w>,
    remove: fn(&mut World, UntypedHandle) -> ReflectedAssetPresence,
}

impl ReflectAsset {
    /// The [`TypeId`] of the [`Handle<T>`] for this asset
    pub fn handle_type_id(&self) -> TypeId {
        self.handle_type_id
    }

    /// The [`TypeId`] of the [`Assets<T>`] resource
    pub fn assets_resource_type_id(&self) -> TypeId {
        self.assets_resource_type_id
    }

    /// Equivalent of [`Assets::get`]
    pub fn get<'w>(&self, world: &'w World, handle: UntypedHandle) -> Option<&'w dyn Reflect> {
        (self.get)(world, handle)
    }

    /// Equivalent of [`Assets::get_mut`]
    pub fn get_mut<'w>(
        &self,
        world: &'w mut World,
        handle: UntypedHandle,
    ) -> Result<&'w mut dyn Reflect, MutableAssetError> {
        // SAFETY: unique world access
        #[expect(
            unsafe_code,
            reason = "Use of unsafe `Self::get_unchecked_mut()` function."
        )]
        unsafe {
            (self.get_unchecked_mut)(world.as_unsafe_world_cell(), handle)
        }
    }

    /// Equivalent of [`Assets::get_mut`], but works with an [`UnsafeWorldCell`].
    ///
    /// Only use this method when you have ensured that you are the *only* one with access to the [`Assets`] resource of the asset type.
    /// Furthermore, this does *not* allow you to have look up two distinct handles,
    /// you can only have at most one alive at the same time.
    /// This means that this is *not allowed*:
    /// ```no_run
    /// # use bevy_asset::{ReflectAsset, UntypedHandle};
    /// # use bevy_ecs::prelude::World;
    /// # let reflect_asset: ReflectAsset = unimplemented!();
    /// # let mut world: World = unimplemented!();
    /// # let handle_1: UntypedHandle = unimplemented!();
    /// # let handle_2: UntypedHandle = unimplemented!();
    /// let unsafe_world_cell = world.as_unsafe_world_cell();
    /// let a = unsafe { reflect_asset.get_unchecked_mut(unsafe_world_cell, handle_1).unwrap() };
    /// let b = unsafe { reflect_asset.get_unchecked_mut(unsafe_world_cell, handle_2).unwrap() };
    /// // ^ not allowed, two mutable references through the same asset resource, even though the
    /// // handles are distinct
    ///
    /// println!("a = {a:?}, b = {b:?}");
    /// ```
    ///
    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method if you know that the [`UnsafeWorldCell`] may be used to access the corresponding `Assets<T>`
    /// * Don't call this method more than once in the same scope.
    #[expect(
        unsafe_code,
        reason = "This function calls unsafe code and has safety requirements."
    )]
    pub unsafe fn get_unchecked_mut<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        handle: UntypedHandle,
    ) -> Result<&'w mut dyn Reflect, MutableAssetError> {
        // SAFETY: requirements are deferred to the caller
        unsafe { (self.get_unchecked_mut)(world, handle) }
    }

    /// Equivalent of [`Assets::add`]
    pub fn add(&self, world: &mut World, value: &dyn PartialReflect) -> UntypedHandle {
        (self.add)(world, value)
    }
    /// Equivalent of [`Assets::insert`]
    pub fn insert(&self, world: &mut World, handle: UntypedHandle, value: &dyn PartialReflect) {
        (self.insert)(world, handle, value);
    }

    /// Equivalent of [`Assets::remove`]
    pub fn remove(&self, world: &mut World, handle: UntypedHandle) -> ReflectedAssetPresence {
        (self.remove)(world, handle)
    }

    /// Equivalent of [`Assets::len`]
    pub fn len(&self, world: &World) -> usize {
        (self.len)(world)
    }

    /// Equivalent of [`Assets::is_empty`]
    pub fn is_empty(&self, world: &World) -> bool {
        self.len(world) == 0
    }

    /// Equivalent of [`Assets::ids`]
    pub fn ids<'w>(&self, world: &'w World) -> impl Iterator<Item = UntypedAssetId> + 'w {
        (self.ids)(world)
    }
}

impl<A: Asset + FromReflect> FromType<A> for ReflectAsset {
    fn from_type() -> Self {
        ReflectAsset {
            handle_type_id: TypeId::of::<Handle<A>>(),
            assets_resource_type_id: TypeId::of::<Assets<A>>(),
            get: |world, handle| {
                let assets = world.resource::<Assets<A>>();
                let asset = assets.get(&handle.typed_debug_checked());
                asset.map(|asset| asset as &dyn Reflect)
            },
            get_unchecked_mut: |world, handle| {
                // SAFETY: `get_unchecked_mut` must be called with `UnsafeWorldCell` having access to `Assets<A>`,
                // and must ensure to only have at most one reference to it live at all times.
                #[expect(unsafe_code, reason = "Uses `UnsafeWorldCell::get_resource_mut()`.")]
                let assets = unsafe { world.get_resource_mut::<Assets<A>>().unwrap().into_inner() };
                let asset = assets.get_mut(&handle.typed_debug_checked());
                asset.map(|asset| asset as &mut dyn Reflect)
            },
            add: |world, value| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::add`");
                assets.add(value).untyped()
            },
            insert: |world, handle, value| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::set`");
                assets.insert(&handle.typed_debug_checked(), value);
            },
            len: |world| {
                let assets = world.resource::<Assets<A>>();
                assets.len()
            },
            ids: |world| {
                let assets = world.resource::<Assets<A>>();
                Box::new(assets.ids().map(AssetId::untyped))
            },
            remove: |world, handle| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value = assets.remove(&handle.typed_debug_checked());
                match value {
                    crate::AssetPresence::None => ReflectedAssetPresence::None,
                    crate::AssetPresence::Unlocked(asset) => {
                        ReflectedAssetPresence::Unlocked(Box::new(asset) as Box<dyn Reflect>)
                    }
                    crate::AssetPresence::Locked(asset_arc) => {
                        ReflectedAssetPresence::Locked(asset_arc as Arc<dyn Reflect>)
                    }
                }
            },
        }
    }
}

/// The reflected version of [`crate::AssetPresence`].
#[derive(Default)]
pub enum ReflectedAssetPresence {
    /// The asset is not present (either has not been added yet, or was removed).
    #[default]
    None,
    /// The asset is present and unlocked. This asset may be mutated.
    Unlocked(Box<dyn Reflect>),
    /// The asset is present, but is locked. The asset cannot be mutated, but can be passed to threads.
    Locked(Arc<dyn Reflect>),
}

impl ReflectedAssetPresence {
    /// Takes the current value out, and leaves [`crate::AssetPresence::None`] in its place.
    pub fn take(&mut self) -> Self {
        core::mem::take(self)
    }

    /// Gets a borrow to the stored asset, if there is an asset.
    pub fn as_ref(&self) -> Option<&dyn Reflect> {
        match self {
            Self::None => None,
            Self::Unlocked(box_reflect) => Some(box_reflect.as_ref()),
            Self::Locked(arc_reflect) => Some(arc_reflect.as_ref()),
        }
    }

    /// Gets a mutable borrow to the stored asset (if unlocked).
    pub fn as_mut(&mut self) -> Result<&mut dyn Reflect, MutableAssetError> {
        match self {
            Self::None => Err(MutableAssetError::Missing),
            Self::Unlocked(asset) => Ok(asset.as_mut()),
            Self::Locked(_) => Err(MutableAssetError::Locked),
        }
    }

    /// Returns whether the asset is missing.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns whether the asset is present.
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Unlocked(_) | Self::Locked(_))
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
///     let value = reflect_asset.get(world, handle).unwrap();
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
    use core::any::TypeId;

    use crate as bevy_asset;
    use crate::{Asset, AssetApp, AssetPlugin, ReflectAsset, UntypedHandle};
    use bevy_app::App;
    use bevy_ecs::reflect::AppTypeRegistry;
    use bevy_reflect::Reflect;

    #[derive(Asset, Reflect)]
    struct AssetType {
        field: String,
    }

    #[test]
    fn test_reflect_asset_operations() {
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default())
            .init_asset::<AssetType>()
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

        let handle = reflect_asset.add(app.world_mut(), &value);
        // struct is a reserved keyword, so we can't use it here
        let strukt = reflect_asset
            .get_mut(app.world_mut(), handle)
            .unwrap()
            .reflect_mut()
            .as_struct()
            .unwrap();
        strukt
            .field_mut("field")
            .unwrap()
            .apply(&String::from("edited"));

        assert_eq!(reflect_asset.len(app.world()), 1);
        let ids: Vec<_> = reflect_asset.ids(app.world()).collect();
        assert_eq!(ids.len(), 1);

        let fetched_handle = UntypedHandle::Weak(ids[0]);
        let asset = reflect_asset
            .get(app.world(), fetched_handle.clone_weak())
            .unwrap();
        assert_eq!(asset.downcast_ref::<AssetType>().unwrap().field, "edited");

        assert!(reflect_asset
            .remove(app.world_mut(), fetched_handle)
            .is_some());
        assert_eq!(reflect_asset.len(app.world()), 0);
    }
}
