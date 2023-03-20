use crate::fxaa::{CameraFxaaPipeline, Fxaa, FxaaPipeline};
use bevy_ecs::{prelude::*, query::QueryState};
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, FilterMode, PipelineCache,
        SamplerDescriptor, TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};
use bevy_utils::default;
use std::sync::Mutex;

pub struct FxaaNode {
    query: QueryState<
        (
            &'static ViewTarget,
            &'static CameraFxaaPipeline,
            &'static Fxaa,
        ),
        With<ExtractedView>,
    >,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl FxaaNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
}

impl Node for FxaaNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(FxaaNode::IN_VIEW, SlotType::Entity)]
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
        let fxaa_pipeline = world.resource::<FxaaPipeline>();

        let (target, pipeline, fxaa) = match self.query.get_manual(world, view_entity) {
            Ok(result) => result,
            Err(_) => return Ok(()),
        };

        if !fxaa.enabled {
            return Ok(());
        };

        let pipeline = pipeline_cache
            .get_render_pipeline(pipeline.pipeline_id)
            .unwrap();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;
        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if source.id() == *id => bind_group,
            cached_bind_group => {
                let sampler = render_context
                    .render_device()
                    .create_sampler(&SamplerDescriptor {
                        mipmap_filter: FilterMode::Linear,
                        mag_filter: FilterMode::Linear,
                        min_filter: FilterMode::Linear,
                        ..default()
                    });

                let bind_group =
                    render_context
                        .render_device()
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &fxaa_pipeline.texture_bind_group,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(source),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&sampler),
                                },
                            ],
                        });

                let (_, bind_group) = cached_bind_group.insert((source.id(), bind_group));
                bind_group
            }
        };

        render_context
            .render_pass(view_entity)
            .set_label("fxaa_pass")
            .add_color_attachment(destination)
            .begin()
            .set_pipeline(pipeline)
            .set_bind_group(0, bind_group, &[])
            .draw(0..3, 0..1);

        Ok(())
    }
}
