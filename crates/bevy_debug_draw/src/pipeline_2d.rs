use bevy_asset::Handle;
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::{
    query::With,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    texture::BevyDefault,
    view::{Msaa, VisibleEntities},
};
use bevy_sprite::*;
use bevy_utils::FloatOrd;

use crate::{DebugDrawMesh, SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct DebugLinePipeline {
    mesh_pipeline: Mesh2dPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for DebugLinePipeline {
    fn from_world(render_world: &mut World) -> Self {
        DebugLinePipeline {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            shader: SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for DebugLinePipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let vertex_buffer_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
        ])?;

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone_weak(),
                entry_point: "vertex".into(),
                shader_defs: vec![],
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone_weak(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: Some(vec![self.mesh_pipeline.view_layout.clone()]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::LineList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: None,
        })
    }
}

pub(crate) type DrawDebugLines = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    DrawMesh2d,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue(
    draw2d_functions: Res<DrawFunctions<Transparent2d>>,
    debug_line_pipeline: Res<DebugLinePipeline>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedMeshPipelines<DebugLinePipeline>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    material_meshes: Query<&Mesh2dHandle, With<DebugDrawMesh>>,
    mut views: Query<(&VisibleEntities, &mut RenderPhase<Transparent2d>)>,
) {
    for (view, mut phase) in &mut views {
        let draw_mesh2d = draw2d_functions.read().get_id::<DrawDebugLines>().unwrap();
        let msaa_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &view.entities {
            let Ok(mesh_handle) = material_meshes.get(*visible_entity) else { continue; };
            let Some(mesh) = render_meshes.get(&mesh_handle.0) else { continue; };

            let mesh_key =
                msaa_key | Mesh2dPipelineKey::from_primitive_topology(PrimitiveTopology::LineList);
            let pipeline = specialized_pipelines
                .specialize(
                    &mut pipeline_cache,
                    &debug_line_pipeline,
                    mesh_key,
                    &mesh.layout,
                )
                .unwrap();
            phase.add(Transparent2d {
                entity: *visible_entity,
                draw_function: draw_mesh2d,
                pipeline,
                sort_key: FloatOrd(f32::MAX),
                batch_range: None,
            });
        }
    }
}
