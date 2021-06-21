use bevy_app::{App, CoreStage, EventReader, Plugin};
use bevy_asset::{AddAsset, AssetEvent, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_math::Vec4;
use bevy_reflect::TypeUuid;
use bevy_render2::{color::Color, render_resource::{Buffer, BufferId, BufferInitDescriptor, BufferUsage}, renderer::{RenderDevice, RenderQueue}};
use bevy_utils::HashSet;
use crevice::std140::{AsStd140, Std140};

// TODO: this shouldn't live in the StandardMaterial type
#[derive(Debug, Clone)]
pub struct StandardMaterialGpuData {
    pub buffer: Buffer,
}

/// A material with "standard" properties used in PBR lighting
/// Standard property values with pictures here https://google.github.io/filament/Material%20Properties.pdf
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7494888b-c082-457b-aacf-517228cc0c22"]
pub struct StandardMaterial {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between.
    pub color: Color,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Color,
    pub gpu_data: Option<StandardMaterialGpuData>,
}

impl StandardMaterial {
    pub fn gpu_data(&self) -> Option<&StandardMaterialGpuData> {
        self.gpu_data.as_ref()
    }
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            color: Color::rgb(1.0, 1.0, 1.0),
            // This is the minimum the roughness is clamped to in shader code
            // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/
            // It's the minimum floating point value that won't be rounded down to 0 in the
            // calculations used. Although technically for 32-bit floats, 0.045 could be
            // used.
            roughness: 0.089,
            // Few materials are purely dielectric or metallic
            // This is just a default for mostly-dielectric
            metallic: 0.01,
            // Minimum real-world reflectance is 2%, most materials between 2-5%
            // Expressed in a linear scale and equivalent to 4% reflectance see https://google.github.io/filament/Material%20Properties.pdf
            reflectance: 0.5,
            emissive: Color::BLACK,
            gpu_data: None,
        }
    }
}

impl From<Color> for StandardMaterial {
    fn from(color: Color) -> Self {
        StandardMaterial {
            color,
            ..Default::default()
        }
    }
}

#[derive(Clone, AsStd140)]
pub struct StandardMaterialUniformData {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between.
    pub color: Vec4,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Vec4,
}

pub struct StandardMaterialPlugin;

impl Plugin for StandardMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<StandardMaterial>().add_system_to_stage(
            CoreStage::PostUpdate,
            standard_material_resource_system.system(),
        );
    }
}

pub fn standard_material_resource_system(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_events: EventReader<AssetEvent<StandardMaterial>>,
) {
    let mut changed_materials = HashSet::default();
    for event in material_events.iter() {
        match event {
            AssetEvent::Created { ref handle } => {
                changed_materials.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_materials.insert(handle.clone_weak());
                // TODO: uncomment this to support mutated materials
                // remove_current_material_resources(render_resource_context, handle, &mut materials);
            }
            AssetEvent::Removed { ref handle } => {
                // if material was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_materials.remove(handle);
            }
        }
    }

    // update changed material data
    for changed_material_handle in changed_materials.iter() {
        if let Some(material) = materials.get_mut(changed_material_handle) {
            // TODO: this avoids creating new materials each frame because storing gpu data in the material flags it as
            // modified. this prevents hot reloading and therefore can't be used in an actual impl.
            if material.gpu_data.is_some() {
                continue;
            }

            let value = StandardMaterialUniformData {
                color: material.color.into(),
                roughness: material.roughness,
                metallic: material.metallic,
                reflectance: material.reflectance,
                emissive: material.emissive.into(),
            };
            let value_std140 = value.as_std140();

            let size = StandardMaterialUniformData::std140_size_static();

            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: None,
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                contents: value_std140.as_bytes(),
            });

            material.gpu_data = Some(StandardMaterialGpuData { buffer });
        }
    }
}
