use std::sync::Mutex;

use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType, SlotValue},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, LoadOp, Operations,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineCache, SamplerDescriptor,
        TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewMainTexture, ViewTarget},
};

use super::{TonemappingPipeline, TonemappingTarget};

pub struct TonemappingNode {
    query: QueryState<(&'static ViewTarget, &'static TonemappingTarget), With<ExtractedView>>,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl TonemappingNode {
    pub const IN_VIEW: &'static str = "view";
    pub const IN_TEXTURE: &'static str = "in_texture";
    pub const OUT_TEXTURE: &'static str = "out_texture";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
}

impl Node for TonemappingNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(TonemappingNode::IN_TEXTURE, SlotType::TextureView),
            SlotInfo::new(TonemappingNode::IN_VIEW, SlotType::Entity),
        ]
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(
            TonemappingNode::OUT_TEXTURE,
            SlotType::TextureView,
        )]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let in_texture = graph.get_input_texture(Self::IN_TEXTURE)?;

        let render_pipeline_cache = world.resource::<RenderPipelineCache>();
        let tonemapping_pipeline = world.resource::<TonemappingPipeline>();

        let (target, tonemapping_target) = match self.query.get_manual(world, view_entity) {
            Ok(query) => query,
            Err(_) => return Ok(()),
        };

        let pipeline = match render_pipeline_cache.get(tonemapping_target.pipeline) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let ldr_texture = match &target.main_texture {
            ViewMainTexture::Hdr { ldr_texture, .. } => ldr_texture,
            ViewMainTexture::Sdr { .. } => {
                // non-hdr does tone mapping in the main pass node
                let in_texture = in_texture.clone();
                graph
                    .set_output(
                        TonemappingNode::OUT_TEXTURE,
                        SlotValue::TextureView(in_texture),
                    )
                    .unwrap();
                return Ok(());
            }
        };

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if in_texture.id() == *id => bind_group,
            cached_bind_group => {
                let sampler = render_context
                    .render_device
                    .create_sampler(&SamplerDescriptor::default());

                let bind_group =
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &tonemapping_pipeline.hdr_texture_bind_group,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(in_texture),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&sampler),
                                },
                            ],
                        });

                let (_, bind_group) = cached_bind_group.insert((in_texture.id(), bind_group));
                bind_group
            }
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("tonemapping_pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: ldr_texture,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Default::default()), // TODO shouldn't need to be cleared
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        graph
            .set_output(
                TonemappingNode::OUT_TEXTURE,
                SlotValue::TextureView(ldr_texture.clone()),
            )
            .unwrap();

        Ok(())
    }
}
