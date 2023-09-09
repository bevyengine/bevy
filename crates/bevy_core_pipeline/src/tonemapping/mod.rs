use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::camera::Camera;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy_render::render_asset::RenderAssets;
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::{CompressedImageFormats, Image, ImageSampler, ImageType};
use bevy_render::view::{ViewTarget, ViewUniform};
use bevy_render::{render_resource::*, Render, RenderApp, RenderSet};

mod node;

use bevy_utils::default;
pub use node::TonemappingNode;

const TONEMAPPING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(17015368199668024512);

const TONEMAPPING_SHARED_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2499430578245347910);

/// 3D LUT (look up table) textures used for tonemapping
#[derive(Resource, Clone, ExtractResource)]
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

        if !app.world.is_resource_added::<TonemappingLuts>() {
            let mut images = app.world.resource_mut::<Assets<Image>>();

            #[cfg(feature = "tonemapping_luts")]
            let tonemapping_luts = {
                TonemappingLuts {
                    blender_filmic: images.add(setup_tonemapping_lut_image(
                        include_bytes!("luts/Blender_-11_12.ktx2"),
                        ImageType::Extension("ktx2"),
                    )),
                    agx: images.add(setup_tonemapping_lut_image(
                        include_bytes!("luts/AgX-default_contrast.ktx2"),
                        ImageType::Extension("ktx2"),
                    )),
                    tony_mc_mapface: images.add(setup_tonemapping_lut_image(
                        include_bytes!("luts/tony_mc_mapface.ktx2"),
                        ImageType::Extension("ktx2"),
                    )),
                }
            };

            #[cfg(not(feature = "tonemapping_luts"))]
            let tonemapping_luts = {
                let placeholder = images.add(lut_placeholder());
                TonemappingLuts {
                    blender_filmic: placeholder.clone(),
                    agx: placeholder.clone(),
                    tony_mc_mapface: placeholder,
                }
            };

            app.insert_resource(tonemapping_luts);
        }

        app.add_plugins(ExtractResourcePlugin::<TonemappingLuts>::default());

        app.register_type::<Tonemapping>();
        app.register_type::<DebandDither>();

        app.add_plugins((
            ExtractComponentPlugin::<Tonemapping>::default(),
            ExtractComponentPlugin::<DebandDither>::default(),
        ));

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SpecializedRenderPipelines<TonemappingPipeline>>()
                .add_systems(
                    Render,
                    prepare_view_tonemapping_pipelines.in_set(RenderSet::Prepare),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TonemappingPipeline>();
        }
    }
}

#[derive(Resource)]
pub struct TonemappingPipeline {
    texture_bind_group: BindGroupLayout,
}

/// Optionally enables a tonemapping shader that attempts to map linear input stimulus into a perceptually uniform image for a given [`Camera`] entity.
#[derive(
    Component, Debug, Hash, Clone, Copy, Reflect, Default, ExtractComponent, PartialEq, Eq,
)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component)]
pub enum Tonemapping {
    /// Bypass tonemapping.
    None,
    /// Suffers from lots hue shifting, brights don't desaturate naturally.
    /// Bright primaries and secondaries don't desaturate at all.
    Reinhard,
    /// Suffers from hue shifting. Brights don't desaturate much at all across the spectrum.
    ReinhardLuminance,
    /// Same base implementation that Godot 4.0 uses for Tonemap ACES.
    /// <https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl>
    /// Not neutral, has a very specific aesthetic, intentional and dramatic hue shifting.
    /// Bright greens and reds turn orange. Bright blues turn magenta.
    /// Significantly increased contrast. Brights desaturate across the spectrum.
    AcesFitted,
    /// By Troy Sobotka
    /// <https://github.com/sobotka/AgX>
    /// Very neutral. Image is somewhat desaturated when compared to other tonemappers.
    /// Little to no hue shifting. Subtle [Abney shifting](https://en.wikipedia.org/wiki/Abney_effect).
    /// NOTE: Requires the `tonemapping_luts` cargo feature.
    AgX,
    /// By Tomasz Stachowiak
    /// Has little hue shifting in the darks and mids, but lots in the brights. Brights desaturate across the spectrum.
    /// Is sort of between Reinhard and ReinhardLuminance. Conceptually similar to reinhard-jodie.
    /// Designed as a compromise if you want e.g. decent skin tones in low light, but can't afford to re-do your
    /// VFX to look good without hue shifting.
    SomewhatBoringDisplayTransform,
    /// Current Bevy default.
    /// By Tomasz Stachowiak
    /// <https://github.com/h3r2tic/tony-mc-mapface>
    /// Very neutral. Subtle but intentional hue shifting. Brights desaturate across the spectrum.
    /// Comment from author:
    /// Tony is a display transform intended for real-time applications such as games.
    /// It is intentionally boring, does not increase contrast or saturation, and stays close to the
    /// input stimulus where compression isn't necessary.
    /// Brightness-equivalent luminance of the input stimulus is compressed. The non-linearity resembles Reinhard.
    /// Color hues are preserved during compression, except for a deliberate [Bezold–Brücke shift](https://en.wikipedia.org/wiki/Bezold%E2%80%93Br%C3%BCcke_shift).
    /// To avoid posterization, selective desaturation is employed, with care to avoid the [Abney effect](https://en.wikipedia.org/wiki/Abney_effect).
    /// NOTE: Requires the `tonemapping_luts` cargo feature.
    #[default]
    TonyMcMapface,
    /// Default Filmic Display Transform from blender.
    /// Somewhat neutral. Suffers from hue shifting. Brights desaturate across the spectrum.
    /// NOTE: Requires the `tonemapping_luts` cargo feature.
    BlenderFilmic,
}

impl Tonemapping {
    pub fn is_enabled(&self) -> bool {
        *self != Tonemapping::None
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TonemappingPipelineKey {
    deband_dither: DebandDither,
    tonemapping: Tonemapping,
}

impl SpecializedRenderPipeline for TonemappingPipeline {
    type Key = TonemappingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if let DebandDither::Enabled = key.deband_dither {
            shader_defs.push("DEBAND_DITHER".into());
        }
        match key.tonemapping {
            Tonemapping::None => shader_defs.push("TONEMAP_METHOD_NONE".into()),
            Tonemapping::Reinhard => shader_defs.push("TONEMAP_METHOD_REINHARD".into()),
            Tonemapping::ReinhardLuminance => {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            }
            Tonemapping::AcesFitted => shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into()),
            Tonemapping::AgX => shader_defs.push("TONEMAP_METHOD_AGX".into()),
            Tonemapping::SomewhatBoringDisplayTransform => {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            }
            Tonemapping::TonyMcMapface => shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into()),
            Tonemapping::BlenderFilmic => {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            }
        }
        RenderPipelineDescriptor {
            label: Some("tonemapping pipeline".into()),
            layout: vec![self.texture_bind_group.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TONEMAPPING_SHADER_HANDLE,
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
            push_constant_ranges: Vec::new(),
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

pub fn prepare_view_tonemapping_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TonemappingPipeline>>,
    upscaling_pipeline: Res<TonemappingPipeline>,
    view_targets: Query<(Entity, Option<&Tonemapping>, Option<&DebandDither>), With<ViewTarget>>,
) {
    for (entity, tonemapping, dither) in view_targets.iter() {
        let key = TonemappingPipelineKey {
            deband_dither: *dither.unwrap_or(&DebandDither::Disabled),
            tonemapping: *tonemapping.unwrap_or(&Tonemapping::None),
        };
        let pipeline = pipelines.specialize(&pipeline_cache, &upscaling_pipeline, key);

        commands
            .entity(entity)
            .insert(ViewTonemappingPipeline(pipeline));
    }
}
/// Enables a debanding shader that applies dithering to mitigate color banding in the final image for a given [`Camera`] entity.
#[derive(
    Component, Debug, Hash, Clone, Copy, Reflect, Default, ExtractComponent, PartialEq, Eq,
)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component)]
pub enum DebandDither {
    #[default]
    Disabled,
    Enabled,
}

pub fn get_lut_bindings<'a>(
    images: &'a RenderAssets<Image>,
    tonemapping_luts: &'a TonemappingLuts,
    tonemapping: &Tonemapping,
    bindings: [u32; 2],
) -> [BindGroupEntry<'a>; 2] {
    let image = match tonemapping {
        // AgX lut texture used when tonemapping doesn't need a texture since it's very small (32x32x32)
        Tonemapping::None
        | Tonemapping::Reinhard
        | Tonemapping::ReinhardLuminance
        | Tonemapping::AcesFitted
        | Tonemapping::AgX
        | Tonemapping::SomewhatBoringDisplayTransform => &tonemapping_luts.agx,
        Tonemapping::TonyMcMapface => &tonemapping_luts.tony_mc_mapface,
        Tonemapping::BlenderFilmic => &tonemapping_luts.blender_filmic,
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

// allow(dead_code) so it doesn't complain when the tonemapping_luts feature is disabled
#[allow(dead_code)]
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

pub fn lut_placeholder() -> Image {
    let format = TextureFormat::Rgba8Unorm;
    let data = vec![255, 0, 255, 255];
    Image {
        data,
        texture_descriptor: TextureDescriptor {
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            format,
            dimension: TextureDimension::D3,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        sampler_descriptor: ImageSampler::Default,
        texture_view_descriptor: None,
    }
}
