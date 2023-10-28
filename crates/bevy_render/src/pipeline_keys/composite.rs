use bevy_ecs::{prelude::*, schedule::NodeConfigs};
pub use bevy_render_macros::PipelineKey;
use bevy_utils::{HashMap, intern::Interned};
use std::any::{Any, TypeId};
use super::*;

#[allow(unused_imports)]
use bevy_utils::all_tuples;

pub trait CompositeKey: KeyType + Default {
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)>;
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>>;
}

#[allow(unused_macros)]
macro_rules! impl_composite_key_tuples {
    ($(($K:ident, $sz:ident, $selfdot:ident)),*) => {
        impl<$($K: KeyType),*> KeyType for ($($K,)*) {
            fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
                let mut result = HashMap::default();
                let mut offset = 0;

                $(
                    let $sz = store.size_for_id(&TypeId::of::<$K>());
                    offset += $sz;
                )*
        
                $(
                    offset -= $sz;
                    result.insert(TypeId::of::<$K>(), SizeOffset($sz, offset));
                )*
        
                result
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
    
            fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
                let mut result = 0u32;
                let mut total_size = 0u8;
    
                $(
                    let (value, size) = $selfdot.pack(store);
                    result = (result << size) | value;
                    total_size += size;
                )*
    
                (result, total_size)
            }
        }

        impl<$($K: KeyType + Default),*> CompositeKey for ($($K,)*) {
            fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
                let mut result = 0u32;
                let mut size = 0u8;
        
                $(
                    let (v, s) = keys.get_raw_and_size::<$K>()?;
                    result = result << s | v;
                    size = size + s;
                )*
        
                Some((result, size))
            }
        
            fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
                let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());

                $(
                    config = config.after(KeySetMarker::<$K>::default());
                )*

                config
            }        
        } 

        impl<$($K: KeyType + KeyTypeUnpack),*> KeyTypeUnpack for ($($K,)*) {
            fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
                let mut shift_bits = 0;
                $(
                    let $sz = $selfdot.size(store);
                    shift_bits += $sz;
                )*

                (
                    $({
                        shift_bits -= $sz;
                        $selfdot.unpack(value >> shift_bits, store)
                    }),*
                )
            }
        }
    }
}

// i don't know how to make this actually work, so i use macro expand and copy/paste
// all_tuples!(impl_composite_key_tuples, 2, 12, K, sz, selfdot);

// Recursive expansion of all_tuples! macro
// =========================================

impl<K0: KeyType, K1: KeyType> KeyType for (K0, K1) {
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<K0: KeyType + Default, K1: KeyType + Default> CompositeKey for (K0, K1) {
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config
    }
}
impl<K0: KeyType + KeyTypeUnpack, K1: KeyType + KeyTypeUnpack> KeyTypeUnpack for (K0, K1) {
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<K0: KeyType, K1: KeyType, K2: KeyType> KeyType for (K0, K1, K2) {
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<K0: KeyType + Default, K1: KeyType + Default, K2: KeyType + Default> CompositeKey
    for (K0, K1, K2)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config
    }
}
impl<K0: KeyType + KeyTypeUnpack, K1: KeyType + KeyTypeUnpack, K2: KeyType + KeyTypeUnpack>
    KeyTypeUnpack for (K0, K1, K2)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<K0: KeyType, K1: KeyType, K2: KeyType, K3: KeyType> KeyType for (K0, K1, K2, K3) {
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<K0: KeyType, K1: KeyType, K2: KeyType, K3: KeyType, K4: KeyType> KeyType
    for (K0, K1, K2, K3, K4)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<K0: KeyType, K1: KeyType, K2: KeyType, K3: KeyType, K4: KeyType, K5: KeyType> KeyType
    for (K0, K1, K2, K3, K4, K5)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<K0: KeyType, K1: KeyType, K2: KeyType, K3: KeyType, K4: KeyType, K5: KeyType, K6: KeyType>
    KeyType for (K0, K1, K2, K3, K4, K5, K6)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        let sz6 = store.size_for_id(&TypeId::of::<K6>());
        offset += sz6;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        offset -= sz6;
        result.insert(TypeId::of::<K6>(), SizeOffset(sz6, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.6.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
        K6: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5, K6)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K6>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config = config.after(KeySetMarker::<K6>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
        K6: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5, K6)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        let sz6 = self.6.size(store);
        shift_bits += sz6;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz6;
                self.6.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<
        K0: KeyType,
        K1: KeyType,
        K2: KeyType,
        K3: KeyType,
        K4: KeyType,
        K5: KeyType,
        K6: KeyType,
        K7: KeyType,
    > KeyType for (K0, K1, K2, K3, K4, K5, K6, K7)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        let sz6 = store.size_for_id(&TypeId::of::<K6>());
        offset += sz6;
        let sz7 = store.size_for_id(&TypeId::of::<K7>());
        offset += sz7;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        offset -= sz6;
        result.insert(TypeId::of::<K6>(), SizeOffset(sz6, offset));
        offset -= sz7;
        result.insert(TypeId::of::<K7>(), SizeOffset(sz7, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.6.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.7.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
        K6: KeyType + Default,
        K7: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5, K6, K7)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K6>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K7>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config = config.after(KeySetMarker::<K6>::default());
        config = config.after(KeySetMarker::<K7>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
        K6: KeyType + KeyTypeUnpack,
        K7: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5, K6, K7)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        let sz6 = self.6.size(store);
        shift_bits += sz6;
        let sz7 = self.7.size(store);
        shift_bits += sz7;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz6;
                self.6.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz7;
                self.7.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<
        K0: KeyType,
        K1: KeyType,
        K2: KeyType,
        K3: KeyType,
        K4: KeyType,
        K5: KeyType,
        K6: KeyType,
        K7: KeyType,
        K8: KeyType,
    > KeyType for (K0, K1, K2, K3, K4, K5, K6, K7, K8)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        let sz6 = store.size_for_id(&TypeId::of::<K6>());
        offset += sz6;
        let sz7 = store.size_for_id(&TypeId::of::<K7>());
        offset += sz7;
        let sz8 = store.size_for_id(&TypeId::of::<K8>());
        offset += sz8;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        offset -= sz6;
        result.insert(TypeId::of::<K6>(), SizeOffset(sz6, offset));
        offset -= sz7;
        result.insert(TypeId::of::<K7>(), SizeOffset(sz7, offset));
        offset -= sz8;
        result.insert(TypeId::of::<K8>(), SizeOffset(sz8, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.6.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.7.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.8.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
        K6: KeyType + Default,
        K7: KeyType + Default,
        K8: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5, K6, K7, K8)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K6>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K7>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K8>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config = config.after(KeySetMarker::<K6>::default());
        config = config.after(KeySetMarker::<K7>::default());
        config = config.after(KeySetMarker::<K8>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
        K6: KeyType + KeyTypeUnpack,
        K7: KeyType + KeyTypeUnpack,
        K8: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5, K6, K7, K8)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        let sz6 = self.6.size(store);
        shift_bits += sz6;
        let sz7 = self.7.size(store);
        shift_bits += sz7;
        let sz8 = self.8.size(store);
        shift_bits += sz8;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz6;
                self.6.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz7;
                self.7.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz8;
                self.8.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<
        K0: KeyType,
        K1: KeyType,
        K2: KeyType,
        K3: KeyType,
        K4: KeyType,
        K5: KeyType,
        K6: KeyType,
        K7: KeyType,
        K8: KeyType,
        K9: KeyType,
    > KeyType for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        let sz6 = store.size_for_id(&TypeId::of::<K6>());
        offset += sz6;
        let sz7 = store.size_for_id(&TypeId::of::<K7>());
        offset += sz7;
        let sz8 = store.size_for_id(&TypeId::of::<K8>());
        offset += sz8;
        let sz9 = store.size_for_id(&TypeId::of::<K9>());
        offset += sz9;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        offset -= sz6;
        result.insert(TypeId::of::<K6>(), SizeOffset(sz6, offset));
        offset -= sz7;
        result.insert(TypeId::of::<K7>(), SizeOffset(sz7, offset));
        offset -= sz8;
        result.insert(TypeId::of::<K8>(), SizeOffset(sz8, offset));
        offset -= sz9;
        result.insert(TypeId::of::<K9>(), SizeOffset(sz9, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.6.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.7.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.8.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.9.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
        K6: KeyType + Default,
        K7: KeyType + Default,
        K8: KeyType + Default,
        K9: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K6>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K7>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K8>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K9>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config = config.after(KeySetMarker::<K6>::default());
        config = config.after(KeySetMarker::<K7>::default());
        config = config.after(KeySetMarker::<K8>::default());
        config = config.after(KeySetMarker::<K9>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
        K6: KeyType + KeyTypeUnpack,
        K7: KeyType + KeyTypeUnpack,
        K8: KeyType + KeyTypeUnpack,
        K9: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        let sz6 = self.6.size(store);
        shift_bits += sz6;
        let sz7 = self.7.size(store);
        shift_bits += sz7;
        let sz8 = self.8.size(store);
        shift_bits += sz8;
        let sz9 = self.9.size(store);
        shift_bits += sz9;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz6;
                self.6.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz7;
                self.7.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz8;
                self.8.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz9;
                self.9.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<
        K0: KeyType,
        K1: KeyType,
        K2: KeyType,
        K3: KeyType,
        K4: KeyType,
        K5: KeyType,
        K6: KeyType,
        K7: KeyType,
        K8: KeyType,
        K9: KeyType,
        K10: KeyType,
    > KeyType for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9, K10)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        let sz6 = store.size_for_id(&TypeId::of::<K6>());
        offset += sz6;
        let sz7 = store.size_for_id(&TypeId::of::<K7>());
        offset += sz7;
        let sz8 = store.size_for_id(&TypeId::of::<K8>());
        offset += sz8;
        let sz9 = store.size_for_id(&TypeId::of::<K9>());
        offset += sz9;
        let sz10 = store.size_for_id(&TypeId::of::<K10>());
        offset += sz10;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        offset -= sz6;
        result.insert(TypeId::of::<K6>(), SizeOffset(sz6, offset));
        offset -= sz7;
        result.insert(TypeId::of::<K7>(), SizeOffset(sz7, offset));
        offset -= sz8;
        result.insert(TypeId::of::<K8>(), SizeOffset(sz8, offset));
        offset -= sz9;
        result.insert(TypeId::of::<K9>(), SizeOffset(sz9, offset));
        offset -= sz10;
        result.insert(TypeId::of::<K10>(), SizeOffset(sz10, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.6.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.7.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.8.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.9.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.10.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
        K6: KeyType + Default,
        K7: KeyType + Default,
        K8: KeyType + Default,
        K9: KeyType + Default,
        K10: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9, K10)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K6>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K7>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K8>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K9>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K10>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config = config.after(KeySetMarker::<K6>::default());
        config = config.after(KeySetMarker::<K7>::default());
        config = config.after(KeySetMarker::<K8>::default());
        config = config.after(KeySetMarker::<K9>::default());
        config = config.after(KeySetMarker::<K10>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
        K6: KeyType + KeyTypeUnpack,
        K7: KeyType + KeyTypeUnpack,
        K8: KeyType + KeyTypeUnpack,
        K9: KeyType + KeyTypeUnpack,
        K10: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9, K10)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        let sz6 = self.6.size(store);
        shift_bits += sz6;
        let sz7 = self.7.size(store);
        shift_bits += sz7;
        let sz8 = self.8.size(store);
        shift_bits += sz8;
        let sz9 = self.9.size(store);
        shift_bits += sz9;
        let sz10 = self.10.size(store);
        shift_bits += sz10;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz6;
                self.6.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz7;
                self.7.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz8;
                self.8.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz9;
                self.9.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz10;
                self.10.unpack(value >> shift_bits, store)
            },
        )
    }
}
impl<
        K0: KeyType,
        K1: KeyType,
        K2: KeyType,
        K3: KeyType,
        K4: KeyType,
        K5: KeyType,
        K6: KeyType,
        K7: KeyType,
        K8: KeyType,
        K9: KeyType,
        K10: KeyType,
        K11: KeyType,
    > KeyType for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11)
{
    fn positions(&self, store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        let mut result = HashMap::default();
        let mut offset = 0;
        let sz0 = store.size_for_id(&TypeId::of::<K0>());
        offset += sz0;
        let sz1 = store.size_for_id(&TypeId::of::<K1>());
        offset += sz1;
        let sz2 = store.size_for_id(&TypeId::of::<K2>());
        offset += sz2;
        let sz3 = store.size_for_id(&TypeId::of::<K3>());
        offset += sz3;
        let sz4 = store.size_for_id(&TypeId::of::<K4>());
        offset += sz4;
        let sz5 = store.size_for_id(&TypeId::of::<K5>());
        offset += sz5;
        let sz6 = store.size_for_id(&TypeId::of::<K6>());
        offset += sz6;
        let sz7 = store.size_for_id(&TypeId::of::<K7>());
        offset += sz7;
        let sz8 = store.size_for_id(&TypeId::of::<K8>());
        offset += sz8;
        let sz9 = store.size_for_id(&TypeId::of::<K9>());
        offset += sz9;
        let sz10 = store.size_for_id(&TypeId::of::<K10>());
        offset += sz10;
        let sz11 = store.size_for_id(&TypeId::of::<K11>());
        offset += sz11;
        offset -= sz0;
        result.insert(TypeId::of::<K0>(), SizeOffset(sz0, offset));
        offset -= sz1;
        result.insert(TypeId::of::<K1>(), SizeOffset(sz1, offset));
        offset -= sz2;
        result.insert(TypeId::of::<K2>(), SizeOffset(sz2, offset));
        offset -= sz3;
        result.insert(TypeId::of::<K3>(), SizeOffset(sz3, offset));
        offset -= sz4;
        result.insert(TypeId::of::<K4>(), SizeOffset(sz4, offset));
        offset -= sz5;
        result.insert(TypeId::of::<K5>(), SizeOffset(sz5, offset));
        offset -= sz6;
        result.insert(TypeId::of::<K6>(), SizeOffset(sz6, offset));
        offset -= sz7;
        result.insert(TypeId::of::<K7>(), SizeOffset(sz7, offset));
        offset -= sz8;
        result.insert(TypeId::of::<K8>(), SizeOffset(sz8, offset));
        offset -= sz9;
        result.insert(TypeId::of::<K9>(), SizeOffset(sz9, offset));
        offset -= sz10;
        result.insert(TypeId::of::<K10>(), SizeOffset(sz10, offset));
        offset -= sz11;
        result.insert(TypeId::of::<K11>(), SizeOffset(sz11, offset));
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn pack(&self, store: &KeyMetaStore) -> (u32, u8) {
        let mut result = 0u32;
        let mut total_size = 0u8;
        let (value, size) = self.0.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.1.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.2.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.3.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.4.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.5.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.6.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.7.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.8.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.9.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.10.pack(store);
        result = (result << size) | value;
        total_size += size;
        let (value, size) = self.11.pack(store);
        result = (result << size) | value;
        total_size += size;
        (result, total_size)
    }
}
impl<
        K0: KeyType + Default,
        K1: KeyType + Default,
        K2: KeyType + Default,
        K3: KeyType + Default,
        K4: KeyType + Default,
        K5: KeyType + Default,
        K6: KeyType + Default,
        K7: KeyType + Default,
        K8: KeyType + Default,
        K9: KeyType + Default,
        K10: KeyType + Default,
        K11: KeyType + Default,
    > CompositeKey for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11)
{
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)> {
        let mut result = 0u32;
        let mut size = 0u8;
        let (v, s) = keys.get_raw_and_size::<K0>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K1>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K2>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K3>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K4>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K5>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K6>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K7>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K8>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K9>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K10>()?;
        result = result << s | v;
        size = size + s;
        let (v, s) = keys.get_raw_and_size::<K11>()?;
        result = result << s | v;
        size = size + s;
        Some((result, size))
    }
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>> {
        let mut config = KeySetMarker::<Self>::default().after(KeySetMarker::<()>::default());
        config = config.after(KeySetMarker::<K0>::default());
        config = config.after(KeySetMarker::<K1>::default());
        config = config.after(KeySetMarker::<K2>::default());
        config = config.after(KeySetMarker::<K3>::default());
        config = config.after(KeySetMarker::<K4>::default());
        config = config.after(KeySetMarker::<K5>::default());
        config = config.after(KeySetMarker::<K6>::default());
        config = config.after(KeySetMarker::<K7>::default());
        config = config.after(KeySetMarker::<K8>::default());
        config = config.after(KeySetMarker::<K9>::default());
        config = config.after(KeySetMarker::<K10>::default());
        config = config.after(KeySetMarker::<K11>::default());
        config
    }
}
impl<
        K0: KeyType + KeyTypeUnpack,
        K1: KeyType + KeyTypeUnpack,
        K2: KeyType + KeyTypeUnpack,
        K3: KeyType + KeyTypeUnpack,
        K4: KeyType + KeyTypeUnpack,
        K5: KeyType + KeyTypeUnpack,
        K6: KeyType + KeyTypeUnpack,
        K7: KeyType + KeyTypeUnpack,
        K8: KeyType + KeyTypeUnpack,
        K9: KeyType + KeyTypeUnpack,
        K10: KeyType + KeyTypeUnpack,
        K11: KeyType + KeyTypeUnpack,
    > KeyTypeUnpack for (K0, K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11)
{
    fn unpack(&self, value: u32, store: &KeyMetaStore) -> Self {
        let mut shift_bits = 0;
        let sz0 = self.0.size(store);
        shift_bits += sz0;
        let sz1 = self.1.size(store);
        shift_bits += sz1;
        let sz2 = self.2.size(store);
        shift_bits += sz2;
        let sz3 = self.3.size(store);
        shift_bits += sz3;
        let sz4 = self.4.size(store);
        shift_bits += sz4;
        let sz5 = self.5.size(store);
        shift_bits += sz5;
        let sz6 = self.6.size(store);
        shift_bits += sz6;
        let sz7 = self.7.size(store);
        shift_bits += sz7;
        let sz8 = self.8.size(store);
        shift_bits += sz8;
        let sz9 = self.9.size(store);
        shift_bits += sz9;
        let sz10 = self.10.size(store);
        shift_bits += sz10;
        let sz11 = self.11.size(store);
        shift_bits += sz11;
        (
            {
                shift_bits -= sz0;
                self.0.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz1;
                self.1.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz2;
                self.2.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz3;
                self.3.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz4;
                self.4.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz5;
                self.5.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz6;
                self.6.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz7;
                self.7.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz8;
                self.8.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz9;
                self.9.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz10;
                self.10.unpack(value >> shift_bits, store)
            },
            {
                shift_bits -= sz11;
                self.11.unpack(value >> shift_bits, store)
            },
        )
    }
}
