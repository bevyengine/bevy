use super::{PipelineDescriptor, PipelineSpecialization};
use crate::{
    draw::{Draw, DrawContext, OutsideFrustum},
    mesh::{Indices, Mesh},
    prelude::{Msaa, Visible},
    renderer::RenderResourceBindings,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    component::Component,
    query::Without,
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
};
use bevy_reflect::Reflect;
use bevy_utils::HashSet;

#[derive(Debug, Default, Clone, Reflect)]
pub struct RenderPipeline {
    pub pipeline: Handle<PipelineDescriptor>,
    pub specialization: PipelineSpecialization,
    /// used to track if PipelineSpecialization::dynamic_bindings is in sync with
    /// RenderResourceBindings
    pub dynamic_bindings_generation: usize,
}

impl RenderPipeline {
    pub fn new(pipeline: Handle<PipelineDescriptor>) -> Self {
        RenderPipeline {
            specialization: Default::default(),
            pipeline,
            dynamic_bindings_generation: std::usize::MAX,
        }
    }

    pub fn specialized(
        pipeline: Handle<PipelineDescriptor>,
        specialization: PipelineSpecialization,
    ) -> Self {
        RenderPipeline {
            pipeline,
            specialization,
            dynamic_bindings_generation: std::usize::MAX,
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct RenderPipelines {
    pub pipelines: Vec<RenderPipeline>,
    #[reflect(ignore)]
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
                .map(|pipeline| RenderPipeline::new(pipeline.clone_weak()))
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

pub fn draw_render_pipelines_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut query: Query<
        (&mut Draw, &mut RenderPipelines, &Handle<Mesh>, &Visible),
        Without<OutsideFrustum>,
    >,
) {
    for (mut draw, mut render_pipelines, mesh_handle, visible) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        // don't render if the mesh isn't loaded yet
        let mesh = if let Some(mesh) = meshes.get(mesh_handle) {
            mesh
        } else {
            continue;
        };

        let index_range = match mesh.indices() {
            Some(Indices::U32(indices)) => Some(0..indices.len() as u32),
            Some(Indices::U16(indices)) => Some(0..indices.len() as u32),
            None => None,
        };

        let render_pipelines = &mut *render_pipelines;
        for pipeline in render_pipelines.pipelines.iter_mut() {
            pipeline.specialization.sample_count = msaa.samples;
            if pipeline.dynamic_bindings_generation
                != render_pipelines.bindings.dynamic_bindings_generation()
            {
                pipeline.specialization.dynamic_bindings = render_pipelines
                    .bindings
                    .iter_dynamic_bindings()
                    .map(|name| name.to_string())
                    .collect::<HashSet<String>>();
                pipeline.dynamic_bindings_generation =
                    render_pipelines.bindings.dynamic_bindings_generation();
                for (handle, _) in render_pipelines.bindings.iter_assets() {
                    if let Some(bindings) = draw_context
                        .asset_render_resource_bindings
                        .get_untyped(handle)
                    {
                        for binding in bindings.iter_dynamic_bindings() {
                            pipeline
                                .specialization
                                .dynamic_bindings
                                .insert(binding.to_string());
                        }
                    }
                }
            }
        }

        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            let render_resource_bindings = &mut [
                &mut render_pipelines.bindings,
                &mut render_resource_bindings,
            ];
            draw_context
                .set_pipeline(
                    &mut draw,
                    &render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();
            draw_context
                .set_bind_groups_from_bindings(&mut draw, render_resource_bindings)
                .unwrap();
            draw_context
                .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
                .unwrap();

            if let Some(indices) = index_range.clone() {
                draw.draw_indexed(indices, 0, 0..1);
            } else {
                draw.draw(0..mesh.count_vertices() as u32, 0..1)
            }
        }
    }
}
