use bevy_asset::Handle;
use bevy_core_pipeline::core_3d::Opaque3d;
use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_pbr::*;
use bevy_render::{mesh::Mesh, render_resource::Shader};
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    texture::BevyDefault,
    view::{ExtractedView, Msaa},
};

use crate::{DebugDrawConfig, DebugDrawMesh, SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct DebugLinePipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for DebugLinePipeline {
    fn from_world(render_world: &mut World) -> Self {
        DebugLinePipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            shader: SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for DebugLinePipeline {
    type Key = (bool, MeshPipelineKey);

    fn specialize(
        &self,
        (depth_test, key): Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        shader_defs.push("DEBUG_LINES_3D".to_string());
        if depth_test {
            shader_defs.push("DEPTH_TEST".to_string());
        }

        let vertex_buffer_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
        ])?;
        let (label, blend, depth_write_enabled);
        if key.contains(MeshPipelineKey::TRANSPARENT_MAIN_PASS) {
            label = "transparent_mesh_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha
            // blended but their depth is not written to the depth buffer.
            depth_write_enabled = false;
        } else {
            label = "opaque_mesh_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer
            // will replace the current fragment value in the output and the depth is
            // written to the depth buffer.
            depth_write_enabled = true;
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone_weak(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone_weak(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend,
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
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        })
    }
}

pub(crate) type DrawDebugLines = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    debug_line_pipeline: Res<DebugLinePipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<DebugLinePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<DebugDrawMesh>>,
    config: Res<DebugDrawConfig>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = opaque_3d_draw_functions
        .read()
        .get_id::<DrawDebugLines>()
        .unwrap();
    let key = MeshPipelineKey::from_msaa_samples(msaa.samples);
    for (view, mut phase) in &mut views {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform, mesh_handle) in &material_meshes {
            if let Some(mesh) = render_meshes.get(mesh_handle) {
                let pipeline = pipelines
                    .specialize(
                        &mut pipeline_cache,
                        &debug_line_pipeline,
                        (!config.always_on_top, key),
                        &mesh.layout,
                    )
                    .unwrap();
                phase.add(Opaque3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}
