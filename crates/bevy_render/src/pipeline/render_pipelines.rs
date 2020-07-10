use super::{PipelineDescriptor, PipelineSpecialization};
use crate::{
    draw::{Draw, DrawContext, DrawError, Drawable},
    render_resource::RenderResourceBindings,
};
use bevy_asset::Handle;
use bevy_property::Properties;
use bevy_ecs::{Query, ResMut};
#[derive(Properties, Default, Clone)]
pub struct RenderPipeline {
    pub pipeline: Handle<PipelineDescriptor>,
    #[property(ignore)]
    pub specialization: PipelineSpecialization,
}

impl RenderPipeline {
    pub fn new(pipeline: Handle<PipelineDescriptor>) -> Self {
        RenderPipeline {
            pipeline,
            ..Default::default()
        }
    }

    pub fn specialized(
        pipeline: Handle<PipelineDescriptor>,
        specialization: PipelineSpecialization,
    ) -> Self {
        RenderPipeline {
            pipeline,
            specialization,
            ..Default::default()
        }
    }
}

#[derive(Properties)]
pub struct RenderPipelines {
    pub pipelines: Vec<RenderPipeline>,
    #[property(ignore)]
    pub bindings: RenderResourceBindings,
}

impl RenderPipelines {
    pub fn from_pipelines(pipelines: Vec<RenderPipeline>) -> Self {
        Self {
            pipelines,
            ..Default::default()
        }
    }

    pub fn from_handles<'a, T: IntoIterator<Item = &'a Handle<PipelineDescriptor>>>(
        handles: T,
    ) -> Self {
        RenderPipelines {
            pipelines: handles
                .into_iter()
                .map(|pipeline| RenderPipeline::new(*pipeline))
                .collect::<Vec<RenderPipeline>>(),
            ..Default::default()
        }
    }
}

impl Default for RenderPipelines {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            pipelines: vec![RenderPipeline::default()],
        }
    }
}

pub struct DrawableRenderPipelines<'a> {
    pub render_pipelines: &'a mut RenderPipelines,
    pub render_resource_bindings: &'a mut RenderResourceBindings,
}

impl<'a> Drawable for DrawableRenderPipelines<'a> {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError> {
        for render_pipeline in self.render_pipelines.pipelines.iter() {
            context.set_pipeline(
                draw,
                render_pipeline.pipeline,
                &render_pipeline.specialization,
            )?;
            context.set_bind_groups_from_bindings(
                draw,
                &mut [
                    &mut self.render_pipelines.bindings,
                    self.render_resource_bindings,
                ],
            )?;
            let indices = context
                .set_vertex_buffers_from_bindings(draw, &[&self.render_pipelines.bindings])?;
            if let Some(indices) = indices {
                draw.draw_indexed(indices, 0, 0..1);
            }
        }

        Ok(())
    }
}

pub fn draw_render_pipelines_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut query: Query<(&mut Draw, &mut RenderPipelines)>,
) {
    for (mut draw, mut render_pipelines) in &mut query.iter() {
        let mut drawable = DrawableRenderPipelines {
            render_pipelines: &mut render_pipelines,
            render_resource_bindings: &mut render_resource_bindings,
        };
        drawable.draw(&mut draw, &mut draw_context).unwrap();
    }
}
