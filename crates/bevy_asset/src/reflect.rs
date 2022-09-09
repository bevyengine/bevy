use std::any::{Any, TypeId};

use bevy_ecs::world::World;
use bevy_reflect::{FromReflect, FromType, Reflect, Uuid};

use crate::{Asset, Assets, Handle, HandleUntyped};

/// A struct used to operate on reflected [`Asset`] of a type.
///
/// A [`ReflectAsset`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectAsset {
    type_uuid: Uuid,
    assets_resource_type_id: TypeId,

    get: fn(&World, HandleUntyped) -> Option<&dyn Reflect>,
    get_mut: fn(&mut World, HandleUntyped) -> Option<&mut dyn Reflect>,
    get_unchecked_mut: unsafe fn(&World, HandleUntyped) -> Option<&mut dyn Reflect>,
    add: fn(&mut World, &dyn Reflect) -> HandleUntyped,
    set: fn(&mut World, HandleUntyped, &dyn Reflect) -> HandleUntyped,
}

impl ReflectAsset {
    /// The [`TypeUuid`] of the asset
    pub fn type_uuid(&self) -> Uuid {
        self.type_uuid
    }

    /// The [`TypeId`] of the [`Assets`] resource
    pub fn assets_resource_type_id(&self) -> TypeId {
        self.assets_resource_type_id
    }

    /// Equivalent of [`Assets::get`]
    pub fn get<'w>(&self, world: &'w World, handle: HandleUntyped) -> Option<&'w dyn Reflect> {
        (self.get)(world, handle)
    }
    /// Equivalent of [`Assets::get_mut`]
    pub fn get_mut<'w>(
        &self,
        world: &'w mut World,
        handle: HandleUntyped,
    ) -> Option<&'w mut dyn Reflect> {
        (self.get_mut)(world, handle)
    }

    /// Equivalent of [`Assets::get_mut`], but does not require a mutable reference to the world.
    /// This is fine if you have ensure that you are the *only* one having access to the `Assets` resource
    /// of the asset type. Furthermore, this does *not* allow you to have look up two distinct handles,
    /// you can only have at most one alive at the same time.
    ///
    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method when you have exclusive access to the world
    /// (or use a scheduler that enforces unique access to the `Assets` resource).
    /// * Don't call this method more than once in the same scope.
    pub unsafe fn get_unchecked_mut<'w>(
        &self,
        world: &'w World,
        handle: HandleUntyped,
    ) -> Option<&'w mut dyn Reflect> {
        // SAFETY: requirements are deferred to the caller
        (self.get_unchecked_mut)(world, handle)
    }

    /// Equivalent of [`Assets::add`]
    pub fn add<'w>(&self, world: &'w mut World, value: &dyn Reflect) -> HandleUntyped {
        (self.add)(world, value)
    }
    /// Equivalent of [`Assets::set`]
    pub fn set<'w>(
        &self,
        world: &'w mut World,
        handle: HandleUntyped,
        value: &dyn Reflect,
    ) -> HandleUntyped {
        (self.set)(world, handle, value)
    }
}

impl<A: Asset + FromReflect> FromType<A> for ReflectAsset {
    fn from_type() -> Self {
        ReflectAsset {
            type_uuid: A::TYPE_UUID,
            assets_resource_type_id: TypeId::of::<Assets<A>>(),
            get: |world, handle| {
                let assets = world.resource::<Assets<A>>();
                let asset = assets.get(&handle.typed());
                asset.map(|asset| asset as &dyn Reflect)
            },
            get_mut: |world, handle| {
                let assets = world.resource_mut::<Assets<A>>().into_inner();
                let asset = assets.get_mut(&handle.typed());
                asset.map(|asset| asset as &mut dyn Reflect)
            },
            get_unchecked_mut: |world, handle| {
                let assets = unsafe {
                    world
                        .get_resource_unchecked_mut::<Assets<A>>()
                        .unwrap()
                        .into_inner()
                };
                let asset = assets.get_mut(&handle.typed());
                asset.map(|asset| asset as &mut dyn Reflect)
            },
            add: |world, value| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::insert`");
                assets.add(value).into()
            },
            set: |world, handle, value| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value: A = FromReflect::from_reflect(value)
                    .expect("could not call `FromReflect::from_reflect` in `ReflectAsset::insert`");
                assets.set(handle, value).into()
            },
        }
    }
}

/// A struct relating a `Handle<T>` back to the `T` asset type.
#[derive(Clone)]
pub struct ReflectHandle {
    type_uuid: Uuid,
    asset_type_id: TypeId,
    downcast_handle_untyped: fn(&dyn Any) -> Option<HandleUntyped>,
}
impl ReflectHandle {
    /// The [`TypeUuid`] of the asset
    pub fn type_uuid(&self) -> Uuid {
        self.type_uuid
    }
    /// The [`TypeId`] of the asset
    pub fn asset_type_id(&self) -> TypeId {
        self.asset_type_id
    }

    /// A way to go from a `Handle<T>` in a `dyn Any` to a [`HandleUntyped`]
    pub fn downcast_handle_untyped(&self, handle: &dyn Any) -> Option<HandleUntyped> {
        (self.downcast_handle_untyped)(handle)
    }
}

impl<A: Asset> FromType<Handle<A>> for ReflectHandle {
    fn from_type() -> Self {
        ReflectHandle {
            type_uuid: A::TYPE_UUID,
            asset_type_id: TypeId::of::<A>(),
            downcast_handle_untyped: |handle: &dyn Any| {
                handle
                    .downcast_ref::<Handle<A>>()
                    .map(|handle| handle.clone_untyped())
            },
        }
    }
}
