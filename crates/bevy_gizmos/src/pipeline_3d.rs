use bevy_asset::Handle;
use bevy_core_pipeline::core_3d::Opaque3d;
use bevy_ecs::{
    entity::Entity,
    query::{ROQueryItem, With},
    system::{
        lifetimeless::{Read, SRes},
        Commands, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_pbr::*;
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    mesh::Mesh,
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::Shader,
    renderer::RenderDevice,
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

use crate::{GizmoConfig, GizmoUniform, GIZMO_SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct GizmoPipeline {
    mesh_pipeline: MeshPipeline,
    gizmo_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

impl FromWorld for GizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let gizmo_binding = BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(GizmoUniform::min_size()),
            },
            count: None,
        };

        let gizmo_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[gizmo_binding],
            label: Some("gizmo_layout"),
        });

        GizmoPipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            gizmo_layout,
            shader: GIZMO_SHADER_HANDLE.typed(),
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
        shader_defs.push("GIZMO_3D".into());
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

        let mut vertex_attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];

        if layout.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(1));
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let mut bind_group_layout = match key.msaa_samples() {
            1 => vec![self.mesh_pipeline.view_layout.clone()],
            _ => {
                shader_defs.push("MULTISAMPLED".into());
                vec![self.mesh_pipeline.view_layout_multisampled.clone()]
            }
        };
        bind_group_layout.push(self.mesh_pipeline.mesh_layout.clone());
        bind_group_layout.push(self.gizmo_layout.clone());

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
    SetGizmoBindGroup,
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
    mesh_handles: Query<(Entity, &Handle<Mesh>), With<GizmoUniform>>,
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

#[derive(Resource)]
pub(crate) struct GizmoBindGroup(BindGroup);

pub(crate) fn queue_gizmo_bind_group_3d(
    mut commands: Commands,
    gizmo_pipeline: Res<GizmoPipeline>,
    render_device: Res<RenderDevice>,
    gizmo_uniforms: Res<ComponentUniforms<GizmoUniform>>,
) {
    if let Some(gizmo_binding) = gizmo_uniforms.uniforms().binding() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: gizmo_binding.clone(),
            }],
            label: Some("gizmo_bind_group"),
            layout: &gizmo_pipeline.gizmo_layout,
        });

        commands.insert_resource(GizmoBindGroup(bind_group));
    }
}

pub(crate) struct SetGizmoBindGroup;
impl<P: PhaseItem> RenderCommand<P> for SetGizmoBindGroup {
    type Param = SRes<GizmoBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<GizmoUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        gizmo_index: ROQueryItem<'_, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(2, &bind_group.into_inner().0, &[gizmo_index.index()]);
        RenderCommandResult::Success
    }
}
