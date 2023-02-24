use bevy_asset::Handle;
use bevy_core_pipeline::gizmo_2d::GizmoLine2d;
use bevy_ecs::{
    prelude::Entity,
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
    view::Msaa,
};
use bevy_sprite::*;
use bevy_utils::FloatOrd;

use crate::{GizmoMesh, LINE_SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct GizmoPipeline2d {
    mesh_pipeline: Mesh2dPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for GizmoPipeline2d {
    fn from_world(render_world: &mut World) -> Self {
        GizmoPipeline2d {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            shader: LINE_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for GizmoPipeline2d {
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
                topology: key.primitive_topology(),
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

pub(crate) type DrawGizmoLines = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    DrawMesh2d,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_gizmos_2d(
    draw_functions: Res<DrawFunctions<GizmoLine2d>>,
    pipeline: Res<GizmoPipeline2d>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedMeshPipelines<GizmoPipeline2d>>,
    gpu_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    mesh_handles: Query<(Entity, &Mesh2dHandle), With<GizmoMesh>>,
    mut views: Query<&mut RenderPhase<GizmoLine2d>>,
) {
    let draw_function = draw_functions.read().id::<DrawGizmoLines>();
    let key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples());
    for mut phase in &mut views {
        for (entity, mesh_handle) in &mesh_handles {
            let Some(mesh) = gpu_meshes.get(&mesh_handle.0) else { continue; };

            let key = key | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);
            let pipeline = specialized_pipelines
                .specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
                .unwrap();
            phase.add(GizmoLine2d {
                entity,
                draw_function,
                pipeline,
                sort_key: FloatOrd(f32::MAX),
            });
        }
    }
}
