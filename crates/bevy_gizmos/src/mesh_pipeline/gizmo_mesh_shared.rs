//! Shared setup for 2D and 3D

use bevy_asset::AssetId;
use bevy_ecs::{
    entity::Entity,
    system::{lifetimeless::SRes, Res, ResMut, Resource, SystemParamItem, SystemState},
    world::{FromWorld, World},
};
use bevy_render::{
    mesh::{GpuBufferInfo, Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_phase::*,
    render_resource::*,
    renderer::RenderDevice,
};

use crate::mesh_pipeline::{GizmoUniform, RenderGizmoInstances, GIZMO_MESH_SHADER_HANDLE};

#[derive(Resource, Clone)]
pub(super) struct GizmoMeshShared {
    pub gizmo_layout: BindGroupLayout,
}

impl GizmoMeshShared {
    pub fn get_descriptor(
        &self,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        if layout.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        if layout.contains(Mesh::ATTRIBUTE_NORMAL) {
            shader_defs.push("VERTEX_NORMALS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
        }

        if layout.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(4));
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let mut push_constant_ranges = Vec::with_capacity(1);
        if cfg!(all(feature = "webgl", target_arch = "wasm32")) {
            push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..4,
            });
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: GIZMO_MESH_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: GIZMO_MESH_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![],
            }),
            layout: vec![self.gizmo_layout.clone()],
            push_constant_ranges,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            label: None,
        })
    }
}

impl FromWorld for GizmoMeshShared {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<Res<RenderDevice>> = SystemState::new(world);
        let render_device = system_state.get_mut(world);

        let gizmo_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[GpuArrayBuffer::<GizmoUniform>::binding_layout(
                0,
                ShaderStages::VERTEX_FRAGMENT,
                &render_device,
            )],
            label: Some("gizmo_layout"),
        });

        GizmoMeshShared { gizmo_layout }
    }
}

pub(super) fn get_batch_data(
    gizmo_instances: &RenderGizmoInstances,
    entity: &Entity,
) -> (GizmoUniform, Option<AssetId<Mesh>>) {
    let gizmo_instance = gizmo_instances.get(entity).unwrap();

    let (inverse_transpose_model_a, inverse_transpose_model_b) =
        gizmo_instance.transform.inverse_transpose_3x3();
    (
        GizmoUniform {
            color: gizmo_instance.color,
            transform: gizmo_instance.transform.to_transpose(),
            inverse_transpose_model_a,
            inverse_transpose_model_b,
        },
        Some(gizmo_instance.mesh_asset_id),
    )
}

/// Bind groups for meshes currently loaded.
#[derive(Resource, Default)]
pub struct GizmoBindgroup {
    model: Option<BindGroup>,
}

pub(super) fn prepare_gizmo_bind_group(
    mut group: ResMut<GizmoBindgroup>,
    gizmo_pipeline: Res<GizmoMeshShared>,
    render_device: Res<RenderDevice>,
    gizmo_uniforms: Res<GpuArrayBuffer<GizmoUniform>>,
) {
    group.model = None;
    let Some(model) = gizmo_uniforms.binding() else {
        return;
    };

    group.model = Some(render_device.create_bind_group(
        "gizmo_uniform_bind_group",
        &gizmo_pipeline.gizmo_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: model.clone(),
        }],
    ));
}

pub struct SetGizmoBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGizmoBindGroup<I> {
    type Param = SRes<GizmoBindgroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: (),
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_group = bind_group.into_inner();
        let Some(bind_group) = bind_group.model.as_ref() else {
            return RenderCommandResult::Failure;
        };

        let mut dynamic_offsets: [u32; 1] = Default::default();
        let mut offset_count = 0;
        if let Some(dynamic_offset) = item.dynamic_offset() {
            dynamic_offsets[offset_count] = dynamic_offset.get();
            offset_count += 1;
        }
        pass.set_bind_group(I, bind_group, &dynamic_offsets[0..offset_count]);

        RenderCommandResult::Success
    }
}

pub(super) struct DrawGizmo;
impl<P: PhaseItem> RenderCommand<P> for DrawGizmo {
    type Param = (SRes<RenderAssets<Mesh>>, SRes<RenderGizmoInstances>);
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: (),
        (meshes, gizmo_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let gizmo_instances = gizmo_instances.into_inner();

        let Some(gizmo_instance) = gizmo_instances.get(&item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.get(gizmo_instance.mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

        let batch_range = item.batch_range();
        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            &(batch_range.start as i32).to_le_bytes(),
        );
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, batch_range.clone());
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, batch_range.clone());
            }
        }
        RenderCommandResult::Success
    }
}
