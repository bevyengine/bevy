use std::any::{Any, TypeId};

use bevy_ecs::world::World;
use bevy_reflect::{FromReflect, FromType, Reflect, Uuid};

use crate::{Asset, Assets, Handle, HandleId, HandleUntyped};

/// A struct used to operate on reflected [`Asset`]s.
///
/// A [`ReflectAsset`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`] if it was registered using [`crate::AddAsset::register_asset_reflect`].
#[derive(Clone)]
pub struct ReflectAsset {
    type_uuid: Uuid,
    assets_resource_type_id: TypeId,

    get: fn(&World, HandleUntyped) -> Option<&dyn Reflect>,
    get_mut: fn(&mut World, HandleUntyped) -> Option<&mut dyn Reflect>,
    get_unchecked_mut: unsafe fn(&World, HandleUntyped) -> Option<&mut dyn Reflect>,
    add: fn(&mut World, &dyn Reflect) -> HandleUntyped,
    set: fn(&mut World, HandleUntyped, &dyn Reflect) -> HandleUntyped,
    len: fn(&World) -> usize,
    ids: for<'w> fn(&'w World) -> Box<dyn Iterator<Item = HandleId> + 'w>,
    remove: fn(&mut World, HandleUntyped) -> Option<Box<dyn Reflect>>,
}

impl ReflectAsset {
    /// The [`bevy_reflect::TypeUuid`] of the asset
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

    /// Equivalent of [`Assets::remove`]
    pub fn remove(&self, world: &mut World, handle: HandleUntyped) -> Option<Box<dyn Reflect>> {
        (self.remove)(world, handle)
    }

    /// Equivalent of [`Assets::len`]
    #[allow(clippy::len_without_is_empty)] // clippy expects the `is_empty` method to have the signature `(&self) -> bool`
    pub fn len(&self, world: &World) -> usize {
        (self.len)(world)
    }

    /// Equivalent of [`Assets::is_empty`]
    pub fn is_empty(&self, world: &World) -> bool {
        self.len(world) == 0
    }

    /// Equivalent of [`Assets::ids`]
    pub fn ids<'w>(&self, world: &'w World) -> impl Iterator<Item = HandleId> + 'w {
        (self.ids)(world)
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
            len: |world| {
                let assets = world.resource::<Assets<A>>();
                assets.len()
            },
            ids: |world| {
                let assets = world.resource::<Assets<A>>();
                Box::new(assets.ids())
            },
            remove: |world, handle| {
                let mut assets = world.resource_mut::<Assets<A>>();
                let value = assets.remove(handle);
                value.map(|value| Box::new(value) as Box<dyn Reflect>)
            },
        }
    }
}

/// Reflect type data struct relating a `Handle<T>` back to the `T` asset type.
#[derive(Clone)]
pub struct ReflectHandle {
    type_uuid: Uuid,
    asset_type_id: TypeId,
    downcast_handle_untyped: fn(&dyn Any) -> Option<HandleUntyped>,
}
impl ReflectHandle {
    /// The [`bevy_reflect::TypeUuid`] of the asset
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

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy_app::{App, AppTypeRegistry};
    use bevy_reflect::{FromReflect, Reflect, ReflectMut, TypeUuid};

    use crate::{AddAsset, AssetPlugin, HandleUntyped, ReflectAsset};

    #[derive(Reflect, FromReflect, TypeUuid)]
    #[uuid = "09191350-1238-4736-9a89-46f04bda6966"]
    struct AssetType {
        field: String,
    }

    #[test]
    fn test() {
        let mut app = App::new();
        app.add_plugin(AssetPlugin)
            .add_asset::<AssetType>()
            .register_asset_reflect::<AssetType>();

        let reflect_asset = {
            let type_registry = app.world.resource::<AppTypeRegistry>();
            let type_registry = type_registry.read();

            type_registry
                .get_type_data::<ReflectAsset>(TypeId::of::<AssetType>())
                .unwrap()
                .clone()
        };

        let value = AssetType {
            field: "test".into(),
        };

        let handle = reflect_asset.add(&mut app.world, &value);
        let strukt = match reflect_asset
            .get_mut(&mut app.world, handle)
            .unwrap()
            .reflect_mut()
        {
            ReflectMut::Struct(s) => s,
            _ => unreachable!(),
        };
        strukt
            .field_mut("field")
            .unwrap()
            .apply(&String::from("edited"));

        assert_eq!(reflect_asset.len(&app.world), 1);
        let ids: Vec<_> = reflect_asset.ids(&app.world).collect();
        assert_eq!(ids.len(), 1);

        let fetched_handle = HandleUntyped::weak(ids[0]);
        let asset = reflect_asset
            .get(&app.world, fetched_handle.clone_weak())
            .unwrap();
        assert_eq!(asset.downcast_ref::<AssetType>().unwrap().field, "edited");

        reflect_asset
            .remove(&mut app.world, fetched_handle)
            .unwrap();
        assert_eq!(reflect_asset.len(&app.world), 0);
    }
}
