use alloc::boxed::Box;
use core::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use bevy_ecs::world::{unsafe_world_cell::UnsafeWorldCell, World};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect, ReflectMut, ReflectRef};

use crate::{
    Asset, AssetId, AssetMut, AssetRef, AssetWriteStrategy, Assets, Handle, InvalidGenerationError,
    UntypedAssetId, UntypedHandle,
};

/// The equivalent of `&dyn Reflect`.
///
/// Holds a type that can pass out a reference to a reflectable type.
///
/// Read-only reflection for the asset reference.
pub struct AssetRefReflect<'a> {
    asset_ref: Box<dyn Deref<Target = dyn Reflect> + 'a>,
}

impl<'a> AssetRefReflect<'a> {
    pub fn new<A: Asset + Reflect>(asset_ref: AssetRef<'a, A>) -> Self {
        /// Wrapper that adapts an `AssetRef<A>` to deref to `dyn Reflect`
        struct AssetRefWrapper<'a, A: Asset + Reflect> {
            asset_ref: AssetRef<'a, A>,
        }

        impl<'a, A: Asset + Reflect> Deref for AssetRefWrapper<'a, A> {
            type Target = dyn Reflect;

            fn deref(&self) -> &Self::Target {
                self.asset_ref.as_reflect()
            }
        }

        Self {
            asset_ref: Box::new(AssetRefWrapper::<'a, A> { asset_ref }),
        }
    }
    #[inline]
    pub fn reflect(&mut self) -> ReflectRef<'_> {
        self.asset_ref.as_ref().reflect_ref()
    }

    /// Attempts to downcast the asset reference to a concrete type.
    ///
    /// Returns `Some(&T)` if the asset is of type `T`, or `None` otherwise.
    #[inline]
    pub fn downcast_ref<T: Asset + Reflect + 'static>(&self) -> Option<&T> {
        self.asset_ref.as_ref().as_any().downcast_ref::<T>()
    }

    /// Returns the asset as `&dyn Any`.
    #[inline]
    pub fn as_any(&self) -> &dyn Any {
        self.asset_ref.as_ref().as_any()
    }
}

impl<'a> Debug for AssetRefReflect<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.asset_ref.as_ref().fmt(f)
    }
}

/// The equivalent of `&mut dyn Reflect`.
///
/// Holds a type that can pass out a reference to a reflectable type.
///
/// Read-write reflection for the asset reference.
pub struct AssetMutReflect<'a> {
    asset_mut: Box<dyn DerefMut<Target = dyn Reflect> + 'a>,
}

impl<'a> AssetMutReflect<'a> {
    pub fn new<A: Asset + Reflect>(asset_mut: AssetMut<'a, A>) -> Self
    where
        A::AssetStorage: AssetWriteStrategy<A>,
    {
        /// Wrapper that adapts an `AssetMut<A>` to deref to `dyn mut Reflect`
        struct AssetMutWrapper<'a, A: Asset<AssetStorage: AssetWriteStrategy<A>> + Reflect> {
            asset_mut: AssetMut<'a, A>,
        }

        impl<'a, A: Asset<AssetStorage: AssetWriteStrategy<A>> + Reflect> Deref for AssetMutWrapper<'a, A> {
            type Target = dyn Reflect;

            fn deref(&self) -> &Self::Target {
                self.asset_mut.as_reflect()
            }
        }

        impl<'a, A: Asset<AssetStorage: AssetWriteStrategy<A>> + Reflect> DerefMut
            for AssetMutWrapper<'a, A>
        {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.asset_mut.as_reflect_mut()
            }
        }

        Self {
            asset_mut: Box::new(AssetMutWrapper::<'a, A> { asset_mut }),
        }
    }
    #[inline]
    pub fn reflect_mut(&mut self) -> ReflectMut<'_> {
        self.asset_mut.as_mut().reflect_mut()
    }

    /// Attempts to downcast the asset reference to a concrete type.
    ///
    /// Returns `Some(&T)` if the asset is of type `T`, or `None` otherwise.
    #[inline]
    pub fn downcast_ref<T: Asset + Reflect + 'static>(&self) -> Option<&T> {
        self.asset_mut.as_ref().as_any().downcast_ref::<T>()
    }

    /// Attempts to downcast the asset mutable reference to a concrete type.
    ///
    /// Returns `Some(&mut T)` if the asset is of type `T`, or `None` otherwise.
    #[inline]
    pub fn downcast_mut<T: Asset + Reflect + 'static>(&mut self) -> Option<&mut T> {
        self.asset_mut.as_mut().as_any_mut().downcast_mut::<T>()
    }

    /// Returns the asset as `&dyn Any`.
    #[inline]
    pub fn as_any(&self) -> &dyn Any {
        self.asset_mut.as_ref().as_any()
    }

    /// Returns the asset as `&mut dyn Any`.
    #[inline]
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self.asset_mut.as_mut().as_any_mut()
    }
}

impl<'a> Debug for AssetMutReflect<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.asset_mut.as_ref().fmt(f)
    }
}

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

    get: fn(&World, UntypedAssetId) -> Option<AssetRefReflect>,
    // SAFETY:
    // - may only be called with an [`UnsafeWorldCell`] which can be used to access the corresponding `Assets<T>` resource mutably
    // - may only be used to access **at most one** access at once
    get_unchecked_mut:
        for<'w> unsafe fn(UnsafeWorldCell<'w>, UntypedAssetId) -> Option<AssetMutReflect<'w>>,
    add: fn(&mut World, &dyn PartialReflect) -> UntypedHandle,
    insert:
        fn(&mut World, UntypedAssetId, &dyn PartialReflect) -> Result<(), InvalidGenerationError>,
    len: fn(&World) -> usize,
    ids: for<'w> fn(&'w World) -> Box<dyn Iterator<Item = UntypedAssetId> + 'w>,
    remove: fn(&mut World, UntypedAssetId), //  -> Option<Box<dyn Reflect>>,
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
    pub fn get<'w>(
        &self,
        world: &'w World,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<AssetRefReflect<'w>> {
        (self.get)(world, asset_id.into())
    }

    /// Equivalent of [`Assets::get_mut`]
    pub fn get_mut<'w>(
        &self,
        world: &'w mut World,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<AssetMutReflect<'w>> {
        // SAFETY: unique world access
        #[expect(
            unsafe_code,
            reason = "Use of unsafe `Self::get_unchecked_mut()` function."
        )]
        unsafe {
            (self.get_unchecked_mut)(world.as_unsafe_world_cell(), asset_id.into())
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
    /// * Only call this method if you know that the [`UnsafeWorldCell`] may be used to access the corresponding `Assets<T>`
    /// * Don't call this method more than once in the same scope.
    #[expect(
        unsafe_code,
        reason = "This function calls unsafe code and has safety requirements."
    )]
    pub unsafe fn get_unchecked_mut<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        asset_id: impl Into<UntypedAssetId>,
    ) -> Option<AssetMutReflect<'w>> {
        // SAFETY: requirements are deferred to the caller
        unsafe { (self.get_unchecked_mut)(world, asset_id.into()) }
    }

    /// Equivalent of [`Assets::add`]
    pub fn add(&self, world: &mut World, value: &dyn PartialReflect) -> UntypedHandle {
        (self.add)(world, value)
    }
    /// Equivalent of [`Assets::insert`]
    pub fn insert(
        &self,
        world: &mut World,
        asset_id: impl Into<UntypedAssetId>,
        value: &dyn PartialReflect,
    ) -> Result<(), InvalidGenerationError> {
        (self.insert)(world, asset_id.into(), value)
    }

    /// Equivalent of [`Assets::remove`]
    pub fn remove(&self, world: &mut World, asset_id: impl Into<UntypedAssetId>) {
        (self.remove)(world, asset_id.into());
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

impl<A: Asset + Reflect + FromReflect> FromType<A> for ReflectAsset
where
    A::AssetStorage: AssetWriteStrategy<A>,
{
    fn from_type() -> Self {
        ReflectAsset {
            handle_type_id: TypeId::of::<Handle<A>>(),
            assets_resource_type_id: TypeId::of::<Assets<A>>(),
            get: |world, asset_id| {
                world
                    .resource::<Assets<A>>()
                    .get(asset_id.typed_debug_checked())
                    .map(|asset_ref| AssetRefReflect::new::<A>(asset_ref))
            },
            get_unchecked_mut: |world, asset_id| {
                // SAFETY: `get_unchecked_mut` must be called with `UnsafeWorldCell` having access to `Assets<A>`,
                // and must ensure to only have at most one reference to it live at all times.
                #[expect(unsafe_code, reason = "Uses `UnsafeWorldCell::get_resource_mut()`.")]
                let assets = unsafe { world.get_resource_mut::<Assets<A>>().unwrap().into_inner() };
                let asset_mut = assets.get_mut(asset_id.typed_debug_checked());
                asset_mut.map(|asset_mut| AssetMutReflect::new::<A>(asset_mut))
            },
            add: |world, value| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::add`");
                assets.add(value).untyped()
            },
            insert: |world, asset_id, value| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::set`");
                assets.insert(asset_id.typed_debug_checked(), value)
            },
            len: |world| {
                let assets = world.resource::<Assets<A>>();
                assets.len()
            },
            ids: |world| {
                let assets = world.resource::<Assets<A>>();
                Box::new(assets.ids().map(AssetId::untyped))
            },
            remove: |world, asset_id| {
                let mut assets = world.resource_mut::<Assets<A>>();
                assets.remove(asset_id.typed_debug_checked());
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

    use crate::{Asset, AssetApp, AssetPlugin, ReflectAsset};
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

        {
            let mut reflect_mut = reflect_asset.get_mut(app.world_mut(), &handle).unwrap();
            // struct is a reserved keyword, so we can't use it here
            let strukt = reflect_mut.reflect_mut().as_struct().unwrap();
            strukt
                .field_mut("field")
                .unwrap()
                .apply(&String::from("edited"));
        }

        assert_eq!(reflect_asset.len(app.world()), 1);
        let ids: Vec<_> = reflect_asset.ids(app.world()).collect();
        assert_eq!(ids.len(), 1);
        let id = ids[0];

        {
            let asset = reflect_asset.get(app.world(), id).unwrap();
            assert_eq!(asset.downcast_ref::<AssetType>().unwrap().field, "edited");
        }

        reflect_asset.remove(app.world_mut(), id);
        assert_eq!(reflect_asset.len(app.world()), 0);
    }
}
