//! glTF extensions defined by the Khronos Group and other vendors

mod khr_materials_anisotropy;
mod khr_materials_clearcoat;
mod khr_materials_specular;

pub(crate) use self::{
    khr_materials_anisotropy::AnisotropyExtension, khr_materials_clearcoat::ClearcoatExtension,
    khr_materials_specular::SpecularExtension,
};
