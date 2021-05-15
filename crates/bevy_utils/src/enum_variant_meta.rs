pub use bevy_derive::EnumVariantMeta;

pub trait EnumVariantMeta {
    fn enum_variant_index(&self) -> usize;
    fn enum_variant_name(&self) -> &'static str;
}
