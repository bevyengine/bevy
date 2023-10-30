use crate::{render_resource::ShaderDefVal, *};
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, query::*, system::*};
pub use bevy_render_macros::PipelineKey;
use bevy_utils::{HashMap, HashSet};
use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
};

use self::composite::CompositeKey;

pub type KeyPrimitive = u64;

mod composite;
mod packed_types;
pub struct KeyMeta {
    pub dynamic_components: HashMap<TypeId, SizeOffset>,
    pub size: u8,
}

impl KeyMeta {
    fn new<K: 'static>() -> Self {
        debug!("new dynamic: {} => {:?}", type_name::<K>(), TypeId::of::<K>());
        Self {
            dynamic_components: Default::default(),
            size: 0,
        }
    }

    fn new_sized<K: FixedSizeKey>() -> Self {
        debug!("new fixed ({}): {} => {:?}", K::fixed_size(), type_name::<K>(), TypeId::of::<K>());
        Self {
            dynamic_components: Default::default(),
            size: K::fixed_size(),
        }
    }
}

/// provides the means to turn a raw u32 into a key
#[derive(Resource, Default)]
pub struct KeyMetaStore {
    metas: HashMap<TypeId, KeyMeta>,
    unfinalized: HashSet<TypeId>,
}

impl std::fmt::Debug for KeyMetaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyMetaStore")
            .field("data", &"...")
            .finish()
    }
}

fn missing<T, U>() -> U {
    panic!(
        "required type {} is not present in container",
        type_name::<T>()
    )
}
fn missing_id<U>() -> U {
    panic!("required composite type id is not present in container")
}

impl KeyMetaStore {
    pub fn register_fixed_size<K: FixedSizeKey>(&mut self) {
        self.metas.insert(TypeId::of::<K>(), KeyMeta::new_sized::<K>());
    }

    pub fn register_dynamic<K: 'static>(&mut self) {
        self.metas.insert(TypeId::of::<K>(), KeyMeta::new::<K>());
        self.unfinalized.insert(TypeId::of::<K>());
    }

    pub fn try_meta<K: AnyKeyType>(&self) -> Option<&KeyMeta> {
        self.metas.get(&TypeId::of::<K>())
    }

    pub fn meta<K: AnyKeyType>(&self) -> &KeyMeta {
        self.try_meta::<K>().unwrap_or_else(missing::<K, _>)
    }

    fn meta_mut<K: AnyKeyType>(&mut self) -> &mut KeyMeta {
        self.metas.get_mut(&TypeId::of::<K>()).unwrap_or_else(missing::<K, _>)
    }

    pub fn add_dynamic_part<K: AnyKeyType, PART: AnyKeyType>(&mut self) {
        if !self.unfinalized.contains(&TypeId::of::<K>()) {
            panic!("{} is not dynamic, or keystore is already finalized", type_name::<K>());
        }

        self.meta_mut::<K>().dynamic_components.insert(TypeId::of::<PART>(), SizeOffset(u8::MAX, u8::MAX));
    }

    pub fn size_for_id(&self, id: &TypeId) -> u8 {
        self.metas.get(id).unwrap_or_else(missing_id).size
    }

    pub fn pipeline_key<K: AnyKeyType + KeyTypeConcrete>(&self, value: KeyPrimitive) -> PipelineKey<K> {
        let value = K::unpack(value, self);
        PipelineKey {
            store: self,
            value,
        }
    }

    pub fn finalize(&mut self) {
        let mut todo = self.unfinalized.clone();
        let mut count = todo.len();
        while count > 0 {
            todo.retain(|k| {
                let (k, mut v) = self.metas.remove_entry(k).unwrap();
                if v.dynamic_components.keys().any(|k| self.unfinalized.contains(k)) {
                    self.metas.insert(k, v);
                    return true;
                }

                let mut offset = 0;
                for (id, so) in v.dynamic_components.iter_mut() {
                    so.0 = self.size_for_id(id);
                    so.1 = offset;
                    offset += so.0;
                }

                v.size = offset;
                self.metas.insert(k, v);
                false
            });

            let new_count = todo.len();
            if count == new_count {
                panic!("circular key reference: {todo:?}");
            }
            count = new_count;
        }
    }
}
pub struct PackedPipelineKey<T: AnyKeyType> {
    pub packed: KeyPrimitive,
    pub size: u8,
    _p: PhantomData<fn() -> T>
}

impl<T: AnyKeyType> PackedPipelineKey<T> {
    pub fn new(packed: KeyPrimitive, size: u8) -> Self {
        Self {
            packed,
            size,
            _p: Default::default(),
        }
    }
}

pub trait KeyTypeConcrete: AnyKeyType {
    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self;

    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset>;

    fn size(store: &KeyMetaStore) -> u8 {
        Self::positions(store).values().map(|so| so.0).sum()
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self> where Self: Sized;
}

pub trait AnyKeyType: Any + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone, Copy, Debug)]
pub struct SizeOffset(pub u8, pub u8);

#[derive(Debug)]
pub struct PipelineKey<'a, T: AnyKeyType> {
    store: &'a KeyMetaStore,
    value: T,
}

impl<'a, T: AnyKeyType> Deref for PipelineKey<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T: AnyKeyType + PartialEq> PartialEq for PipelineKey<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<'a, T: AnyKeyType + KeyTypeConcrete> PipelineKey<'a, T> {
    pub fn extract<U: AnyKeyType + KeyTypeConcrete>(&'a self) -> Option<PipelineKey<'a, U>> {
        let positions = T::positions(self.store);
        let SizeOffset(size, offset) = positions.get(&TypeId::of::<U>())?;
        let key = T::pack(&self.value, &self.store);
        let value = (key.packed >> offset) & ((1 << size) - 1);
        Some(self.store.pipeline_key(value))
    }
}


#[derive(Component, Default)]
pub struct PipelineKeys {
    packed_keys: HashMap<TypeId, (KeyPrimitive, u8)>,
    shader_defs: Vec<ShaderDefVal>,
}

impl PipelineKeys {
    pub fn get_raw_by_id(&self, id: &TypeId) -> Option<KeyPrimitive> {
        self.packed_keys.get(id).map(|(v, _)| *v)
    }

    pub fn get_raw<K: AnyKeyType>(&self) -> Option<KeyPrimitive> {
        self.get_raw_by_id(&TypeId::of::<K>())
    }

    pub fn get_raw_and_size_by_id(&self, id: &TypeId) -> Option<(KeyPrimitive, u8)> {
        self.packed_keys.get(id).copied()
    }

    pub fn get_packed_key<K: AnyKeyType>(&self) -> Option<PackedPipelineKey<K>> {
        let (raw, size) = self.get_raw_and_size_by_id(&TypeId::of::<K>())?;
        Some(PackedPipelineKey::new(raw, size))
    }

    pub fn set_raw<K: AnyKeyType>(&mut self, value: KeyPrimitive, size: u8) {
        self.packed_keys.insert(TypeId::of::<K>(), (value, size));
    }

    pub fn get_key<'a, K: AnyKeyType + KeyTypeConcrete>(&self, store: &'a KeyMetaStore) -> Option<PipelineKey<'a, K>> {
        Some(store.pipeline_key(self.get_raw::<K>()?))
    }
}

pub struct PipelineKeyPlugin;

#[derive(SystemSet)]
pub struct KeySetMarker<T>(PhantomData<fn() -> T>);

impl<T> std::hash::Hash for KeySetMarker<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> std::fmt::Debug for KeySetMarker<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("KeySetMarker").field(&self.0).finish()
    }
}

impl<T> Eq for KeySetMarker<T> {}

impl<T> PartialEq for KeySetMarker<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<T> Clone for KeySetMarker<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for KeySetMarker<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Plugin for PipelineKeyPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<KeyMetaStore>();
        // app.add_systems()
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .world
            .resource_mut::<KeyMetaStore>()
            .finalize();
    }
}

pub trait FixedSizeKey: 'static {
    fn fixed_size() -> u8;
}

pub trait SystemKey: AnyKeyType + KeyTypeConcrete + FixedSizeKey {
    type Param: SystemParam + 'static;
    type Query: ReadOnlyWorldQuery + 'static;

    fn from_params(
        params: &SystemParamItem<Self::Param>,
        query_item: QueryItem<Self::Query>,
    ) -> Self;

    fn shader_defs(&self) -> Vec<ShaderDefVal>;
}

// #[derive(PipelineKey)]
// [dynamic_key]
// pub struct MyKey(u32);
pub trait DynamicKey: AnyKeyType + KeyTypeConcrete {}

pub trait AddPipelineKey {
    fn register_system_key<K: SystemKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self;
    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self;
    fn register_dynamic_key<K: DynamicKey, F: ReadOnlyWorldQuery + 'static>(&mut self)
        -> &mut Self;
    fn register_dynamic_key_part<K: DynamicKey, PART: AnyKeyType>(&mut self) -> &mut Self;
}

impl AddPipelineKey for App {
    fn register_system_key<K: SystemKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .register_fixed_size::<K>();
        self.add_systems(
            Render,
            (|p: StaticSystemParam<K::Param>, store: Res<KeyMetaStore>, mut q: Query<(&mut PipelineKeys, K::Query), F>| {
                let p = p.into_inner();
                for (mut keys, query) in q.iter_mut() {
                    let key = K::from_params(&p, query);
                    keys.shader_defs.extend(key.shader_defs());
                    let PackedPipelineKey{ packed, size, .. } = K::pack(&key, &store);
                    keys.set_raw::<K>(packed, size);
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::PrepareKeys),
        );
        self.add_systems(
            ExtractSchedule,
            |mut commands: Commands, q: Query<Entity, F>| {
                for ent in q.iter() {
                    commands.entity(ent).insert(PipelineKeys::default());
                }
            },
        );
        self
    }

    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .register_dynamic::<K>();
        self.add_systems(
            Render,
            (|mut q: Query<&mut PipelineKeys, F>| {
                for mut keys in q.iter_mut() {
                    if let Some(PackedPipelineKey{packed, size, ..}) = K::from_keys(&keys) {
                        keys.set_raw::<K>(packed, size);
                    }
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::PrepareKeys),
        );
        self.configure_sets(Render, K::set_config());
        self
    }

    fn register_dynamic_key<K: DynamicKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .register_dynamic::<K>();
        self.add_systems(
            Render,
            (|store: Res<KeyMetaStore>, mut q: Query<&mut PipelineKeys, F>| {
                let dynamic_components = store.meta::<K>().dynamic_components.clone();
                'ent: for mut keys in q.iter_mut() {
                    let mut value = 0;
                    let mut size = 0;
                    for (id, so) in dynamic_components.iter() {
                        let Some((part, part_size)) = keys.get_raw_and_size_by_id(id) else {
                            break 'ent
                        };

                        value |= part << so.1;
                        size += part_size;
                    }
                    keys.set_raw::<K>(value, size);
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::PrepareKeys),
        );
        self
    }

    fn register_dynamic_key_part<K: DynamicKey, PART: AnyKeyType>(&mut self) -> &mut Self {
        let mut store = self.world.resource_mut::<KeyMetaStore>();
        store.add_dynamic_part::<K, PART>();
        self.configure_sets(
            Render,
            KeySetMarker::<K>::default().after(KeySetMarker::<PART>::default()),
        );
        self
    }
}

/// generate a binary pipeline key based on the presence of a marker component
/// TODO scope needs some work, requires num_enum in scope currently
#[macro_export]
macro_rules! impl_has_world_key {
    ($key:ident, $component:ident, $def:expr) => {
        use bevy_render::pipeline_keys::*;
        #[derive(
            PipelineKey, Default, Clone, Copy, Debug, PartialEq, Eq, Hash
        )]
        pub struct $key(bool);
        impl SystemKey for $key {
            type Param = ();
            type Query = bevy_ecs::prelude::Has<$component>;

            fn from_params(_: &(), has_component: bool) -> Self {
                Self(has_component)
            }

            fn shader_defs(&self) -> Vec<bevy_render::render_resource::ShaderDefVal> {
                if self.0 {
                    vec![$def.into()]
                } else {
                    vec![]
                }
            }
        }
    };
}
