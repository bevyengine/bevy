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

mod composite;
struct KeyMeta {
    dynamic_components: HashMap<TypeId, SizeOffset>,
    size: u8,
}

impl KeyMeta {
    fn new<K: KeyType + KeyTypeStatic>(store: &KeyMetaStore) -> Self {
        let size = K::size(store);
        Self {
            dynamic_components: Default::default(),
            size,
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
    pub fn register<K: KeyType + KeyTypeStatic>(&mut self) {
        self.metas.insert(TypeId::of::<K>(), KeyMeta::new::<K>(&self));
    }

    pub fn register_composite<K: KeyType + KeyTypeStatic>(&mut self) {
        self.register::<K>();
        self.unfinalized.insert(TypeId::of::<K>());
    }

    fn try_meta<K: KeyType>(&self) -> Option<&KeyMeta> {
        self.metas.get(&TypeId::of::<K>())
    }

    fn meta<K: KeyType>(&self) -> &KeyMeta {
        self.try_meta::<K>().unwrap_or_else(missing::<K, _>)
    }

    fn meta_mut<K: KeyType>(&mut self) -> &mut KeyMeta {
        self.metas.get_mut(&TypeId::of::<K>()).unwrap_or_else(missing::<K, _>)
    }

    pub fn add_dynamic_part<K: KeyType, PART: KeyType>(&mut self) {
        if !self.unfinalized.contains(&TypeId::of::<K>()) {
            panic!("{} is not dynamic, or keystore is already finalized", type_name::<K>());
        }
        if !self.metas.contains_key(&TypeId::of::<PART>()) {
            return missing::<PART, _>();
        }

        self.meta_mut::<K>().dynamic_components.insert(TypeId::of::<K>(), SizeOffset(u8::MAX, u8::MAX));
    }

    pub fn size_for_id(&self, id: &TypeId) -> u8 {
        self.metas.get(id).unwrap_or_else(missing_id).size
    }

    pub fn pipeline_key<K: KeyType + KeyTypeStatic>(&self, value: u32) -> Option<PipelineKey<K>> {
        let value = K::unpack(value, self);
        Some(PipelineKey {
            store: self,
            value,
        })
    }

    pub fn finalize(&mut self) {
        let mut todo = self.unfinalized.clone();
        let mut count = todo.len();
        while count > 0 {
            todo.retain(|k| {
                let (k, mut v) = self.metas.remove_entry(k).unwrap();
                if v.dynamic_components.keys().any(|k| self.unfinalized.contains(k)) {
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

pub trait KeyTypeStatic {
    fn unpack(value: u32, store: &KeyMetaStore) -> Self;

    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset>;

    fn size(store: &KeyMetaStore) -> u8 {
        Self::positions(store).values().map(|so| so.0).sum()
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> (u32, u8);
}

pub trait KeyType: Any + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone, Copy, Debug)]
pub struct SizeOffset(pub u8, pub u8);

#[derive(Debug)]
pub struct PipelineKey<'a, T: KeyType> {
    store: &'a KeyMetaStore,
    value: T,
}

impl<'a, T: KeyType> Deref for PipelineKey<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T: KeyType + PartialEq> PartialEq for PipelineKey<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<'a, T: KeyType + KeyTypeStatic> PipelineKey<'a, T> {
    pub fn extract<U: KeyType + KeyTypeStatic>(&'a self) -> Option<PipelineKey<'a, U>> {
        let positions = T::positions(self.store);
        let SizeOffset(size, offset) = positions.get(&TypeId::of::<U>())?;
        let (value, _) = T::pack(&self.value, &self.store);
        let value = (value >> offset) & ((1 << size) - 1);
        self.store.pipeline_key(value)
    }
}


#[derive(Component, Default)]
pub struct PipelineKeys {
    values_and_sizes: HashMap<TypeId, (u32, u8)>,
    shader_defs: Vec<ShaderDefVal>,
}

impl PipelineKeys {
    pub fn get_raw_by_id(&self, id: &TypeId) -> Option<u32> {
        self.values_and_sizes.get(id).map(|(v, _)| *v)
    }

    pub fn get_raw<K: KeyType>(&self) -> Option<u32> {
        self.get_raw_by_id(&TypeId::of::<K>())
    }

    pub fn get_raw_and_size_by_id(&self, id: &TypeId) -> Option<(u32, u8)> {
        self.values_and_sizes.get(id).copied()
    }

    pub fn get_raw_and_size<K: KeyType>(&self) -> Option<(u32, u8)> {
        self.get_raw_and_size_by_id(&TypeId::of::<K>())
    }

    pub fn set_raw<K: KeyType>(&mut self, value: u32, size: u8) {
        self.values_and_sizes.insert(TypeId::of::<K>(), (value, size));
    }

    // fn set_part_at_size_offset<K: KeyType>(
    //     &mut self,
    //     part_value: u32,
    //     size_offset: SizeOffset,
    // ) {
    //     let value = self.keys.entry(TypeId::of::<K>()).or_default();
    //     *value &= !(((1 << size_offset.0) - 1) << size_offset.1);
    //     *value |= part_value << size_offset.1;
    // }

    pub fn get_key<'a, K: KeyType + KeyTypeStatic>(&self, store: &'a KeyMetaStore) -> Option<PipelineKey<'a, K>> {
        store.pipeline_key(self.get_raw::<K>()?)
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

pub trait WorldKey: KeyType + KeyTypeStatic + Default {
    type Param: SystemParam + 'static;
    type Query: ReadOnlyWorldQuery + 'static;

    fn from_params(
        params: &SystemParamItem<Self::Param>,
        query_item: QueryItem<Self::Query>,
    ) -> Self;

    fn shader_defs(&self) -> Vec<ShaderDefVal>;
}

pub trait DynamicKey: KeyType + KeyTypeStatic + Default {
}

pub trait AddPipelineKey {
    fn register_world_key<K: WorldKey, F: ReadOnlyWorldQuery + 'static>(
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
    fn register_world_key<K: WorldKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self {
        self.world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin")
            .register::<K>();
        self.add_systems(
            Render,
            (|p: StaticSystemParam<K::Param>, store: Res<KeyMetaStore>, mut q: Query<(&mut PipelineKeys, K::Query), F>| {
                let p = p.into_inner();
                for (mut keys, query) in q.iter_mut() {
                    let key = K::from_params(&p, query);
                    keys.shader_defs.extend(key.shader_defs());
                    let (key, size) = K::pack(&key, &store);
                    keys.set_raw::<K>(key, size);
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
        let mut store = self
            .world
            .get_resource_mut::<KeyMetaStore>()
            .expect("should be run on the RenderApp after adding the PipelineKeyPlugin");

        store.register_composite::<K>();
        self.add_systems(
            Render,
            (|mut q: Query<&mut PipelineKeys, F>| {
                for mut keys in q.iter_mut() {
                    if let Some((value, size)) = K::from_keys(&keys) {
                        keys.set_raw::<K>(value, size);
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
            .register_composite::<K>();
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

    fn register_dynamic_key_part<K: DynamicKey, PART: KeyType>(&mut self) -> &mut Self {
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

impl KeyType for bool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeStatic for bool {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(1, 0))])
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> (u32, u8) {
        if *value {
            (1, 1) 
        } else {
            (0, 1)
        }
    }

    fn unpack(value: u32, _: &KeyMetaStore) -> Self {
        value != 0
    }    
}
