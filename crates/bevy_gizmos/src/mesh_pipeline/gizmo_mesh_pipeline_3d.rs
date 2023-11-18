use bevy_app::{App, Plugin};
use bevy_asset::AssetId;
use bevy_core_pipeline::{
    core_3d::{Opaque3d, CORE_3D_DEPTH_FORMAT},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    schedule::IntoSystemConfigs,
    system::{lifetimeless::SRes, Query, Res, ResMut, Resource, SystemParamItem, SystemState},
    world::{FromWorld, World},
};
use bevy_log::error;
use bevy_pbr::*;
use bevy_render::{
    batching::{batch_and_prepare_render_phase, write_batched_instance_buffer, GetBatchData},
    camera::Projection,
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::{prepare_assets, RenderAssets},
    render_phase::*,
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, ViewTarget, VisibleEntities},
    Render, RenderApp, RenderSet,
};

use crate::mesh_pipeline::{
    gizmo_mesh_shared::{get_batch_data, DrawGizmo, GizmoMeshShared, SetGizmoBindGroup},
    GizmoUniform, Immediate, RenderGizmoInstances,
};

pub struct GizmoMesh3dPlugin;

impl Plugin for GizmoMesh3dPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SpecializedMeshPipelines<GizmoMesh3dPipeline>>()
                .add_render_command::<Opaque3d, DrawGizmo3d>()
                .add_systems(
                    Render,
                    (
                        queue_gizmos_3d
                            .in_set(RenderSet::QueueMeshes)
                            .after(prepare_assets::<Mesh>),
                        batch_and_prepare_render_phase::<Opaque3d, GizmoMesh3dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<GizmoMesh3dPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<GizmoMesh3dPipeline>();
    }
}

#[derive(Component)]
pub(super) struct Gizmo3d;

#[derive(Resource, Clone)]
struct GizmoMesh3dPipeline {
    pub gizmo_pipeline: GizmoMeshShared,
    view_layouts: [MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT],
}

impl GizmoMesh3dPipeline {
    pub fn get_view_layout(&self, layout_key: MeshPipelineViewLayoutKey) -> &BindGroupLayout {
        let index = layout_key.bits() as usize;
        let layout = &self.view_layouts[index];
        &layout.bind_group_layout
    }
}

impl FromWorld for GizmoMesh3dPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<Res<RenderDevice>> = SystemState::new(world);
        let render_device = system_state.get_mut(world);

        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);

        let view_layouts =
            generate_view_layouts(&render_device, clustered_forward_buffer_binding_type);

        GizmoMesh3dPipeline {
            gizmo_pipeline: world.resource::<GizmoMeshShared>().clone(),
            view_layouts,
        }
    }
}

impl GetBatchData for GizmoMesh3dPipeline {
    type Param = SRes<RenderGizmoInstances>;
    type Query = Entity;
    type QueryFilter = With<Gizmo3d>;
    type CompareData = AssetId<Mesh>;
    type BufferData = GizmoUniform;

    fn get_batch_data(
        gizmo_instances: &SystemParamItem<Self::Param>,
        entity: &QueryItem<Self::Query>,
    ) -> (Self::BufferData, Option<Self::CompareData>) {
        get_batch_data(gizmo_instances, entity)
    }
}

impl SpecializedMeshPipeline for GizmoMesh3dPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();

        shader_defs.push("3D".into());

        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());

        if key.msaa_samples() > 1 {
            shader_defs.push("MULTISAMPLED".into());
        };

        let view_projection = key.intersection(MeshPipelineKey::VIEW_PROJECTION_RESERVED_BITS);
        if view_projection == MeshPipelineKey::VIEW_PROJECTION_NONSTANDARD {
            shader_defs.push("VIEW_PROJECTION_NONSTANDARD".into());
        } else if view_projection == MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE {
            shader_defs.push("VIEW_PROJECTION_PERSPECTIVE".into());
        } else if view_projection == MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC {
            shader_defs.push("VIEW_PROJECTION_ORTHOGRAPHIC".into());
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        shader_defs.push("WEBGL2".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED ".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(MeshPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let mut descriptor = self.gizmo_pipeline.get_descriptor(layout)?;

        descriptor.vertex.shader_defs.extend(shader_defs.clone());

        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader_defs.extend(shader_defs);

        let format = if key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        fragment.targets.push(Some(ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        }));

        descriptor
            .layout
            .insert(0, self.get_view_layout(key.into()).clone());
        descriptor.primitive.topology = key.primitive_topology();
        descriptor.multisample.count = key.msaa_samples();
        descriptor.depth_stencil = Some(DepthStencilState {
            format: CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        descriptor.label = Some("opaque_gizmo_mesh_pipeline".into());

        Ok(descriptor)
    }
}

type DrawGizmo3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetGizmoBindGroup<1>,
    DrawGizmo,
);

fn queue_gizmos_3d(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    pipeline: Res<GizmoMesh3dPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GizmoMesh3dPipeline>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    mut render_mesh_instances: ResMut<RenderGizmoInstances>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&Projection>,
        &mut RenderPhase<Opaque3d>,
    )>,
    immediate: Query<Entity, With<Immediate>>,
) {
    for (view, visible_entities, tonemapping, dither, projection, mut opaque_phase) in &mut views {
        let draw_opaque_pbr = opaque_draw_functions.read().id::<DrawGizmo3d>();

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
            };
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }
        let rangefinder = view.rangefinder3d();

        let visibile = visible_entities.entities.iter();
        // Mesh gizmos created via the immediate mode API won't be in the views VisibleEntities but may be visible.
        for entity in visibile.copied().chain(immediate.iter()) {
            let Some(gizmo_instance) = render_mesh_instances.get_mut(&entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(gizmo_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = view_key;

            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);

            let pipeline_id =
                pipelines.specialize(&pipeline_cache, &pipeline, mesh_key, &mesh.layout);
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            let distance = rangefinder.distance_translation(&gizmo_instance.transform.translation);
            // TODO: + material.properties.depth_bias;
            opaque_phase.add(Opaque3d {
                entity,
                draw_function: draw_opaque_pbr,
                pipeline: pipeline_id,
                distance,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}
