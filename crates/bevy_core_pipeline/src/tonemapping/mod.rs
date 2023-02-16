use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_reflect::{FromReflect, Reflect, ReflectFromReflect, TypeUuid};
use bevy_render::camera::Camera;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::render_asset::RenderAssets;
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::{CompressedImageFormats, Image, ImageType};
use bevy_render::view::{ViewTarget, ViewUniform};
use bevy_render::{render_resource::*, RenderApp, RenderSet};

mod node;

use bevy_utils::default;
pub use node::TonemappingNode;

const TONEMAPPING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17015368199668024512);

const TONEMAPPING_SHARED_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2499430578245347910);

#[derive(Resource)]
pub struct TonemappingLuts {
    blender_filmic: Handle<Image>,
    agx: Handle<Image>,
    tony_mc_mapface: Handle<Image>,
}

pub struct TonemappingPlugin;

impl Plugin for TonemappingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TONEMAPPING_SHADER_HANDLE,
            "tonemapping.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TONEMAPPING_SHARED_SHADER_HANDLE,
            "tonemapping_shared.wgsl",
            Shader::from_wgsl
        );

        let mut images = app.world.resource_mut::<Assets<Image>>();

        let tonemapping_luts = TonemappingLuts {
            blender_filmic: images.add(setup_tonemapping_lut_image(
                include_bytes!("luts/Blender_-11_12.ktx2"),
                ImageType::Extension("ktx2"),
            )),
            agx: images.add(setup_tonemapping_lut_image(
                include_bytes!("luts/AgX-default_contrast.ktx2"),
                ImageType::Extension("ktx2"),
            )),
            tony_mc_mapface: images.add(setup_tonemapping_lut_image(
                include_bytes!("luts/TonyMcMapface.ktx2"),
                ImageType::Extension("ktx2"),
            )),
        };

        app.register_type::<Tonemapping>();

        app.add_plugin(ExtractComponentPlugin::<Tonemapping>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(tonemapping_luts)
                .init_resource::<TonemappingPipeline>()
                .init_resource::<SpecializedRenderPipelines<TonemappingPipeline>>()
                .add_system(queue_view_tonemapping_pipelines.in_set(RenderSet::Queue));
        }
    }
}

#[derive(Resource)]
pub struct TonemappingPipeline {
    texture_bind_group: BindGroupLayout,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect, FromReflect)]
#[reflect(FromReflect)]
pub enum TonemappingMethod {
    None,
    /// Suffers from lots hue of shifting, brights don't desaturate naturally.
    Reinhard,
    /// Old bevy default. Suffers from hue shifting, brights don't desaturate much at all.
    ReinhardLuminance,
    /// Bad
    Aces,
    /// Very Good
    AgX,
    /// Also good
    SomewhatBoringDisplayTransform,
    /// Very Good
    TonyMcMapface,
    /// Also good
    #[default]
    BlenderFilmic,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TonemappingPipelineKey {
    deband_dither: bool,
    method: TonemappingMethod,
}

impl SpecializedRenderPipeline for TonemappingPipeline {
    type Key = TonemappingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.deband_dither {
            shader_defs.push("DEBAND_DITHER".into());
        }
        match key.method {
            TonemappingMethod::None => shader_defs.push("TONEMAP_METHOD_NONE".into()),
            TonemappingMethod::Reinhard => shader_defs.push("TONEMAP_METHOD_REINHARD".into()),
            TonemappingMethod::ReinhardLuminance => {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            }
            TonemappingMethod::Aces => shader_defs.push("TONEMAP_METHOD_ACES".into()),
            TonemappingMethod::AgX => shader_defs.push("TONEMAP_METHOD_AGX".into()),
            TonemappingMethod::SomewhatBoringDisplayTransform => {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into())
            }
            TonemappingMethod::TonyMcMapface => {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into())
            }
            TonemappingMethod::BlenderFilmic => {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            }
        }
        RenderPipelineDescriptor {
            label: Some("tonemapping pipeline".into()),
            layout: Some(vec![self.texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TONEMAPPING_SHADER_HANDLE.typed(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

impl FromWorld for TonemappingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let mut entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
        ];
        entries.extend(get_lut_bind_group_layout_entries([3, 4]));

        let tonemap_texture_bind_group = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("tonemapping_hdr_texture_bind_group_layout"),
                entries: &entries,
            });

        TonemappingPipeline {
            texture_bind_group: tonemap_texture_bind_group,
        }
    }
}

#[derive(Component)]
pub struct ViewTonemappingPipeline(CachedRenderPipelineId);

pub fn queue_view_tonemapping_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TonemappingPipeline>>,
    upscaling_pipeline: Res<TonemappingPipeline>,
    view_targets: Query<(Entity, &Tonemapping)>,
) {
    for (entity, tonemapping) in view_targets.iter() {
        if let Tonemapping::Enabled {
            deband_dither,
            method,
        } = tonemapping
        {
            let key = TonemappingPipelineKey {
                deband_dither: *deband_dither,
                method: *method,
            };
            let pipeline = pipelines.specialize(&pipeline_cache, &upscaling_pipeline, key);

            commands
                .entity(entity)
                .insert(ViewTonemappingPipeline(pipeline));
        }
    }
}

#[derive(Component, Debug, Hash, Clone, Reflect, Default, ExtractComponent, PartialEq, Eq)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component)]
pub enum Tonemapping {
    #[default]
    Disabled,
    Enabled {
        deband_dither: bool,
        method: TonemappingMethod,
    },
}

impl Tonemapping {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Tonemapping::Enabled { .. })
    }
}

pub fn get_lut_bindings<'a>(
    images: &'a RenderAssets<Image>,
    tonemapping_luts: &'a TonemappingLuts,
    tonemapping: &Tonemapping,
    bindings: [u32; 2],
) -> [BindGroupEntry<'a>; 2] {
    let image = match tonemapping {
        Tonemapping::Disabled => &tonemapping_luts.agx,
        Tonemapping::Enabled {
            deband_dither: _,
            method,
        } => match method {
            //AgX lut texture used when tonemapping doesn't need a texture since it's very small (32x32x32)
            TonemappingMethod::None
            | TonemappingMethod::Reinhard
            | TonemappingMethod::ReinhardLuminance
            | TonemappingMethod::Aces
            | TonemappingMethod::AgX
            | TonemappingMethod::SomewhatBoringDisplayTransform => &tonemapping_luts.agx,
            TonemappingMethod::TonyMcMapface => &tonemapping_luts.tony_mc_mapface,
            TonemappingMethod::BlenderFilmic => &tonemapping_luts.blender_filmic,
        },
    };
    let lut_image = images.get(image).unwrap();
    [
        BindGroupEntry {
            binding: bindings[0],
            resource: BindingResource::TextureView(&lut_image.texture_view),
        },
        BindGroupEntry {
            binding: bindings[1],
            resource: BindingResource::Sampler(&lut_image.sampler),
        },
    ]
}

pub fn get_lut_bind_group_layout_entries(bindings: [u32; 2]) -> [BindGroupLayoutEntry; 2] {
    [
        BindGroupLayoutEntry {
            binding: bindings[0],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D3,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: bindings[1],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
    ]
}

fn setup_tonemapping_lut_image(bytes: &[u8], image_type: ImageType) -> Image {
    let mut image =
        Image::from_buffer(bytes, image_type, CompressedImageFormats::NONE, false).unwrap();

    image.sampler_descriptor = bevy_render::texture::ImageSampler::Descriptor(SamplerDescriptor {
        label: Some("Tonemapping LUT sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..default()
    });

    image
}
