use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy_render::render_asset::{RenderAssetUsages, RenderAssets};
use bevy_render::render_resource::binding_types::{
    sampler, texture_2d, texture_3d, uniform_buffer,
};
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::{CompressedImageFormats, GpuImage, Image, ImageSampler, ImageType};
use bevy_render::view::{ExtractedView, ViewTarget, ViewUniform};
use bevy_render::{camera::Camera, texture::FallbackImage};
use bevy_render::{render_resource::*, Render, RenderApp, RenderSet};
#[cfg(not(feature = "tonemapping_luts"))]
use bevy_utils::tracing::error;
use bitflags::bitflags;

mod node;

use bevy_utils::default;
pub use node::TonemappingNode;

const TONEMAPPING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(17015368199668024512);

const TONEMAPPING_SHARED_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2499430578245347910);

const TONEMAPPING_LUT_BINDINGS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(8392056472189465073);

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
        load_internal_asset!(
            app,
            TONEMAPPING_LUT_BINDINGS_SHADER_HANDLE,
            "lut_bindings.wgsl",
            Shader::from_wgsl
        );

        if !app.world().is_resource_added::<TonemappingLuts>() {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();

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

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<TonemappingPipeline>>()
            .add_systems(
                Render,
                prepare_view_tonemapping_pipelines.in_set(RenderSet::Prepare),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<TonemappingPipeline>();
    }
}

#[derive(Resource)]
pub struct TonemappingPipeline {
    texture_bind_group: BindGroupLayout,
    sampler: Sampler,
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
    /// Is sort of between Reinhard and `ReinhardLuminance`. Conceptually similar to reinhard-jodie.
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

bitflags! {
    /// Various flags describing what tonemapping needs to do.
    ///
    /// This allows the shader to skip unneeded steps.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TonemappingPipelineKeyFlags: u8 {
        /// The hue needs to be changed.
        const HUE_ROTATE                = 0x01;
        /// The white balance needs to be adjusted.
        const WHITE_BALANCE             = 0x02;
        /// Saturation/contrast/gamma/gain/lift for one or more sections
        /// (shadows, midtones, highlights) need to be adjusted.
        const SECTIONAL_COLOR_GRADING   = 0x04;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TonemappingPipelineKey {
    deband_dither: DebandDither,
    tonemapping: Tonemapping,
    flags: TonemappingPipelineKeyFlags,
}

impl SpecializedRenderPipeline for TonemappingPipeline {
    type Key = TonemappingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        shader_defs.push(ShaderDefVal::UInt(
            "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
            3,
        ));
        shader_defs.push(ShaderDefVal::UInt(
            "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
            4,
        ));

        if let DebandDither::Enabled = key.deband_dither {
            shader_defs.push("DEBAND_DITHER".into());
        }

        // Define shader flags depending on the color grading options in use.
        if key.flags.contains(TonemappingPipelineKeyFlags::HUE_ROTATE) {
            shader_defs.push("HUE_ROTATE".into());
        }
        if key
            .flags
            .contains(TonemappingPipelineKeyFlags::WHITE_BALANCE)
        {
            shader_defs.push("WHITE_BALANCE".into());
        }
        if key
            .flags
            .contains(TonemappingPipelineKeyFlags::SECTIONAL_COLOR_GRADING)
        {
            shader_defs.push("SECTIONAL_COLOR_GRADING".into());
        }

        match key.tonemapping {
            Tonemapping::None => shader_defs.push("TONEMAP_METHOD_NONE".into()),
            Tonemapping::Reinhard => shader_defs.push("TONEMAP_METHOD_REINHARD".into()),
            Tonemapping::ReinhardLuminance => {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            }
            Tonemapping::AcesFitted => shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into()),
            Tonemapping::AgX => {
                #[cfg(not(feature = "tonemapping_luts"))]
                error!(
                    "AgX tonemapping requires the `tonemapping_luts` feature.
                    Either enable the `tonemapping_luts` feature for bevy in `Cargo.toml` (recommended),
                    or use a different `Tonemapping` method in your `Camera2dBundle`/`Camera3dBundle`."
                );
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            }
            Tonemapping::SomewhatBoringDisplayTransform => {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            }
            Tonemapping::TonyMcMapface => {
                #[cfg(not(feature = "tonemapping_luts"))]
                error!(
                    "TonyMcMapFace tonemapping requires the `tonemapping_luts` feature.
                    Either enable the `tonemapping_luts` feature for bevy in `Cargo.toml` (recommended),
                    or use a different `Tonemapping` method in your `Camera2dBundle`/`Camera3dBundle`."
                );
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }
            Tonemapping::BlenderFilmic => {
                #[cfg(not(feature = "tonemapping_luts"))]
                error!(
                    "BlenderFilmic tonemapping requires the `tonemapping_luts` feature.
                    Either enable the `tonemapping_luts` feature for bevy in `Cargo.toml` (recommended),
                    or use a different `Tonemapping` method in your `Camera2dBundle`/`Camera3dBundle`."
                );
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
        let mut entries = DynamicBindGroupLayoutEntries::new_with_indices(
            ShaderStages::FRAGMENT,
            (
                (0, uniform_buffer::<ViewUniform>(true)),
                (
                    1,
                    texture_2d(TextureSampleType::Float { filterable: false }),
                ),
                (2, sampler(SamplerBindingType::NonFiltering)),
            ),
        );
        let lut_layout_entries = get_lut_bind_group_layout_entries();
        entries =
            entries.extend_with_indices(((3, lut_layout_entries[0]), (4, lut_layout_entries[1])));

        let render_device = render_world.resource::<RenderDevice>();
        let tonemap_texture_bind_group = render_device
            .create_bind_group_layout("tonemapping_hdr_texture_bind_group_layout", &entries);

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        TonemappingPipeline {
            texture_bind_group: tonemap_texture_bind_group,
            sampler,
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
    view_targets: Query<
        (
            Entity,
            &ExtractedView,
            Option<&Tonemapping>,
            Option<&DebandDither>,
        ),
        With<ViewTarget>,
    >,
) {
    for (entity, view, tonemapping, dither) in view_targets.iter() {
        // As an optimization, we omit parts of the shader that are unneeded.
        let mut flags = TonemappingPipelineKeyFlags::empty();
        flags.set(
            TonemappingPipelineKeyFlags::HUE_ROTATE,
            view.color_grading.global.hue != 0.0,
        );
        flags.set(
            TonemappingPipelineKeyFlags::WHITE_BALANCE,
            view.color_grading.global.temperature != 0.0 || view.color_grading.global.tint != 0.0,
        );
        flags.set(
            TonemappingPipelineKeyFlags::SECTIONAL_COLOR_GRADING,
            view.color_grading
                .all_sections()
                .any(|section| *section != default()),
        );

        let key = TonemappingPipelineKey {
            deband_dither: *dither.unwrap_or(&DebandDither::Disabled),
            tonemapping: *tonemapping.unwrap_or(&Tonemapping::None),
            flags,
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
    images: &'a RenderAssets<GpuImage>,
    tonemapping_luts: &'a TonemappingLuts,
    tonemapping: &Tonemapping,
    fallback_image: &'a FallbackImage,
) -> (&'a TextureView, &'a Sampler) {
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
    let lut_image = images.get(image).unwrap_or(&fallback_image.d3);
    (&lut_image.texture_view, &lut_image.sampler)
}

pub fn get_lut_bind_group_layout_entries() -> [BindGroupLayoutEntryBuilder; 2] {
    [
        texture_3d(TextureSampleType::Float { filterable: true }),
        sampler(SamplerBindingType::Filtering),
    ]
}

// allow(dead_code) so it doesn't complain when the tonemapping_luts feature is disabled
#[allow(dead_code)]
fn setup_tonemapping_lut_image(bytes: &[u8], image_type: ImageType) -> Image {
    let image_sampler = ImageSampler::Descriptor(bevy_render::texture::ImageSamplerDescriptor {
        label: Some("Tonemapping LUT sampler".to_string()),
        address_mode_u: bevy_render::texture::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy_render::texture::ImageAddressMode::ClampToEdge,
        address_mode_w: bevy_render::texture::ImageAddressMode::ClampToEdge,
        mag_filter: bevy_render::texture::ImageFilterMode::Linear,
        min_filter: bevy_render::texture::ImageFilterMode::Linear,
        mipmap_filter: bevy_render::texture::ImageFilterMode::Linear,
        ..default()
    });
    Image::from_buffer(
        #[cfg(all(debug_assertions, feature = "dds"))]
        "Tonemapping LUT sampler".to_string(),
        bytes,
        image_type,
        CompressedImageFormats::NONE,
        false,
        image_sampler,
        RenderAssetUsages::RENDER_WORLD,
    )
    .unwrap()
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
        sampler: ImageSampler::Default,
        texture_view_descriptor: None,
        asset_usage: RenderAssetUsages::RENDER_WORLD,
    }
}
