use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::prelude::*;
use bevy_asset::{
    load_internal_asset, load_internal_binary_asset, AssetServer, Handle, HandleUntyped,
};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_reflect::{FromReflect, Reflect, ReflectFromReflect, TypeUuid};
use bevy_render::camera::Camera;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::{CompressedImageFormats, Image, ImageType};
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, RenderApp, RenderSet};

mod node;

pub use node::TonemappingNode;

const TONEMAPPING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17015368199668024512);

const TONEMAPPING_SHARED_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2499430578245347910);

const AGX_LUT_IMAGE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1419536523291344910);

#[derive(Resource)]
struct AGXLut(Handle<Image>);

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

        // TODO when this works remove luts from assets
        //load_internal_binary_asset!(
        //    app,
        //    AGX_LUT_IMAGE_HANDLE,
        //    "luts/AgX-default_contrast.lut.exr",
        //    |bytes| -> Image {
        //        Image::from_buffer(
        //            bytes,
        //            ImageType::Extension("exr"),
        //            CompressedImageFormats::NONE,
        //            false,
        //        )
        //        .unwrap()
        //    }
        //);

        let mut state = SystemState::<Res<AssetServer>>::new(&mut app.world);
        let asset_server = state.get_mut(&mut app.world);
        let agx_lut = asset_server.load("luts/AgX-default_contrast.lut.exr");

        app.register_type::<Tonemapping>();

        app.add_plugin(ExtractComponentPlugin::<Tonemapping>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(AGXLut(agx_lut))
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

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Reflect, FromReflect)]
#[reflect(FromReflect)]
pub enum TonemappingMethod {
    None,
    /// Suffers from lots hue shifting, brights don't desaturate naturally.
    Reinhard,
    /// old bevy default. Suffers from hue shifting, brights don't desaturate much at all.
    ReinhardLuminance,
    /// Bad
    Aces,
    /// Good
    #[default]
    AgX,
    /// Also good
    SBDT,
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
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into())
            }
            TonemappingMethod::Aces => shader_defs.push("TONEMAP_METHOD_ACES".into()),
            TonemappingMethod::AgX => shader_defs.push("TONEMAP_METHOD_AGX".into()),
            TonemappingMethod::SBDT => shader_defs.push("TONEMAP_METHOD_SBDT".into()),
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
        let tonemap_texture_bind_group = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("tonemapping_hdr_texture_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
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

#[derive(Component, Clone, Reflect, Default, ExtractComponent)]
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
