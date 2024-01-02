use bevy_asset::Handle;
use bevy_ecs::prelude::{FromWorld, Resource, World};
use bevy_render::{
    prelude::Shader,
    render_resource::{
        BindGroupLayout, BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType,
        ColorTargetState, ColorWrites, FragmentState, MultisampleState, PrimitiveState,
        RenderPipelineDescriptor, ShaderStages, VertexState,
    },
    renderer::RenderDevice,
    texture::BevyDefault,
};

use super::OVERLAY_SHADER_HANDLE;

#[derive(Clone, Resource)]
pub(crate) struct OverlayPipeline {
    shader: Handle<Shader>,
    pub(crate) layout: Vec<BindGroupLayout>,
}

impl FromWorld for OverlayPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let diagnostics_bind_group_layout = render_device.create_bind_group_layout(
            "diagnostics_bind_group_layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );

        Self {
            shader: OVERLAY_SHADER_HANDLE,
            layout: vec![diagnostics_bind_group_layout],
        }
    }
}

impl OverlayPipeline {
    pub(crate) fn get_pipeline(&self) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("Overlay Pipeline".into()),
            layout: self.layout.clone(),
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "vs_main".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: BevyDefault::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        }
    }
}
