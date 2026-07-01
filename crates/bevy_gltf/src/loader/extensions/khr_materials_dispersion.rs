use gltf::Material;
use serde_json::Value;

/// Parsed data from the `KHR_materials_dispersion` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_dispersion/README.md>
#[derive(Default)]
pub(crate) struct DispersionExtension {
    /// The strength of the chromatic dispersion effect. Defaults to `0.0` (no dispersion).
    pub(crate) dispersion: f32,
}

impl DispersionExtension {
    pub(crate) fn parse(material: &Material) -> Option<Self> {
        let extension = material
            .extensions()?
            .get("KHR_materials_dispersion")?
            .as_object()?;

        let dispersion = extension
            .get("dispersion")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;

        Some(DispersionExtension { dispersion })
    }
}
