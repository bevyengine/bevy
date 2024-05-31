use bevy_asset::Asset;
use bevy_pbr::{ExtendedMaterial, Material, MaterialExtension, StandardMaterial};

/// A material that can be created from a [`StandardMaterial`] and/or `gltf_extras`.
/// This allows this material to replace `StandardMaterial` in gltf loaders.
///
/// By default [`StandardMaterial`] and [`ExtendedMaterial<StandardMaterial, impl FromStandardMaterial>`]
/// implement this trait.
pub trait FromStandardMaterial: Asset + Sized {
    /// Create a material from a [`StandardMaterial`] and `gltf_extra` as json.
    ///
    /// This function cannot fail, try return a default value in case of failure.
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
    M: FromStandardMaterial + MaterialExtension,
{
    fn from_standard_material(material: StandardMaterial, gltf_extras: Option<&str>) -> Self {
        ExtendedMaterial {
            extension: M::from_standard_material(material.clone(), gltf_extras),
            base: T::from_standard_material(material, gltf_extras),
        }
    }
}
