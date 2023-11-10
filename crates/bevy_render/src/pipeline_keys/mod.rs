use crate::{render_resource::ShaderDefVal, *};
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, query::*, schedule::NodeConfigs, system::*};
pub use bevy_render_macros::PipelineKey;
use bevy_utils::{intern::Interned, HashMap, HashSet};
use std::{
    any::{type_name, TypeId},
    marker::PhantomData,
};

mod composite;
mod packed_types;

/// A pipeline key is a struct or enum implementing the `PipelineKeyType` trait, which provides methods to convert it
/// into a packed set of bits in a `PackedPipelineKey` struct, and back again.
/// Pipeline keys are used to cache pipeline variants, so it must be possible to obtain all information that can change
/// the pipeline (shader defs, bindings, outputs, entry points, etc) from the pipeline's key value.
/// Keys may specify a set of `ShaderDefVal`s (based on the key value) that can be applied to the vertex and fragment states
/// of a `RenderPipelineDescriptor` or `ComputePipelineDescriptor`, via the `KeyShaderDefs` trait.
/// Key values can be calculated independently for a given entity via the `SystemKey` trait.
/// Keys can also be composites of other keys, either as simple tuples, as structs, or as dynamic keys, the components of which
/// can be independently registered to allow extensibility.

/// The primitive type underlying packed pipeline keys.
pub type KeyPrimitive = u128;

/// Main trait for a pipeline key struct. This trait implementation will typically be derived with
/// #[derive(PipelineKey)]
pub trait PipelineKeyType: 'static {
    /// Pack an instance of the type into a `KeyPrimitive` and store in a typed `PackedPipelineKey` struct
    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self>
    where
        Self: Sized;

    /// unpack a `KeyPrimitive` and return an instance of the original type
    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self;

    /// For composite keys (structs, tuples and dynamic keys), returns a `HashMap` containing the `TypeId`s of
    /// component key types within this key, and the bitsize and offset into the `KeyPrimitive` at which
    /// they are stored in the packed representation of this type.
    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset>;

    /// number of bits required to store this key type
    fn size(store: &KeyMetaStore) -> u8 {
        Self::positions(store).values().map(|so| so.size).sum()
    }

    /// `ShaderDefVal`s that correspond to the given packed value of this key type.
    /// The default implementation unpacks all components and returns the set of
    /// `ShaderDefVal`s of the components.
    fn shader_defs(value: KeyPrimitive, store: &KeyMetaStore) -> Vec<ShaderDefVal> {
        let mut defs = Vec::default();
        for (id, so) in Self::positions(store) {
            let Some(part_def_fn) = store.def_fn_for_id(&id) else {
                debug!("{id:?} not registered for shader defs");
                continue;
            };
            let part_value = (value >> so.offset) & ((1 << so.size) - 1);
            defs.extend(part_def_fn(part_value, store));
        }

        defs
    }
}

/// A packed representation of a `PipelineKeyType` value.
#[derive(Debug)]
pub struct PackedPipelineKey<T: PipelineKeyType> {
    pub packed: KeyPrimitive,
    pub size: u8,
    _p: PhantomData<fn() -> T>,
}

impl<T: PipelineKeyType> Clone for PackedPipelineKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: PipelineKeyType> Copy for PackedPipelineKey<T> {}

impl<T: PipelineKeyType> PackedPipelineKey<T> {
    pub fn new(packed: KeyPrimitive, size: u8) -> Self {
        assert!((size as u32) < KeyPrimitive::BITS);
        Self {
            packed,
            size,
            _p: Default::default(),
        }
    }

    /// Helper function to set the value for a component of a composite `PipelineKeyType`
    /// to a new value
    pub fn insert<K: PipelineKeyType>(&mut self, value: &K, cache: &PipelineCache) {
        let store = cache.key_store();
        self.insert_packed(K::pack(value, store), cache);
    }

    /// Helper function to set the value for a component of a composite `PipelineKeyType`
    /// to a new packed value
    pub fn insert_packed<K: PipelineKeyType>(
        &mut self,
        value: PackedPipelineKey<K>,
        cache: &PipelineCache,
    ) {
        let store = cache.key_store();
        let positions = T::positions(store);
        let so = positions.get(&TypeId::of::<K>()).unwrap();
        self.packed &= !(((1 << so.size) - 1) << so.offset);
        self.packed |= value.packed << so.offset;
    }
}

/// `PipelineKeyType`s with manual implementations of this trait can provide shader defs that may be used
/// by the pipeline to specify the shader defs on the pipeline's shader.
/// To specify shader defs, the derive macro must have a `#[custom_shader_defs]` attribute, and must
/// be registered via the `AddPipelineKey` trait of the `App`.
pub trait KeyShaderDefs {
    fn shader_defs(&self) -> Vec<ShaderDefVal>;
}

/// Provides a means to construct a packed composite pipeline key from the packed components.
/// This trait is implemented for tuple and struct composite keys.
pub trait KeyRepack: PipelineKeyType {
    /// For a composite tuple the `PackedParts` will be a tuple of `PackedPipelineKey`s corresponding
    /// to the tuple members.
    /// For a composite struct, the `PackedParts` for a struct will be a tuple of `PackedPipelineKey`s
    /// corresponding to the struct fields.
    type PackedParts;

    /// Construct a composite `PackedPipelineKey` from packed components.
    fn repack(source: Self::PackedParts) -> PackedPipelineKey<Self>
    where
        Self: Sized;
}

/// Component for storing packed `PipelineKeyType` values on an entity.
#[derive(Component, Default)]
pub struct PipelineKeys {
    packed_keys: HashMap<TypeId, (KeyPrimitive, u8)>,
}

impl PipelineKeys {
    /// Get the `KeyPrimitive` value for a given `TypeId`
    pub fn get_raw_by_id(&self, id: &TypeId) -> Option<KeyPrimitive> {
        self.packed_keys.get(id).map(|(v, _)| *v)
    }

    /// Get the `KeyPrimitive` value for a given `PipelineKeyType`
    pub fn get_raw<K: PipelineKeyType>(&self) -> Option<KeyPrimitive> {
        self.get_raw_by_id(&TypeId::of::<K>())
    }

    /// Get the `KeyPrimitive` value and size for a given `TypeId`
    pub fn get_raw_and_size_by_id(&self, id: &TypeId) -> Option<(KeyPrimitive, u8)> {
        self.packed_keys.get(id).copied()
    }

    /// Get the `KeyPrimitive` value and size for a given `PipelineKeyType`
    pub fn get_packed_key<K: PipelineKeyType>(&self) -> Option<PackedPipelineKey<K>> {
        let (raw, size) = self.get_raw_and_size_by_id(&TypeId::of::<K>())?;
        Some(PackedPipelineKey::new(raw, size))
    }

    /// Set the value of a given `PipelineKeyType` for the containing entity
    pub fn set_raw<K: PipelineKeyType>(&mut self, value: KeyPrimitive, size: u8) {
        self.packed_keys.insert(TypeId::of::<K>(), (value, size));
    }

    /// Get a `PipelineKey` struct containing a reconstructed instance of a given `PipelineKeyType` for the containing entity
    pub fn get_key<'a, K: PipelineKeyType>(
        &self,
        cache: &'a PipelineCache,
    ) -> Option<PipelineKey<'a, K>> {
        Some(cache.key_store().pipeline_key(self.get_raw::<K>()?))
    }
}

/// A wrapper around the underlying `PipelineKeyType` that also contains a reference to the `KeyMetaStore`,
/// allowing access to dynamic key components, and to `ShaderDefVal`s that the key's value (or components) specify.
#[derive(Debug, Copy, Clone)]
pub struct PipelineKey<'a, T: PipelineKeyType> {
    store: &'a KeyMetaStore,
    value: T,
}

impl<'a, T: PipelineKeyType> Deref for PipelineKey<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T: PipelineKeyType + PartialEq> PartialEq for PipelineKey<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<'a, T: PipelineKeyType> PipelineKey<'a, T> {
    /// Extract a `PipelineKey` for a given component of this key, if it is present.
    /// This is mainly useful for dynamic keys (on fixed keys we can just access the members directly if required),
    /// but may also be useful for obtaining a `PipelineKey` wrapper of a component.
    pub fn try_extract<U: PipelineKeyType>(&'a self) -> Option<PipelineKey<'a, U>> {
        let positions = T::positions(self.store);
        let SizeOffset { size, offset } = positions.get(&TypeId::of::<U>())?;
        let key = T::pack(&self.value, self.store);
        let value = (key.packed >> offset) & ((1 << size) - 1);
        Some(self.store.pipeline_key(value))
    }

    /// Extract a `PipelineKey` for a given component of this key, panics if not found.
    pub fn extract<U: PipelineKeyType>(&'a self) -> PipelineKey<'a, U> {
        let positions = T::positions(self.store);
        let SizeOffset { size, offset } = positions
            .get(&TypeId::of::<U>())
            .unwrap_or_else(missing::<U, _>);
        let key = T::pack(&self.value, self.store);
        let value = (key.packed >> offset) & ((1 << size) - 1);
        self.store.pipeline_key(value)
    }

    /// construct a `PipelineKey` for an arbitrary type, using the `KeyMetaStore` reference from this key.
    pub fn construct<K: PipelineKeyType>(&'a self, value: K) -> PipelineKey<'a, K> {
        PipelineKey {
            store: self.store,
            value,
        }
    }

    /// Get a list of shader defs corresponding to this key's value (and of any components)
    pub fn shader_defs(&'a self) -> Vec<ShaderDefVal> {
        debug!(
            "{} shader defs, positions: {:?}",
            type_name::<T>(),
            T::positions(self.store)
        );
        T::shader_defs(T::pack(&self.value, self.store).packed, self.store)
    }
}

/// Implemented for non-dynamic keys, allows access to the bitsize without providing the `KeyMetaStore`
pub trait FixedSizeKey: 'static {
    fn fixed_size() -> u8;
}

/// Specifies how to determine a key value for a given entity.
pub trait SystemKey: PipelineKeyType + FixedSizeKey {
    type Param: SystemParam + 'static;
    type Query: ReadOnlyWorldQuery + 'static;

    fn from_params(
        params: &SystemParamItem<Self::Param>,
        query_item: QueryItem<Self::Query>,
    ) -> Option<Self>
    where
        Self: Sized;
}

/// Build composite keys from their parts. parts must be system keys, or other composites.
/// Automatically implemented by the derive macro for structs and tuples
pub trait CompositeKey: PipelineKeyType {
    fn from_keys(keys: &PipelineKeys) -> Option<PackedPipelineKey<Self>>
    where
        Self: Sized;
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>>;
}

/// Marker trait for dynamic keys. This shouldn't be implemented directly, instead use :
/// ```no_compile
/// #[derive(PipelineKey)]
/// #[dynamic_key]
/// ```
pub trait DynamicKey: PipelineKeyType {}

/// App trait for registering `PipelineKeyType`s
pub trait AddPipelineKey {
    /// Register a key. This is required for a key to provide `ShaderDefVal`s.
    fn register_key<K: PipelineKeyType + FixedSizeKey + 'static>(&mut self) -> &mut Self;

    /// Register a `SystemKey`. The key's value will be generated and stored in the `PipelineKeys` component for entities
    /// matching the query filter `F`.
    fn register_system_key<K: SystemKey, F: ReadOnlyWorldQuery + 'static>(&mut self) -> &mut Self;

    /// Register a composite (tuple or struct) key. The key's value will be generated and stored in the `PipelineKeys`
    /// component for entities matching the query filter `F`, and where all the component keys are generated.
    fn register_composite_key<K: CompositeKey, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
    ) -> &mut Self;

    /// Register a dynamic key. The key's value will be generated and stored in the `PipelineKeys`
    /// component for entities matching the query filter `F`, and where all the component keys are generated.
    fn register_dynamic_key<K: DynamicKey, F: ReadOnlyWorldQuery + 'static>(&mut self)
        -> &mut Self;

    /// Add a component to a dynamic key.
    fn register_dynamic_key_part<K: DynamicKey, PART: PipelineKeyType>(&mut self) -> &mut Self;
}

// ---- implementation details that you probably don't need to know about below this line ---- //

type ShaderDefFn = fn(KeyPrimitive, &KeyMetaStore) -> Vec<ShaderDefVal>;

/// Information contained in the `KeyMetaStore` for each registered key.
/// Used by dynamic keys to access information about their components.
pub struct KeyMeta {
    /// function for obtaining the key's shader defs from the key value
    pub shader_def_fn: ShaderDefFn,
    /// components of the dynamic key with the given TypeId
    pub dynamic_components: HashMap<TypeId, SizeOffset>,
    /// bitsize of the key
    pub size: u8,
}

impl KeyMeta {
    fn new<K: PipelineKeyType + 'static>() -> Self {
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

    fn new_sized<K: PipelineKeyType + FixedSizeKey>() -> Self {
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

// Resource containing key meta information during app construction.
// The contents will be removed in `RenderPlugin::finish` and should not be used directly.
// Keys should instead be registered via the `AddPipelineKey` trait on the `App`.
#[derive(Resource)]
pub(crate) struct KeyMetaStoreInitializer(pub(crate) Option<KeyMetaStore>);

impl KeyMetaStoreInitializer {
    pub(crate) fn take_final(&mut self) -> KeyMetaStore {
        let mut store = self.0.take().expect("key store already taken");
        store.finalize();
        store
    }
}

/// Stores information about each registered key type. Users should not need to access this
/// directly, methods on `PipelineCache` or on the key traits should be used instead.
#[derive(Default)]
pub struct KeyMetaStore {
    // key meta information
    metas: HashMap<TypeId, KeyMeta>,
    // during startup, a set of registered keys with unknown sizes
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
    pub fn register_fixed_size<K: PipelineKeyType + FixedSizeKey>(&mut self) {
        self.metas
            .insert(TypeId::of::<K>(), KeyMeta::new_sized::<K>());
    }

    pub fn register_dynamic<K: PipelineKeyType + 'static>(&mut self) {
        self.metas.insert(TypeId::of::<K>(), KeyMeta::new::<K>());
        self.unfinalized.insert(TypeId::of::<K>());
    }

    pub fn try_meta<K: PipelineKeyType>(&self) -> Option<&KeyMeta> {
        self.metas.get(&TypeId::of::<K>())
    }

    pub fn meta<K: PipelineKeyType>(&self) -> &KeyMeta {
        self.try_meta::<K>().unwrap_or_else(missing::<K, _>)
    }

    fn meta_mut<K: PipelineKeyType>(&mut self) -> &mut KeyMeta {
        self.metas
            .get_mut(&TypeId::of::<K>())
            .unwrap_or_else(missing::<K, _>)
    }

    fn add_dynamic_part<K: PipelineKeyType, PART: PipelineKeyType>(&mut self) {
        if !self.unfinalized.contains(&TypeId::of::<K>()) {
            panic!(
                "{} is not dynamic, or keystore is already finalized",
                type_name::<K>()
            );
        }

        self.meta_mut::<K>().dynamic_components.insert(
            TypeId::of::<PART>(),
            SizeOffset {
                size: u8::MAX,
                offset: u8::MAX,
            },
        );
    }

    fn size_for_id(&self, id: &TypeId) -> u8 {
        self.metas.get(id).unwrap_or_else(|| missing_id(id)).size
    }

    fn def_fn_for_id(&self, id: &TypeId) -> Option<ShaderDefFn> {
        self.metas.get(id).map(|meta| meta.shader_def_fn)
    }

    pub(crate) fn pipeline_key<K: PipelineKeyType>(&self, value: KeyPrimitive) -> PipelineKey<K> {
        let value = K::unpack(value, self);
        PipelineKey { store: self, value }
    }

    fn finalize(&mut self) {
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
                    so.size = self.size_for_id(id);
                    so.offset = offset;
                    offset += so.size;
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

/// describes the location of a component within the packed representation of a composite key
#[derive(Clone, Copy, Debug)]
pub struct SizeOffset {
    pub size: u8,
    pub offset: u8,
}

#[derive(SystemSet)]
struct KeySetMarker<T>(PhantomData<fn() -> T>);

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
        Self(self.0)
    }
}

impl<T> Default for KeySetMarker<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Initializes the `KeyMetaStore` for `PipelineKey` registration.
pub struct PipelineKeyPlugin;

impl Plugin for PipelineKeyPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(KeyMetaStoreInitializer(Some(KeyMetaStore::default())));
        }
    }
}

impl AddPipelineKey for App {
    fn register_key<K: PipelineKeyType + FixedSizeKey + 'static>(&mut self) -> &mut Self {
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

                        value |= part << so.offset;
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

    fn register_dynamic_key_part<K: DynamicKey, PART: PipelineKeyType>(&mut self) -> &mut Self {
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

/// Generate a boolean pipeline key based on the presence of a marker component, and add a given `ShaderDefVal` when enabled.
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
