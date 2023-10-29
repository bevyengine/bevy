use bevy_ecs::{prelude::*, schedule::NodeConfigs};
pub use bevy_render_macros::PipelineKey;
use bevy_utils::{HashMap, intern::Interned};
use std::any::{Any, TypeId};
use super::*;

#[allow(unused_imports)]
use bevy_utils::all_tuples;

pub trait CompositeKey: AnyKeyType + KeyTypeConcrete {
    fn from_keys(keys: &PipelineKeys) -> Option<(u32, u8)>;
    fn set_config() -> NodeConfigs<Interned<dyn bevy_ecs::schedule::SystemSet>>;
}

#[allow(unused_macros)]
macro_rules! impl_composite_key_tuples {
    ($(($K:ident, $sz:ident, $value:ident)),*) => {
        impl<$($K: AnyKeyType),*> AnyKeyType for ($($K,)*) {
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl<$($K: AnyKeyType + KeyTypeConcrete),*> KeyTypeConcrete for ($($K,)*) {
            fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
                let mut result = HashMap::default();
                let mut offset = 0;

                $(
                    let $sz = $K::size(store);
                    offset += $sz;
                )*
        
                $(
                    offset -= $sz;
                    result.insert(TypeId::of::<$K>(), SizeOffset($sz, offset));
                )*
        
                result
            }

            fn pack(value: &Self, store: &KeyMetaStore) -> (u32, u8) {
                let mut result = 0u32;
                let mut total_size = 0u8;

                let ($($value,)*) = value;

                $(
                    let (value, size) = $K::pack($value, store);
                    result = (result << size) | value;
                    total_size += size;
                )*
    
                (result, total_size)
            }
            
            fn unpack(value: u32, store: &KeyMetaStore) -> Self {
                let mut shift_bits = 0;
                $(
                    let $sz = $K::size(store);
                    shift_bits += $sz;
                )*

                (
                    $({
                        shift_bits -= $sz;
                        $K::unpack((value >> shift_bits) & ((1 << $sz) - 1), store)
                    },)*
                )
            }
        }

        impl<$($K: AnyKeyType + KeyTypeConcrete),*> CompositeKey for ($($K,)*) {
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

        impl<$($K: FixedSizeKey),*> FixedSizeKey for ($($K,)*) {
            fn fixed_size() -> u8 {
                $($K::fixed_size() + )* 0
            }
        }
    }
}

all_tuples!(impl_composite_key_tuples, 1, 16, K, sz, selfdot);

