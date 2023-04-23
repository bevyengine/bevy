use bevy_asset::Handle;
use bevy_core_pipeline::core_3d::Opaque3d;
use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_pbr::*;
use bevy_render::{
    mesh::Mesh,
    render_resource::Shader,
    view::{ExtractedView, ViewTarget},
};
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    texture::BevyDefault,
    view::Msaa,
};

use crate::{GizmoConfig, GizmoMesh, LINE_SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct GizmoPipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for GizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        GizmoPipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            shader: LINE_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for GizmoPipeline {
    type Key = (bool, MeshPipelineKey);

    fn specialize(
        &self,
        (depth_test, key): Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        shader_defs.push("GIZMO_LINES_3D".into());
        shader_defs.push(ShaderDefVal::Int(
            "MAX_DIRECTIONAL_LIGHTS".to_string(),
            MAX_DIRECTIONAL_LIGHTS as i32,
        ));
        shader_defs.push(ShaderDefVal::Int(
            "MAX_CASCADES_PER_LIGHT".to_string(),
            MAX_CASCADES_PER_LIGHT as i32,
        ));
        if depth_test {
            shader_defs.push("DEPTH_TEST".into());
        }

        let vertex_buffer_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
        ])?;
        let bind_group_layout = match key.msaa_samples() {
            1 => vec![self.mesh_pipeline.view_layout.clone()],
            _ => {
                shader_defs.push("MULTISAMPLED".into());
                vec![self.mesh_pipeline.view_layout_multisampled.clone()]
            }
        };

        let format = if key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

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
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: bind_group_layout,
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
            label: Some("gizmo_3d_pipeline".into()),
        })
    }
}

pub(crate) type DrawGizmoLines = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_gizmos_3d(
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    pipeline: Res<GizmoPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    mesh_handles: Query<(Entity, &Handle<Mesh>), With<GizmoMesh>>,
    config: Res<GizmoConfig>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_function = draw_functions.read().get_id::<DrawGizmoLines>().unwrap();
    let key = MeshPipelineKey::from_msaa_samples(msaa.samples());
    for (view, mut phase) in &mut views {
        let key = key | MeshPipelineKey::from_hdr(view.hdr);
        for (entity, mesh_handle) in &mesh_handles {
            if let Some(mesh) = render_meshes.get(mesh_handle) {
                let key = key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(
                        &pipeline_cache,
                        &pipeline,
                        (!config.on_top, key),
                        &mesh.layout,
                    )
                    .unwrap();
                phase.add(Opaque3d {
                    entity,
                    pipeline,
                    draw_function,
                    distance: 0.,
                });
            }
        }
    }
}
