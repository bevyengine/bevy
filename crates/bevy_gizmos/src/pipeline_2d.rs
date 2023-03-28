use bevy_asset::Handle;
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::{
    prelude::Entity,
    query::{ROQueryItem, With},
    system::{
        lifetimeless::{Read, SRes},
        Commands, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_phase::{
        DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
        TrackedRenderPass,
    },
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, ViewTarget},
};
use bevy_sprite::*;
use bevy_utils::FloatOrd;

use crate::{GizmoUniform, GIZMO_SHADER_HANDLE};

#[derive(Resource)]
pub(crate) struct GizmoPipeline {
    mesh_pipeline: Mesh2dPipeline,
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
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            gizmo_layout,
            shader: GIZMO_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for GizmoPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];

        if layout.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(1));
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let format = if key.contains(Mesh2dPipelineKey::HDR) {
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
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![
                self.mesh_pipeline.view_layout.clone(),
                self.mesh_pipeline.mesh_layout.clone(),
                self.gizmo_layout.clone(),
            ],
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
    SetGizmoBindGroup,
    DrawMesh2d,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_gizmos_2d(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<GizmoPipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedMeshPipelines<GizmoPipeline>>,
    gpu_meshes: Res<RenderAssets<Mesh>>,
    msaa: Res<Msaa>,
    mesh_handles: Query<(Entity, &Mesh2dHandle), With<GizmoUniform>>,
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

#[derive(Resource)]
pub(crate) struct GizmoBindGroup(BindGroup);

pub(crate) fn queue_gizmo_bind_group_2d(
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
