use super::{ComputePipelineDescriptor, PipelineSpecialization};
use crate::{
    draw::{Draw, DrawContext},
    prelude::Msaa,
    renderer::RenderResourceBindings,
};
use bevy_asset::Handle;
use bevy_ecs::{Query, Res, ResMut};
use bevy_property::Properties;
#[derive(Properties, Default, Clone)]
pub struct ComputePipeline {
    pub pipeline: Handle<ComputePipelineDescriptor>,
    pub specialization: PipelineSpecialization,
}

impl ComputePipeline {
    pub fn new(pipeline: Handle<ComputePipelineDescriptor>) -> Self {
        ComputePipeline {
            pipeline,
            ..Default::default()
        }
    }

    pub fn specialized(
        pipeline: Handle<ComputePipelineDescriptor>,
        specialization: PipelineSpecialization,
    ) -> Self {
        ComputePipeline {
            pipeline,
            specialization,
            ..Default::default()
        }
    }
}

#[derive(Properties)]
pub struct ComputePipelines {
    pub pipelines: Vec<ComputePipeline>,
    #[property(ignore)]
    pub bindings: RenderResourceBindings,
}

impl ComputePipelines {
    pub fn from_pipelines(pipelines: Vec<ComputePipeline>) -> Self {
        Self {
            pipelines,
            ..Default::default()
        }
    }

    pub fn from_handles<'a, T: IntoIterator<Item = &'a Handle<ComputePipelineDescriptor>>>(
        handles: T,
    ) -> Self {
        ComputePipelines {
            pipelines: handles
                .into_iter()
                .map(|pipeline| ComputePipeline::new(*pipeline))
                .collect::<Vec<ComputePipeline>>(),
            ..Default::default()
        }
    }
}

impl Default for ComputePipelines {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            pipelines: vec![ComputePipeline::default()],
        }
    }
}

pub fn draw_compute_pipelines_system(
    mut _draw_context: DrawContext,
    mut _render_resource_bindings: ResMut<RenderResourceBindings>,
    _msaa: Res<Msaa>,
    mut query: Query<(&mut Draw, &mut ComputePipelines)>,
) {
    // TODO: Compute doesn't have a concept of "drawing" here.
    // We likely want a "ComputeCommand" type
    for (mut _draw, mut _render_pipelines) in &mut query.iter() {
       todo!();
    }
}
