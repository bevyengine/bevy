use super::*;
use bevy_ecs::schedule::NodeConfigs;
pub use bevy_render_macros::PipelineKey;
use bevy_utils::{intern::Interned, HashMap};
use std::any::{Any, TypeId};

#[allow(unused_imports)]
use bevy_utils::all_tuples;

pub trait CompositeKey: AnyKeyType + KeyTypeConcrete {
    fn from_keys(keys: &PipelineKeys) -> Option<PackedPipelineKey<Self>>
    where
        Self: Sized;
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

            fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self> {
                let mut result = 0 as KeyPrimitive;
                let mut total_size = 0u8;

                let ($($value,)*) = value;

                $(
                    let PackedPipelineKey{ packed, size, .. } = $K::pack($value, store);
                    assert_eq!(size, $K::size(store), "{} mismatch", type_name::<$K>());

                    result = (result << size) | packed;
                    total_size += size;
                    debug!("{} added {}={:#b} @ {} -> {:#b}", type_name::<Self>(), type_name::<$K>(), packed, size, result);
                )*

                PackedPipelineKey::new(result, total_size)
            }

            fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self {
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
            fn from_keys(keys: &PipelineKeys) -> Option<PackedPipelineKey<Self>> {
                let mut result = 0 as KeyPrimitive;
                let mut total_size = 0u8;

                $(
                    let PackedPipelineKey{ packed, size, .. } = keys.get_packed_key::<$K>()?;
                    result = result << size | packed;
                    total_size = total_size + size;
                )*

                Some(PackedPipelineKey::new(result, total_size))
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

        impl<$($K: KeyTypeConcrete),*> KeyRepack for ($($K,)*) {
            type PackedParts = ($(PackedPipelineKey<$K>,)*);

            fn repack( ( $($value,)* ): Self::PackedParts) -> PackedPipelineKey<Self> {
                let mut result = 0 as KeyPrimitive;
                let mut total_size = 0u8;

                $(
                    let PackedPipelineKey{ packed, size, .. } = $value;
                    result = (result << size) | packed;
                    total_size += size;
                )*

                PackedPipelineKey::new(result, total_size)
            }
        }
    }
}

all_tuples!(impl_composite_key_tuples, 1, 16, K, sz, selfdot);
