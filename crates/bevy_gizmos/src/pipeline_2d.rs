use bevy_asset::Handle;
use bevy_core_pipeline::core_2d::Transparent2d;
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
    view::{ExtractedView, Msaa, ViewTarget},
};
use bevy_sprite::*;
use bevy_utils::FloatOrd;

use crate::{GizmoMesh, LINE_SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct GizmoLinePipeline {
    mesh_pipeline: Mesh2dPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for GizmoLinePipeline {
    fn from_world(render_world: &mut World) -> Self {
        GizmoLinePipeline {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            shader: LINE_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for GizmoLinePipeline {
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

        let format = if key.contains(Mesh2dPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

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
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.mesh_pipeline.view_layout.clone()],
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
            label: Some("gizmo_2d_pipeline".into()),
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
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<GizmoLinePipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedMeshPipelines<GizmoLinePipeline>>,
    gpu_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    mesh_handles: Query<(Entity, &Mesh2dHandle), With<GizmoMesh>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent2d>)>,
) {
    let draw_function = draw_functions.read().get_id::<DrawGizmoLines>().unwrap();
    let key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples());
    for (view, mut phase) in &mut views {
        let key = key | Mesh2dPipelineKey::from_hdr(view.hdr);
        for (entity, mesh_handle) in &mesh_handles {
            let Some(mesh) = gpu_meshes.get(&mesh_handle.0) else { continue; };

            let key = key | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);
            let pipeline = specialized_pipelines
                .specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
                .unwrap();
            phase.add(Transparent2d {
                entity,
                draw_function,
                pipeline,
                sort_key: FloatOrd(f32::MAX),
                batch_range: None,
            });
        }
    }
}
