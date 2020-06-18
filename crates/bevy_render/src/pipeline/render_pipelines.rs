use super::{PipelineDescriptor, PipelineSpecialization};
use crate::{
    draw::{DrawContext, DrawError, Drawable},
    render_resource::RenderResourceBindings,
};
use bevy_asset::Handle;
use bevy_property::Properties;
#[derive(Properties, Default, Clone)]
pub struct RenderPipeline {
    pub pipeline: Handle<PipelineDescriptor>,
    #[property(ignore)]
    pub specialized_pipeline: Option<Handle<PipelineDescriptor>>,
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

    pub fn specialized(pipeline: Handle<PipelineDescriptor>, specialization: PipelineSpecialization) -> Self {
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

impl Drawable for RenderPipelines {
    fn draw(&mut self, draw: &mut DrawContext) -> Result<(), DrawError> {
        for render_pipeline in self.pipelines.iter() {
            let specialized_handle = if let Some(handle) = render_pipeline.specialized_pipeline {
                handle
            } else {
                continue;
            };
            let pipeline = draw.pipelines.get(&specialized_handle).unwrap();
            let layout = pipeline.get_layout().unwrap();
            draw.set_pipeline(specialized_handle)?;
            for bind_group_descriptor in layout.bind_groups.iter() {
                if let Some(local_bind_group) = self
                    .bindings
                    .get_descriptor_bind_group(bind_group_descriptor.id)
                {
                    draw.set_bind_group(bind_group_descriptor.index, local_bind_group);
                } else if let Some(global_bind_group) = draw
                    .render_resource_bindings
                    .get_descriptor_bind_group(bind_group_descriptor.id)
                {
                    draw.set_bind_group(bind_group_descriptor.index, global_bind_group);
                }
            }
            let mut indices = 0..0;
            for (slot, vertex_buffer_descriptor) in
                layout.vertex_buffer_descriptors.iter().enumerate()
            {
                if let Some((vertex_buffer, index_buffer)) = self
                    .bindings
                    .get_vertex_buffer(&vertex_buffer_descriptor.name)
                {
                    draw.set_vertex_buffer(slot as u32, vertex_buffer, 0);
                    if let Some(index_buffer) = index_buffer {
                        if let Some(buffer_info) =
                            draw.render_resource_context.get_buffer_info(index_buffer)
                        {
                            indices = 0..(buffer_info.size / 2) as u32;
                        } else {
                            panic!("expected buffer type");
                        }
                        draw.set_index_buffer(index_buffer, 0);
                    }
                }
            }

            draw.draw_indexed(indices, 0, 0..1);
        }

        Ok(())
    }
}
