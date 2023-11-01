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
    pub shader_def_fn: ShaderDefFn,
    pub dynamic_components: HashMap<TypeId, SizeOffset>,
    pub size: u8,
}

impl KeyMeta {
    fn new<K: KeyTypeConcrete + 'static>() -> Self {
        debug!(
            "new dynamic: {} => {:?}",
            type_name::<K>(),
            TypeId::of::<K>()
        );
        Self {
            shader_def_fn: K::shader_defs,
            dynamic_components: Default::default(),
            size: 0,
        }
    }

    fn new_sized<K: KeyTypeConcrete + FixedSizeKey>() -> Self {
        debug!(
            "new fixed ({}): {} => {:?}",
            K::fixed_size(),
            type_name::<K>(),
            TypeId::of::<K>()
        );
        Self {
            shader_def_fn: K::shader_defs,
            dynamic_components: Default::default(),
            size: K::fixed_size(),
        }
    }
}

#[derive(Resource)]
pub struct KeyMetaStoreInitializer(pub(crate) Option<KeyMetaStore>);

impl KeyMetaStoreInitializer {
    pub(crate) fn take_final(&mut self) -> KeyMetaStore {
        let mut store = self.0.take().expect("key store already taken");
        store.finalize();
        store
    }
}

/// provides the means to turn a raw u32 into a key
#[derive(Default)]
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
fn missing_id<U>(id: &TypeId) -> U {
    panic!("required type id {id:?} is not present in container")
}

impl KeyMetaStore {
    pub fn register_fixed_size<K: KeyTypeConcrete + FixedSizeKey>(&mut self) {
        debug!("fixed size {} -> {:?}", type_name::<K>(), TypeId::of::<K>());
        self.metas
            .insert(TypeId::of::<K>(), KeyMeta::new_sized::<K>());
    }

    pub fn register_dynamic<K: KeyTypeConcrete + 'static>(&mut self) {
        debug!("dynamic {} -> {:?}", type_name::<K>(), TypeId::of::<K>());
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
        self.metas
            .get_mut(&TypeId::of::<K>())
            .unwrap_or_else(missing::<K, _>)
    }

    pub fn add_dynamic_part<K: AnyKeyType, PART: AnyKeyType>(&mut self) {
        if !self.unfinalized.contains(&TypeId::of::<K>()) {
            panic!(
                "{} is not dynamic, or keystore is already finalized",
                type_name::<K>()
            );
        }

        self.meta_mut::<K>()
            .dynamic_components
            .insert(TypeId::of::<PART>(), SizeOffset(u8::MAX, u8::MAX));
    }

    pub fn size_for_id(&self, id: &TypeId) -> u8 {
        self.metas.get(id).unwrap_or_else(|| missing_id(id)).size
    }

    pub fn def_fn_for_id(&self, id: &TypeId) -> Option<ShaderDefFn> {
        self.metas.get(id).map(|meta| meta.shader_def_fn)
    }

    // todo make private when moved into pipeline cache / specializer
    pub fn pipeline_key<K: AnyKeyType + KeyTypeConcrete>(
        &self,
        value: KeyPrimitive,
    ) -> PipelineKey<K> {
        let value = K::unpack(value, self);
        PipelineKey { store: self, value }
    }

    pub fn finalize(&mut self) {
        let mut todo = self.unfinalized.clone();
        let mut count = todo.len();
        while count > 0 {
            todo.retain(|k| {
                let (k, mut v) = self.metas.remove_entry(k).unwrap();
                if v.dynamic_components
                    .keys()
                    .any(|k| self.unfinalized.contains(k))
                {
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
#[derive(Debug)]
pub struct PackedPipelineKey<T: AnyKeyType> {
    pub packed: KeyPrimitive,
    pub size: u8,
    _p: PhantomData<fn() -> T>,
}

impl<T: AnyKeyType> Clone for PackedPipelineKey<T> {
    fn clone(&self) -> Self {
        Self {
            packed: self.packed,
            size: self.size,
            _p: PhantomData,
        }
    }
}

impl<T: AnyKeyType> Copy for PackedPipelineKey<T> {}

impl<T: AnyKeyType + KeyTypeConcrete> PackedPipelineKey<T> {
    pub fn new(packed: KeyPrimitive, size: u8) -> Self {
        Self {
            packed,
            size,
            _p: Default::default(),
        }
    }

    pub fn insert<K: KeyTypeConcrete>(&mut self, value: &K, cache: &PipelineCache) {
        let store = cache.key_store();
        self.insert_packed(K::pack(value, store), cache);
    }

    pub fn insert_packed<K: KeyTypeConcrete>(
        &mut self,
        value: PackedPipelineKey<K>,
        cache: &PipelineCache,
    ) {
        let store = cache.key_store();
        let positions = T::positions(store);
        let so = positions.get(&TypeId::of::<K>()).unwrap();
        self.packed &= !(((1 << so.0) - 1) << so.1);
        self.packed |= value.packed << so.1;
    }
}

pub trait KeyRepack: KeyTypeConcrete {
    type PackedParts;

    fn repack(source: Self::PackedParts) -> PackedPipelineKey<Self>
    where
        Self: Sized;
}

type ShaderDefFn = fn(KeyPrimitive, &KeyMetaStore) -> Vec<ShaderDefVal>;

pub trait KeyShaderDefs {
    fn shader_defs(&self) -> Vec<ShaderDefVal>;
}

pub trait KeyTypeConcrete: AnyKeyType {
    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self;

    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset>;

    fn size(store: &KeyMetaStore) -> u8 {
        Self::positions(store).values().map(|so| so.0).sum()
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self>
    where
        Self: Sized;

    fn shader_defs(value: KeyPrimitive, store: &KeyMetaStore) -> Vec<ShaderDefVal> {
        let mut defs = Vec::default();
        for (id, so) in Self::positions(store) {
            let Some(part_def_fn) = store.def_fn_for_id(&id) else {
                debug!("{id:?} not registered for shader defs");
                continue;
            };
            let part_value = (value >> so.1) & ((1 << so.0) - 1);
            defs.extend(part_def_fn(part_value, store));
        }

        defs
    }
}

pub trait AnyKeyType: Any + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone, Copy, Debug)]
pub struct SizeOffset(pub u8, pub u8);

#[derive(Debug, Copy, Clone)]
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
    pub fn try_extract<U: AnyKeyType + KeyTypeConcrete>(&'a self) -> Option<PipelineKey<'a, U>> {
        let positions = T::positions(self.store);
        let SizeOffset(size, offset) = positions.get(&TypeId::of::<U>())?;
        let key = T::pack(&self.value, &self.store);
        let value = (key.packed >> offset) & ((1 << size) - 1);
        Some(self.store.pipeline_key(value))
    }

    pub fn extract<U: AnyKeyType + KeyTypeConcrete>(&'a self) -> PipelineKey<'a, U> {
        debug!(
            "{} requesting {}, positions: {:?}",
            type_name::<T>(),
            type_name::<U>(),
            T::positions(&self.store)
        );
        let positions = T::positions(self.store);
        let SizeOffset(size, offset) = positions
            .get(&TypeId::of::<U>())
            .unwrap_or_else(missing::<U, _>);
        let key = T::pack(&self.value, &self.store);
        let value = (key.packed >> offset) & ((1 << size) - 1);
        self.store.pipeline_key(value)
    }

    pub fn construct<K: KeyTypeConcrete>(&'a self, value: K) -> PipelineKey<'a, K> {
        PipelineKey {
            store: self.store,
            value,
        }
    }

    pub fn shader_defs(&'a self) -> Vec<ShaderDefVal> {
        debug!(
            "{} shader defs, positions: {:?}",
            type_name::<T>(),
            T::positions(&self.store)
        );
        T::shader_defs(T::pack(&self.value, &self.store).packed, &self.store)
    }
}

#[derive(Component, Default)]
pub struct PipelineKeys {
    packed_keys: HashMap<TypeId, (KeyPrimitive, u8)>,
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

    pub fn get_packed_key<K: AnyKeyType + KeyTypeConcrete>(&self) -> Option<PackedPipelineKey<K>> {
        let (raw, size) = self.get_raw_and_size_by_id(&TypeId::of::<K>())?;
        Some(PackedPipelineKey::new(raw, size))
    }

    pub fn set_raw<K: AnyKeyType>(&mut self, value: KeyPrimitive, size: u8) {
        self.packed_keys.insert(TypeId::of::<K>(), (value, size));
    }

    pub fn get_key<'a, K: AnyKeyType + KeyTypeConcrete>(
        &self,
        cache: &'a PipelineCache,
    ) -> Option<PipelineKey<'a, K>> {
        Some(cache.key_store().pipeline_key(self.get_raw::<K>()?))
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
        app.sub_app_mut(RenderApp)
            .insert_resource(KeyMetaStoreInitializer(Some(KeyMetaStore::default())));
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
    ) -> Option<Self>
    where
        Self: Sized;
}

// #[derive(PipelineKey)]
// [dynamic_key]
// pub struct MyKey(u32);
pub trait DynamicKey: AnyKeyType + KeyTypeConcrete {}

pub trait AddPipelineKey {
    fn register_key<K: KeyTypeConcrete + FixedSizeKey + 'static>(&mut self) -> &mut Self;
    fn register_system_key<K: SystemKey, F: ReadOnlyWorldQuery + 'static>(&mut self) -> &mut Self;
    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self;
    fn register_dynamic_key<K: DynamicKey, F: ReadOnlyWorldQuery + 'static>(&mut self)
        -> &mut Self;
    fn register_dynamic_key_part<K: DynamicKey, PART: AnyKeyType>(&mut self) -> &mut Self;
}

impl AddPipelineKey for App {
    fn register_key<K: KeyTypeConcrete + FixedSizeKey + 'static>(&mut self) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStoreInitializer>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .0
            .as_mut()
            .expect("keys must be registered before RenderPlugin::finish")
            .register_fixed_size::<K>();
        self
    }

    fn register_system_key<K: SystemKey, F: ReadOnlyWorldQuery + 'static>(&mut self) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStoreInitializer>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .0
            .as_mut()
            .expect("keys must be registered before RenderPlugin::finish")
            .register_fixed_size::<K>();
        self.add_systems(
            Render,
            (|p: StaticSystemParam<K::Param>,
              cache: Res<PipelineCache>,
              mut q: Query<(&mut PipelineKeys, K::Query), F>| {
                let p = p.into_inner();
                for (mut keys, query) in q.iter_mut() {
                    let Some(key) = K::from_params(&p, query) else {
                        continue;
                    };
                    let PackedPipelineKey { packed, size, .. } = cache.pack_key(&key);
                    keys.set_raw::<K>(packed, size);
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::PrepareKeys),
        );
        self
    }

    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStoreInitializer>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .0
            .as_mut()
            .expect("keys must be registered before RenderPlugin::finish")
            .register_dynamic::<K>();
        self.add_systems(
            Render,
            (|mut q: Query<&mut PipelineKeys, F>| {
                for mut keys in q.iter_mut() {
                    if let Some(PackedPipelineKey { packed, size, .. }) = K::from_keys(&keys) {
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
            .get_resource_mut::<KeyMetaStoreInitializer>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .0
            .as_mut()
            .expect("keys must be registered before RenderPlugin::finish")
            .register_dynamic::<K>();
        self.add_systems(
            Render,
            (|cache: Res<PipelineCache>, mut q: Query<&mut PipelineKeys, F>| {
                let dynamic_components = &cache.key_store().meta::<K>().dynamic_components;
                'ent: for mut keys in q.iter_mut() {
                    let mut value = 0;
                    let mut size = 0;
                    for (id, so) in dynamic_components.iter() {
                        let Some((part, part_size)) = keys.get_raw_and_size_by_id(id) else {
                            break 'ent;
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
        let mut init = self
            .world
            .get_resource_mut::<KeyMetaStoreInitializer>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin");
        let store = init
            .0
            .as_mut()
            .expect("keys must be registered before RenderPlugin::finish");

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
        #[derive(PipelineKey, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[custom_shader_defs]
        pub struct $key(pub bool);
        impl SystemKey for $key {
            type Param = ();
            type Query = bevy_ecs::prelude::Has<$component>;

            fn from_params(_: &(), has_component: bool) -> Option<Self> {
                Some(Self(has_component))
            }
        }

        impl KeyShaderDefs for $key {
            fn shader_defs(&self) -> Vec<bevy_render::render_resource::ShaderDefVal> {
                if self.0 {
                    vec![$def.into()]
                } else {
                    vec![]
                }
            }
        }

        impl $key {
            pub fn enabled(&self) -> bool {
                self.0
            }
        }
    };
}
