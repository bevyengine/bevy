use bevy_asset::Asset;
use bevy_pbr::{ExtendedMaterial, Material, MaterialExtension, StandardMaterial};

/// Infallible conversion from `json` encoded `gltf_extras`.
pub trait FromGltfExtras {
    /// Infallible conversion from an optional json string.
    ///
    /// Consider returning a default value on failure.
    fn from_gltf_extras(gltf_extra: Option<&str>) -> Self;
}

/// A material that can be created from a [`StandardMaterial`] and/or `gltf_extras`.
/// This allows this material to swap out `StandardMaterial` in gltf loaders.
///
/// By default [`StandardMaterial`] and [`ExtendedMaterial<StandardMaterial, impl FromGltfExtras>`]
/// implement this trait.
pub trait FromStandardMaterial: Asset + Sized {
    /// Create a material from a [`StandardMaterial`] and `gltf_extra` as json.
    fn from_standard_material(material: StandardMaterial, gltf_extras: Option<&str>) -> Self;
}

impl FromStandardMaterial for StandardMaterial {
    fn from_standard_material(material: StandardMaterial, _: Option<&str>) -> Self {
        material
    }
}

impl<T, M> FromStandardMaterial for ExtendedMaterial<T, M>
where
    T: FromStandardMaterial + Material,
    M: FromGltfExtras + MaterialExtension,
{
    fn from_standard_material(material: StandardMaterial, gltf_extra: Option<&str>) -> Self {
        ExtendedMaterial {
            base: T::from_standard_material(material, gltf_extra),
            extension: M::from_gltf_extras(gltf_extra),
        }
    }
}
