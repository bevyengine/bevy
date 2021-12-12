use crate::{AlphaMode, PbrPipeline, StandardMaterialFlags};
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Handle};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_math::Vec4;
use bevy_reflect::TypeUuid;
use bevy_render::{
    color::Color,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_resource::{BindGroup, Buffer, BufferInitDescriptor, BufferUsages},
    renderer::RenderDevice,
    texture::Image,
};
use crevice::std140::{AsStd140, Std140};
use wgpu::{BindGroupDescriptor, BindGroupEntry, BindingResource};

/// A material with "standard" properties used in PBR lighting
/// Standard property values with pictures here
/// <https://google.github.io/filament/Material%20Properties.pdf>.
///
/// May be created directly from a [`Color`] or an [`Image`].
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7494888b-c082-457b-aacf-517228cc0c22"]
pub struct StandardMaterial {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between. If used together with a base_color_texture, this is factored into the final
    /// base color as `base_color * base_color_texture_value`
    pub base_color: Color,
    pub base_color_texture: Option<Handle<Image>>,
    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Color,
    pub emissive_texture: Option<Handle<Image>>,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `roughness * roughness_texture_value`
    pub perceptual_roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `metallic * metallic_texture_value`
    pub metallic: f32,
    pub metallic_roughness_texture: Option<Handle<Image>>,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    pub normal_map_texture: Option<Handle<Image>>,
    pub occlusion_texture: Option<Handle<Image>>,
    pub double_sided: bool,
    pub unlit: bool,
    pub alpha_mode: AlphaMode,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            base_color: Color::rgb(1.0, 1.0, 1.0),
            base_color_texture: None,
            emissive: Color::BLACK,
            emissive_texture: None,
            // This is the minimum the roughness is clamped to in shader code
            // See <https://google.github.io/filament/Filament.html#materialsystem/parameterization/>
            // It's the minimum floating point value that won't be rounded down to 0 in the
            // calculations used. Although technically for 32-bit floats, 0.045 could be
            // used.
            perceptual_roughness: 0.089,
            // Few materials are purely dielectric or metallic
            // This is just a default for mostly-dielectric
            metallic: 0.01,
            metallic_roughness_texture: None,
            // Minimum real-world reflectance is 2%, most materials between 2-5%
            // Expressed in a linear scale and equivalent to 4% reflectance see
            // <https://google.github.io/filament/Material%20Properties.pdf>
            reflectance: 0.5,
            occlusion_texture: None,
            normal_map_texture: None,
            double_sided: false,
            unlit: false,
            alpha_mode: AlphaMode::Opaque,
        }
    }
}

impl From<Color> for StandardMaterial {
    fn from(color: Color) -> Self {
        StandardMaterial {
            base_color: color,
            ..Default::default()
        }
    }
}

impl From<Handle<Image>> for StandardMaterial {
    fn from(texture: Handle<Image>) -> Self {
        StandardMaterial {
            base_color_texture: Some(texture),
            ..Default::default()
        }
    }
}

/// The GPU representation of the uniform data of a [`StandardMaterial`].
#[derive(Clone, Default, AsStd140)]
pub struct StandardMaterialUniformData {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between.
    pub base_color: Vec4,
    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Vec4,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    pub flags: u32,
    /// When the alpha mode mask flag is set, any base color alpha above this cutoff means fully opaque,
    /// and any below means fully transparent.
    pub alpha_cutoff: f32,
}

/// This plugin adds the [`StandardMaterial`] asset to the app.
pub struct StandardMaterialPlugin;

impl Plugin for StandardMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenderAssetPlugin::<StandardMaterial>::default())
            .add_asset::<StandardMaterial>();
    }
}

/// The GPU representation of a [`StandardMaterial`].
#[derive(Debug, Clone)]
pub struct GpuStandardMaterial {
    /// A buffer containing the [`StandardMaterialUniformData`] of the material.
    pub buffer: Buffer,
    /// The bind group specifying how the [`StandardMaterialUniformData`] and
    /// all the textures of the material are bound.
    pub bind_group: BindGroup,
    pub has_normal_map: bool,
    pub flags: StandardMaterialFlags,
    pub base_color_texture: Option<Handle<Image>>,
    pub alpha_mode: AlphaMode,
}

impl RenderAsset for StandardMaterial {
    type ExtractedAsset = StandardMaterial;
    type PreparedAsset = GpuStandardMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<PbrPipeline>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, pbr_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (base_color_texture_view, base_color_sampler) = if let Some(result) = pbr_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.base_color_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (emissive_texture_view, emissive_sampler) = if let Some(result) = pbr_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.emissive_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (metallic_roughness_texture_view, metallic_roughness_sampler) = if let Some(result) =
            pbr_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, &material.metallic_roughness_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
        let (normal_map_texture_view, normal_map_sampler) = if let Some(result) = pbr_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.normal_map_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
        let (occlusion_texture_view, occlusion_sampler) = if let Some(result) = pbr_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.occlusion_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
        let mut flags = StandardMaterialFlags::NONE;
        if material.base_color_texture.is_some() {
            flags |= StandardMaterialFlags::BASE_COLOR_TEXTURE;
        }
        if material.emissive_texture.is_some() {
            flags |= StandardMaterialFlags::EMISSIVE_TEXTURE;
        }
        if material.metallic_roughness_texture.is_some() {
            flags |= StandardMaterialFlags::METALLIC_ROUGHNESS_TEXTURE;
        }
        if material.occlusion_texture.is_some() {
            flags |= StandardMaterialFlags::OCCLUSION_TEXTURE;
        }
        if material.double_sided {
            flags |= StandardMaterialFlags::DOUBLE_SIDED;
        }
        if material.unlit {
            flags |= StandardMaterialFlags::UNLIT;
        }
        let has_normal_map = material.normal_map_texture.is_some();
        // NOTE: 0.5 is from the glTF default - do we want this?
        let mut alpha_cutoff = 0.5;
        match material.alpha_mode {
            AlphaMode::Opaque => flags |= StandardMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(c) => {
                alpha_cutoff = c;
                flags |= StandardMaterialFlags::ALPHA_MODE_MASK
            }
            AlphaMode::Blend => flags |= StandardMaterialFlags::ALPHA_MODE_BLEND,
        };

        let value = StandardMaterialUniformData {
            base_color: material.base_color.as_linear_rgba_f32().into(),
            emissive: material.emissive.into(),
            roughness: material.perceptual_roughness,
            metallic: material.metallic,
            reflectance: material.reflectance,
            flags: flags.bits(),
            alpha_cutoff,
        };
        let value_std140 = value.as_std140();

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("pbr_standard_material_uniform_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: value_std140.as_bytes(),
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(base_color_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(base_color_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(emissive_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(emissive_sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(metallic_roughness_texture_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Sampler(metallic_roughness_sampler),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(occlusion_texture_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::Sampler(occlusion_sampler),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(normal_map_texture_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::Sampler(normal_map_sampler),
                },
            ],
            label: Some("pbr_standard_material_bind_group"),
            layout: &pbr_pipeline.material_layout,
        });

        Ok(GpuStandardMaterial {
            buffer,
            bind_group,
            flags,
            has_normal_map,
            base_color_texture: material.base_color_texture,
            alpha_mode: material.alpha_mode,
        })
    }
}
