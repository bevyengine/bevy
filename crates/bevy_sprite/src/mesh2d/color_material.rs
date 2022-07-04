use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetServer, Assets, Handle, HandleUntyped};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_math::Vec4;
use bevy_reflect::TypeUuid;
use bevy_render::{
    color::Color,
    prelude::Shader,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
    render_resource::*,
    renderer::RenderDevice,
    texture::Image,
};

use crate::{Material2d, Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle};

pub const COLOR_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3253086872234592509);

#[derive(Default)]
pub struct ColorMaterialPlugin;

impl Plugin for ColorMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            COLOR_MATERIAL_SHADER_HANDLE,
            "color_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(Material2dPlugin::<ColorMaterial>::default());

        app.world
            .resource_mut::<Assets<ColorMaterial>>()
            .set_untracked(
                Handle::<ColorMaterial>::default(),
                ColorMaterial {
                    color: Color::rgb(1.0, 0.0, 1.0),
                    ..Default::default()
                },
            );
    }
}

/// A [2d material](Material2d) that renders [2d meshes](crate::Mesh2dHandle) with a texture tinted by a uniform color
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "e228a544-e3ca-4e1e-bb9d-4d8bc1ad8c19"]
pub struct ColorMaterial {
    pub color: Color,
    pub texture: Option<Handle<Image>>,
}

impl Default for ColorMaterial {
    fn default() -> Self {
        ColorMaterial {
            color: Color::WHITE,
            texture: None,
        }
    }
}

impl From<Color> for ColorMaterial {
    fn from(color: Color) -> Self {
        ColorMaterial {
            color,
            ..Default::default()
        }
    }
}

impl From<Handle<Image>> for ColorMaterial {
    fn from(texture: Handle<Image>) -> Self {
        ColorMaterial {
            texture: Some(texture),
            ..Default::default()
        }
    }
}

// NOTE: These must match the bit flags in bevy_sprite/src/mesh2d/color_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ColorMaterialFlags: u32 {
        const TEXTURE           = (1 << 0);
        const NONE              = 0;
        const UNINITIALIZED     = 0xFFFF;
    }
}

/// The GPU representation of the uniform data of a [`ColorMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct ColorMaterialUniformData {
    pub color: Vec4,
    pub flags: u32,
}

/// The GPU representation of a [`ColorMaterial`].
#[derive(Debug, Clone)]
pub struct GpuColorMaterial {
    /// A buffer containing the [`ColorMaterialUniformData`] of the material.
    pub buffer: Buffer,
    /// The bind group specifying how the [`ColorMaterialUniformData`] and
    /// the texture of the material are bound.
    pub bind_group: BindGroup,
    pub flags: ColorMaterialFlags,
    pub texture: Option<Handle<Image>>,
}

impl RenderAsset for ColorMaterial {
    type ExtractedAsset = ColorMaterial;
    type PreparedAsset = GpuColorMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<Material2dPipeline<ColorMaterial>>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, color_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (texture_view, sampler) = if let Some(result) = color_pipeline
            .mesh2d_pipeline
            .get_image_texture(gpu_images, &material.texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let mut flags = ColorMaterialFlags::NONE;
        if material.texture.is_some() {
            flags |= ColorMaterialFlags::TEXTURE;
        }

        let value = ColorMaterialUniformData {
            color: material.color.as_linear_rgba_f32().into(),
            flags: flags.bits(),
        };

        let byte_buffer = [0u8; ColorMaterialUniformData::SHADER_SIZE.get() as usize];
        let mut buffer = encase::UniformBuffer::new(byte_buffer);
        buffer.write(&value).unwrap();

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("color_material_uniform_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: buffer.as_ref(),
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
            label: Some("color_material_bind_group"),
            layout: &color_pipeline.material2d_layout,
        });

        Ok(GpuColorMaterial {
            buffer,
            bind_group,
            flags,
            texture: material.texture,
        })
    }
}

impl Material2d for ColorMaterial {
    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(COLOR_MATERIAL_SHADER_HANDLE.typed())
    }

    #[inline]
    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(
        render_device: &RenderDevice,
    ) -> bevy_render::render_resource::BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ColorMaterialUniformData::min_size()),
                    },
                    count: None,
                },
                // Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Texture Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("color_material_layout"),
        })
    }
}

/// A component bundle for entities with a [`Mesh2dHandle`](crate::Mesh2dHandle) and a [`ColorMaterial`].
pub type ColorMesh2dBundle = MaterialMesh2dBundle<ColorMaterial>;
