use crate::{render_resource::ShaderDefVal, *};
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, query::*, system::*};
pub use bevy_render_macros::PipelineKey;
use bevy_utils::{HashMap, HashSet};
use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
};

/// provides the means to turn a raw u32 into a key
#[derive(Resource, Default)]
pub struct KeyMetaStore {
    data: HashMap<TypeId, Box<dyn KeyType>>,
}

impl std::fmt::Debug for KeyMetaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyMetaStore")
            .field("data", &"...")
            .finish()
    }
}

impl KeyMetaStore {
    pub fn put<T: KeyType>(&mut self, t: T) {
        let type_id = TypeId::of::<T>();
        self.data.insert(type_id, Box::new(t));
    }

    pub fn try_borrow<T: KeyType>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.data
            .get(&type_id)
            .and_then(|b| b.as_any().downcast_ref())
    }

    pub fn borrow<T: KeyType>(&self) -> &T {
        self.try_borrow().unwrap_or_else(|| {
            panic!(
                "required type {} is not present in container",
                type_name::<T>()
            )
        })
    }

    pub fn size_for_id(&self, id: &TypeId) -> Option<u8> {
        Some(self.data.get(id)?.size())
    }

    pub fn key_from<U: KeyType>(&self, value: u32) -> PipelineKey<U> {
        let datamap = self.borrow::<U>().positions();
        PipelineKey {
            store: self,
            datamap,
            value,
            _p: Default::default(),
        }
    }

    pub fn finalize(&mut self) {
        let mut todo = self.data.keys().copied().collect::<HashSet<_>>();
        let mut count = todo.len();
        while count > 0 {
            todo.retain(|k| {
                let (k, mut v) = self.data.remove_entry(k).unwrap();
                let result = v.finalize(&self);
                self.data.insert(k, v);
                !result
            });

            if count == todo.len() {
                panic!("circular key reference: {todo:?}");
            }
            count = todo.len();
        }
    }
}

pub trait KeyType: Any + Send + Sync + 'static {
    fn positions(&self) -> HashMap<TypeId, SizeOffset>;

    fn size(&self) -> u8 {
        self.positions().values().map(|so| so.0).sum()
    }

    #[allow(unused_variables)]
    fn finalize(&mut self, store: &KeyMetaStore) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Clone, Copy, Debug)]
pub struct SizeOffset(pub u8, pub u8);

#[derive(Debug)]
pub struct PipelineKey<'a, T: KeyType> {
    store: &'a KeyMetaStore,
    datamap: HashMap<TypeId, SizeOffset>,
    value: u32,
    _p: PhantomData<fn() -> T>,
}

impl<'a, T: KeyType> PartialEq for PipelineKey<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_value() == other.raw_value()
    }
}

impl<'a, T: KeyType> PipelineKey<'a, T> {
    pub fn extract<U: KeyType>(&'a self) -> Option<PipelineKey<'a, U>> {
        let SizeOffset(size, offset) = self.datamap.get(&TypeId::of::<U>())?;

        let value = (self.value >> offset) & ((1 << size) - 1);
        Some(PipelineKey {
            store: self.store,
            datamap: self.store.borrow::<U>().positions(),
            value,
            _p: PhantomData,
        })
    }

    pub fn raw_value(&self) -> u32 {
        self.value
    }
}

impl<K1: KeyType, K2: KeyType> KeyType for (K1, K2) {
    fn positions(&self) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let s0 = self.0.size();
        result.insert(TypeId::of::<K1>(), SizeOffset(s0, 0));
        let s1 = self.1.size();
        result.insert(TypeId::of::<K2>(), SizeOffset(s1, s0));
        result
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct NoKey;
impl KeyType for NoKey {
    fn positions(&self) -> HashMap<TypeId, SizeOffset> {
        HashMap::default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<K1: KeyType + Default, K2: KeyType + Default> CompositeKey for (K1, K2) {
    type K1 = K1;
    type K2 = K2;
    type K3 = NoKey;
    type K4 = NoKey;
    type K5 = NoKey;
    type K6 = NoKey;
    type K7 = NoKey;
    type K8 = NoKey;
    type K9 = NoKey;
    type K10 = NoKey;
    type K11 = NoKey;
}

impl<K1: KeyType + Default, K2: KeyType + Default, K3: KeyType + Default, K4: KeyType + Default, 
K5: KeyType + Default, K6: KeyType + Default, K7: KeyType + Default, K8: KeyType + Default, 
K9: KeyType + Default, K10: KeyType + Default, K11: KeyType + Default> KeyType for (K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11) {
    fn positions(&self) -> HashMap<TypeId, SizeOffset> {
        macro_rules! step {
            ($i:expr, $k:ident, $offset:ident, $result:ident) => {
                let size = $i.size();
                $result.insert(TypeId::of::<$k>(), SizeOffset(size, $offset));
                $offset += size;
            }
        }

        let mut result = HashMap::default();
        let mut offset = 0;

        step!(self.0, K1, offset, result);
        step!(self.1, K2, offset, result);
        step!(self.2, K3, offset, result);
        step!(self.3, K4, offset, result);
        step!(self.4, K5, offset, result);
        step!(self.5, K6, offset, result);
        step!(self.6, K7, offset, result);
        step!(self.7, K8, offset, result);
        step!(self.8, K9, offset, result);
        step!(self.9, K10, offset, result);
        step!(self.10, K11, offset, result);

        let _ = offset;

        result
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<K1: KeyType + Default, K2: KeyType + Default, K3: KeyType + Default, K4: KeyType + Default, 
    K5: KeyType + Default, K6: KeyType + Default, K7: KeyType + Default, K8: KeyType + Default, 
    K9: KeyType + Default, K10: KeyType + Default, K11: KeyType + Default> CompositeKey for (K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11) {
    type K1 = K1;
    type K2 = K2;
    type K3 = K3;
    type K4 = K4;
    type K5 = K5;
    type K6 = K6;
    type K7 = K7;
    type K8 = K8;
    type K9 = K9;
    type K10 = K10;
    type K11 = K11;
}

pub struct DynKey<T> {
    positions: HashMap<TypeId, Option<SizeOffset>>,
    _p: PhantomData<fn() -> T>,
}

impl<T> Default for DynKey<T> {
    fn default() -> Self {
        Self {
            positions: Default::default(),
            _p: Default::default(),
        }
    }
}
impl<T: 'static> DynamicKey for DynKey<T> {
    fn append(&mut self, id: TypeId) {
        self.positions.insert(id, None);
    }
}

impl<T: 'static> KeyType for DynKey<T> {
    fn positions(&self) -> HashMap<TypeId, SizeOffset> {
        self.positions
            .iter()
            .map(|(k, v)| (*k, v.unwrap()))
            .collect()
    }

    fn finalize(&mut self, store: &KeyMetaStore) -> bool {
        println!("dyn finalize {:?}", self.positions);
        let mut offset = self
            .positions
            .values()
            .flatten()
            .map(|so| so.0 + so.1)
            .max()
            .unwrap_or(0);
        for (k, v) in self.positions.iter_mut() {
            if v.is_none() {
                let Some(size) = store.size_for_id(k) else {
                    return false;
                };

                *v = Some(SizeOffset(size, offset));
                offset += size;
            }
        }
        println!("done! {:?}", self.positions);
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Component, Default)]
pub struct PipelineKeys {
    keys: HashMap<TypeId, u32>,
    shader_defs: Vec<ShaderDefVal>,
}

impl PipelineKeys {
    pub fn get_raw_by_id(&self, id: &TypeId) -> Option<u32> {
        self.keys.get(id).copied()
    }

    pub fn get_raw<K: KeyType>(&self) -> Option<u32> {
        self.keys.get(&TypeId::of::<K>()).copied()
    }

    pub fn set_raw<K: KeyType>(&mut self, value: u32) {
        self.keys.insert(TypeId::of::<K>(), value);
    }

    pub fn set_part_at_size_offset<K: KeyType>(
        &mut self,
        part_value: u32,
        size_offset: SizeOffset,
    ) {
        let value = self.keys.entry(TypeId::of::<K>()).or_default();
        *value &= !(((1 << size_offset.0) - 1) << size_offset.1);
        *value |= part_value << size_offset.1;
    }

    pub fn set_part_with_meta<K: KeyType, PART: KeyType>(&mut self, meta: &K, part_value: u32) {
        let so = *meta
            .positions()
            .get(&TypeId::of::<PART>())
            .unwrap_or_else(|| {
                panic!(
                    "{} not registered in {}",
                    type_name::<PART>(),
                    type_name::<K>()
                )
            });
        self.set_part_at_size_offset::<K>(part_value, so);
    }

    pub fn set_part<K: KeyType, PART: KeyType>(&mut self, store: &KeyMetaStore, part_value: u32) {
        self.set_part_with_meta::<K, PART>(store.borrow::<K>(), part_value);
    }

    pub fn get_key<'a, K: KeyType>(&self, store: &'a KeyMetaStore) -> Option<PipelineKey<'a, K>> {
        Some(store.key_from(self.get_raw::<K>()?))
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

pub trait WorldKey: KeyType + Default {
    type Param: SystemParam + 'static;
    type Query: ReadOnlyWorldQuery + 'static;

    fn from_params(
        params: &SystemParamItem<Self::Param>,
        query_item: QueryItem<Self::Query>,
    ) -> Self;

    fn shader_defs(&self) -> Vec<ShaderDefVal>;
}

pub trait CompositeKey: KeyType + Default {
    type K1: KeyType;
    type K2: KeyType;
    type K3: KeyType;
    type K4: KeyType;
    type K5: KeyType;
    type K6: KeyType;
    type K7: KeyType;
    type K8: KeyType;
    type K9: KeyType;
    type K10: KeyType;
    type K11: KeyType;
}

pub trait DynamicKey: KeyType + Default {
    fn append(&mut self, id: TypeId);
}

pub trait AddPipelineKey {
    fn register_world_key<K: WorldKey + Into<u32>, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self;
    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self;
    fn register_dynamic_key<K: DynamicKey, F: ReadOnlyWorldQuery + 'static>(&mut self)
        -> &mut Self;
    fn register_dynamic_key_part<K: DynamicKey, PART: KeyType>(&mut self) -> &mut Self;
}

impl AddPipelineKey for App {
    fn register_world_key<K: WorldKey + Into<u32>, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .put(K::default());
        self.add_systems(
            Render,
            (|p: StaticSystemParam<K::Param>, mut q: Query<(&mut PipelineKeys, K::Query), F>| {
                let p = p.into_inner();
                for (mut keys, query) in q.iter_mut() {
                    let key = K::from_params(&p, query);
                    keys.shader_defs.extend(key.shader_defs());
                    let key = key.into();
                    keys.set_raw::<K>(key);
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::Queue),
        );
        self.add_systems(
            Render,
            (|mut commands: Commands, q: Query<Entity, F>| {
                for ent in q.iter() {
                    commands.entity(ent).insert(PipelineKeys::default());
                }
            })
            .in_set(RenderSet::Prepare),
        );
        self
    }

    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        let mut store = self
            .world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin");

        store.put(K::default());
        self.add_systems(
            Render,
            (|store: Res<KeyMetaStore>, mut q: Query<&mut PipelineKeys, F>| {
                for mut keys in q.iter_mut() {
                    let Some(k1) = keys.get_raw::<K::K1>() else { continue; };
                    keys.set_part::<K, K::K1>(&store, k1);
                    let Some(k2) = keys.get_raw::<K::K2>() else { continue; };
                    keys.set_part::<K, K::K2>(&store, k2);
                    let Some(k3) = keys.get_raw::<K::K3>() else { continue; };
                    keys.set_part::<K, K::K3>(&store, k3);
                    let Some(k4) = keys.get_raw::<K::K4>() else { continue; };
                    keys.set_part::<K, K::K4>(&store, k4);
                    let Some(k5) = keys.get_raw::<K::K5>() else { continue; };
                    keys.set_part::<K, K::K5>(&store, k5);
                    let Some(k6) = keys.get_raw::<K::K6>() else { continue; };
                    keys.set_part::<K, K::K6>(&store, k6);
                    let Some(k7) = keys.get_raw::<K::K7>() else { continue; };
                    keys.set_part::<K, K::K7>(&store, k7);
                    let Some(k8) = keys.get_raw::<K::K8>() else { continue; };
                    keys.set_part::<K, K::K8>(&store, k8);
                    let Some(k9) = keys.get_raw::<K::K9>() else { continue; };
                    keys.set_part::<K, K::K9>(&store, k9);
                    let Some(k10) = keys.get_raw::<K::K10>() else { continue; };
                    keys.set_part::<K, K::K10>(&store, k10);
                    let Some(k11) = keys.get_raw::<K::K11>() else { continue; };
                    keys.set_part::<K, K::K11>(&store, k11);
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::Queue),
        );
        self.configure_sets(
            Render,
            KeySetMarker::<K>::default()
            .after(KeySetMarker::<K::K1>::default())
            .after(KeySetMarker::<K::K2>::default())
            .after(KeySetMarker::<K::K3>::default())
            .after(KeySetMarker::<K::K4>::default())
            .after(KeySetMarker::<K::K5>::default())
            .after(KeySetMarker::<K::K6>::default())
            .after(KeySetMarker::<K::K7>::default())
            .after(KeySetMarker::<K::K8>::default())
            .after(KeySetMarker::<K::K9>::default())
            .after(KeySetMarker::<K::K10>::default())
            .after(KeySetMarker::<K::K11>::default()),
        );
        self
    }

    fn register_dynamic_key<K: DynamicKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .put(K::default());
        self.add_systems(
            Render,
            (|store: Res<KeyMetaStore>, mut q: Query<&mut PipelineKeys, F>| {
                let meta = store.borrow::<K>();
                for mut keys in q.iter_mut() {
                    let Some(parts) = meta
                        .positions()
                        .iter()
                        .map(|(id, so)| keys.get_raw_by_id(id).map(|raw| (raw, *so)))
                        .collect::<Option<Vec<(u32, SizeOffset)>>>()
                    else {
                        continue;
                    };

                    for (value, so) in parts {
                        keys.set_part_at_size_offset::<K>(value, so);
                    }
                }
            })
            .in_set(KeySetMarker::<K>::default())
            .in_set(RenderSet::Queue),
        );
        self
    }

    fn register_dynamic_key_part<K: DynamicKey, PART: KeyType>(&mut self) -> &mut Self {
        let mut store = self.world.resource_mut::<KeyMetaStore>();
        let k = store.data.get_mut(&TypeId::of::<K>()).unwrap();
        let k = k.as_any_mut().downcast_mut::<K>().unwrap();
        k.append(TypeId::of::<PART>());
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
            PipelineKey, Default, Clone, Copy, num_enum::FromPrimitive, num_enum::IntoPrimitive,
        )]
        #[repr(u32)]
        pub enum $key {
            #[default]
            Off,
            On,
        }
        impl WorldKey for $key {
            type Param = ();
            type Query = bevy_ecs::prelude::Has<$component>;

            fn from_params(_: &(), has_component: bool) -> Self {
                match has_component {
                    true => $key::On,
                    false => $key::Off,
                }
            }

            fn shader_defs(&self) -> Vec<bevy_render::render_resource::ShaderDefVal> {
                if matches!(self, $key::On) {
                    vec![$def.into()]
                } else {
                    vec![]
                }
            }
        }
    };
}

pub trait FixedSizePipelineKey {
    fn size() -> u8;
}

impl FixedSizePipelineKey for bool {
    fn size() -> u8 { 1 }
}
