mod node;

pub use node::UpscalingNode;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_render::renderer::{RenderDevice, SurfaceTextureFormat};
use bevy_render::view::ExtractedView;
use bevy_render::{render_resource::*, RenderApp, RenderStage};

use bevy_reflect::TypeUuid;

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;

const UPSCALING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 14589267395627146578);

pub struct UpscalingPlugin;

impl Plugin for UpscalingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UPSCALING_SHADER_HANDLE,
            "upscaling.wgsl",
            Shader::from_wgsl
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<UpscalingPipeline>()
            .init_resource::<SpecializedRenderPipelines<UpscalingPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_upscaling_bind_groups);
    }
}

#[derive(Resource)]
pub struct UpscalingPipeline {
    ldr_texture_bind_group: BindGroupLayout,
    surface_texture_format: TextureFormat,
}

impl FromWorld for UpscalingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let surface_texture_format = render_world.resource::<SurfaceTextureFormat>().0;

        let ldr_texture_bind_group =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("upscaling_ldr_texture_bind_group_layout"),
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
                ],
            });

        UpscalingPipeline {
            ldr_texture_bind_group,
            surface_texture_format,
        }
    }
}

#[repr(u8)]
pub enum UpscalingMode {
    Filtering = 0,
    Nearest = 1,
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct UpscalingPipelineKey: u32 {
        const NONE                         = 0;
        const UPSCALING_MODE_RESERVED_BITS = UpscalingPipelineKey::UPSCALING_MODE_MASK_BITS << UpscalingPipelineKey::UPSCALING_MODE_SHIFT_BITS;
    }
}

impl UpscalingPipelineKey {
    const UPSCALING_MODE_MASK_BITS: u32 = 0b1111; // enough for 16 different modes
    const UPSCALING_MODE_SHIFT_BITS: u32 = 32 - 4;

    pub fn from_upscaling_mode(upscaling_mode: UpscalingMode) -> Self {
        let upscaling_mode_bits = ((upscaling_mode as u32) & Self::UPSCALING_MODE_MASK_BITS)
            << Self::UPSCALING_MODE_SHIFT_BITS;
        UpscalingPipelineKey::from_bits(upscaling_mode_bits).unwrap()
    }

    pub fn upscaling_mode(&self) -> UpscalingMode {
        let upscaling_mode_bits =
            (self.bits >> Self::UPSCALING_MODE_SHIFT_BITS) & Self::UPSCALING_MODE_MASK_BITS;
        match upscaling_mode_bits {
            0 => UpscalingMode::Filtering,
            1 => UpscalingMode::Nearest,
            other => panic!("invalid upscaling mode bits in UpscalingPipelineKey: {other}"),
        }
    }
}

impl SpecializedRenderPipeline for UpscalingPipeline {
    type Key = UpscalingPipelineKey;

    fn specialize(&self, _: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("upscaling pipeline".into()),
            layout: Some(vec![self.ldr_texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: UPSCALING_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: self.surface_texture_format,
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

#[derive(Component)]
pub struct UpscalingTarget {
    pub pipeline: CachedRenderPipelineId,
}

fn queue_upscaling_bind_groups(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UpscalingPipeline>>,
    upscaling_pipeline: Res<UpscalingPipeline>,
    view_targets: Query<Entity, With<ExtractedView>>,
) {
    for entity in view_targets.iter() {
        let key = UpscalingPipelineKey::from_upscaling_mode(UpscalingMode::Filtering);
        let pipeline = pipelines.specialize(&mut pipeline_cache, &upscaling_pipeline, key);

        commands.entity(entity).insert(UpscalingTarget { pipeline });
    }
}
