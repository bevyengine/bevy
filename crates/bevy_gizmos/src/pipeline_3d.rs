use std::cmp::Reverse;

use bevy_asset::Handle;
use bevy_core_pipeline::prelude::Camera3d;
use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_pbr::*;
use bevy_render::{
    mesh::Mesh,
    prelude::Camera,
    render_phase::{CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem},
    render_resource::Shader,
    view::{ExtractedView, ViewTarget},
    Extract,
};
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    texture::BevyDefault,
    view::Msaa,
};
use bevy_utils::FloatOrd;

use crate::{GizmoConfig, GizmoMesh, LINE_SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct GizmoPipeline3d {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for GizmoPipeline3d {
    fn from_world(render_world: &mut World) -> Self {
        GizmoPipeline3d {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            shader: LINE_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for GizmoPipeline3d {
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
        let (label, blend, depth_write_enabled);
        if key.contains(MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA) {
            label = "transparent_gizmo_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha
            // blended but their depth is not written to the depth buffer.
            depth_write_enabled = false;
        } else {
            label = "opaque_gizmo_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer
            // will replace the current fragment value in the output and the depth is
            // written to the depth buffer.
            depth_write_enabled = true;
        }
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
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: Some(bind_group_layout),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
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

pub(crate) type DrawGizmoLines = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

pub struct GizmoLine3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for GizmoLine3d {
    // NOTE: Values increase towards the camera. Front-to-back ordering for opaque means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }
}

impl CachedRenderPipelinePhaseItem for GizmoLine3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_gizmo_line_3d_camera_phase(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<GizmoLine3d>::default());
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_gizmos_3d(
    draw_functions: Res<DrawFunctions<GizmoLine3d>>,
    pipeline: Res<GizmoPipeline3d>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GizmoPipeline3d>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    mesh_handles: Query<(Entity, &Handle<Mesh>), With<GizmoMesh>>,
    config: Res<GizmoConfig>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<GizmoLine3d>)>,
) {
    let draw_function = draw_functions.read().id::<DrawGizmoLines>();
    let key = MeshPipelineKey::from_msaa_samples(msaa.samples());
    for (view, mut phase) in &mut views {
        let key = key | MeshPipelineKey::from_hdr(view.hdr);
        for (entity, mesh_handle) in &mesh_handles {
            let Some(mesh) = render_meshes.get(mesh_handle) else {
                continue;
            };

            let key = key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
            let pipeline = pipelines
                .specialize(
                    &pipeline_cache,
                    &pipeline,
                    (!config.on_top, key),
                    &mesh.layout,
                )
                .unwrap();
            phase.add(GizmoLine3d {
                entity,
                pipeline,
                draw_function,
                distance: 0.,
            });
        }
    }
}
