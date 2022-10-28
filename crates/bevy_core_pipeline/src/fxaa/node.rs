use std::sync::Mutex;

use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
        CachedRenderPipelineId, FilterMode, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor, SamplerDescriptor, TextureView, TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};
use bevy_utils::default;

use super::{FXAAPipelineBindGroup, FXAAPipelines, FXAATexture, FXAA};

pub struct FXAANode {
    query: QueryState<
        (
            &'static ExtractedView,
            &'static ViewTarget,
            &'static FXAATexture,
            &'static FXAAPipelines,
            &'static FXAA,
        ),
        With<ExtractedView>,
    >,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl FXAANode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
}

impl Node for FXAANode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(FXAANode::IN_VIEW, SlotType::Entity)]
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
        let pipeline_cache = world.resource::<PipelineCache>();
        let fxaa_bind_group = world.resource::<FXAAPipelineBindGroup>();

        let (view, target, fxaa_texture, pipelines, fxaa) =
            match self.query.get_manual(world, view_entity) {
                Ok(result) => result,
                Err(_) => return Ok(()),
            };

        if !fxaa.enabled {
            return Ok(());
        };

        let main_texture = target.main_texture.texture();
        let fxaa_texture = &fxaa_texture.output.default_view;

        let pipeline_id = if view.hdr {
            pipelines.to_ldr_pipeline_id
        } else {
            pipelines.blit_pipeline_id
        };
        self.render_pass(
            render_context,
            pipeline_cache,
            pipeline_id,
            main_texture,
            fxaa_texture,
            &fxaa_bind_group,
        );

        let pipeline_id = if view.hdr {
            pipelines.fxaa_hdr_pipeline_id
        } else {
            pipelines.fxaa_ldr_pipeline_id
        };
        self.render_pass(
            render_context,
            pipeline_cache,
            pipeline_id,
            fxaa_texture,
            main_texture,
            &fxaa_bind_group,
        );

        Ok(())
    }
}

impl FXAANode {
    fn render_pass(
        &self,
        render_context: &mut RenderContext,
        pipeline_cache: &PipelineCache,
        pipeline_id: CachedRenderPipelineId,
        in_texture: &TextureView,
        out_texture: &TextureView,
        bind_group_layout: &BindGroupLayout,
    ) {
        let pipeline = match pipeline_cache.get_render_pipeline(pipeline_id) {
            Some(pipeline) => pipeline,
            None => return,
        };

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if in_texture.id() == *id => bind_group,
            cached_bind_group => {
                let sampler = render_context
                    .render_device
                    .create_sampler(&SamplerDescriptor {
                        mipmap_filter: FilterMode::Linear,
                        mag_filter: FilterMode::Linear,
                        min_filter: FilterMode::Linear,
                        ..default()
                    });

                let bind_group =
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: bind_group_layout,
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
            label: Some("fxaa_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: out_texture,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
