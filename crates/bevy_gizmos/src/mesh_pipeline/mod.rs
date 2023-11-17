use std::cell::Cell;

use bevy_app::{Plugin, First};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    schedule::IntoSystemConfigs,
    system::{Commands, Local, Query, Res, ResMut, Resource},
};
use bevy_math::{Affine3, Vec4};
use bevy_render::{
    color::Color,
    mesh::Mesh,
    render_resource::{GpuArrayBuffer, Shader, ShaderDefVal, ShaderType},
    renderer::RenderDevice,
    view::{InheritedVisibility, ViewVisibility, Visibility},
    Extract, ExtractSchedule, RenderApp, RenderSet, Render,
};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_utils::EntityHashMap;
use thread_local::ThreadLocal;

use crate::{gizmos::GizmoStorage, mesh_pipeline::gizmo_mesh_shared::{GizmoMeshShared, GizmoBindgroup, prepare_gizmo_bind_group}};

#[cfg(feature = "bevy_sprite")]
mod gizmo_mesh_pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod gizmo_mesh_pipeline_3d;

mod gizmo_mesh_shared;

const GIZMO_MESH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(7012430448968376275);

pub struct GizmoMeshPlugin;

impl Plugin for GizmoMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderGizmoInstances>()
            .init_resource::<GizmoBindgroup>()
            .add_systems(
                ExtractSchedule,
                (extract_gizmos_meshes, extract_immediate_gizmo_meshes).chain(),
            )
            .add_systems(
                Render,
                prepare_gizmo_bind_group.in_set(RenderSet::PrepareBindGroups),
            );

        
        #[cfg(feature = "bevy_sprite")]
        app.add_plugins(gizmo_mesh_pipeline_2d::GizmoMesh2dPlugin);
        #[cfg(feature = "bevy_pbr")]
        app.add_plugins(gizmo_mesh_pipeline_3d::GizmoMesh3dPlugin);

        app.add_systems(First, clear_immediate_mode_meshes);
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let mut gizmo_bindings_shader_defs = Vec::new();

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            if let Some(per_object_buffer_batch_size) = GpuArrayBuffer::<GizmoUniform>::batch_size(
                render_app.world.resource::<RenderDevice>(),
            ) {
                gizmo_bindings_shader_defs.push(ShaderDefVal::UInt(
                    "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                    per_object_buffer_batch_size,
                ));
            }

            render_app.insert_resource(GpuArrayBuffer::<GizmoUniform>::new(
                render_app.world.resource::<RenderDevice>(),
            ));

            render_app.init_resource::<GizmoMeshShared>();
        }

        // Load the shader here as it depends on runtime information about
        // whether storage buffers are supported, or the maximum uniform buffer binding size.
        load_internal_asset!(
            app,
            GIZMO_MESH_SHADER_HANDLE,
            "gizmo_mesh.wgsl",
            Shader::from_wgsl_with_defs,
            gizmo_bindings_shader_defs
        );
    }
}

#[derive(ShaderType, Clone)]
struct GizmoUniform {
    // Affine 4x3 matrices transposed to 3x4
    pub transform: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub inverse_transpose_model_a: [Vec4; 2],
    pub inverse_transpose_model_b: f32,
    pub color: Vec4,
}

struct RenderGizmoInstance {
    pub transform: Affine3,
    pub mesh_asset_id: AssetId<Mesh>,
    pub color: Vec4,
}

#[derive(Default, Resource, Deref, DerefMut)]
struct RenderGizmoInstances(EntityHashMap<Entity, RenderGizmoInstance>);

/// Controls the way the gizmo will be rendered.
#[derive(Component, Default, Clone, Debug)]
pub struct GizmoStyle {
    /// Color
    pub color: Color,
}

/// 
#[derive(Bundle, Default, Clone, Debug)]
pub struct GizmoMeshBundle {
    /// Controls the look of the gizmo.
    pub style: GizmoStyle,
    /// The mesh to be rendered.
    pub mesh: Handle<Mesh>,
    /// The transform of the entity.
    pub transform: Transform,
    /// The computed global transform of the entity.
    pub global_transform: GlobalTransform,
    /// The visibility of the entity.
    pub visibility: Visibility,
    /// The inherited visibility of the entity.
    pub inherited_visibility: InheritedVisibility,
    /// The view visibility of the entity.
    pub view_visibility: ViewVisibility,
}

/// Extracts gizmo meshes from main world .
fn extract_gizmos_meshes(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut render_mesh_instances: ResMut<RenderGizmoInstances>,
    mut thread_local_queues: Local<ThreadLocal<Cell<Vec<(Entity, RenderGizmoInstance)>>>>,
    gizmos_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            &Handle<Mesh>,
            &GizmoStyle,
        )>,
    >,
) {
    gizmos_query
        .par_iter()
        .for_each(|(entity, view_visibility, transform, handle, gizmo)| {
            if !view_visibility.get() {
                return;
            }
            let transform = transform.affine();
            let tls = thread_local_queues.get_or_default();
            let mut queue = tls.take();
            queue.push((
                entity,
                RenderGizmoInstance {
                    mesh_asset_id: handle.id(),
                    transform: (&transform).into(),
                    color: gizmo.color.as_linear_rgba_f32().into(),
                },
            ));
            tls.set(queue);
        });

    render_mesh_instances.clear();
    let mut entities = Vec::with_capacity(*previous_len);
    for queue in thread_local_queues.iter_mut() {
        // FIXME: Remove this - it is just a workaround to enable rendering to work as
        // render commands require an entity to exist at the moment.

        entities.extend(queue.get_mut().iter().map(|(e, _)| {
            (
                *e,
                (
                    #[cfg(feature = "bevy_sprite")]
                    gizmo_mesh_pipeline_2d::Gizmo2d,
                    #[cfg(feature = "bevy_pbr")]
                    gizmo_mesh_pipeline_3d::Gizmo3d,
                ),
            )
        }));
        render_mesh_instances.extend(queue.get_mut().drain(..));
    }
    *previous_len = entities.len();
    commands.insert_or_spawn_batch(entities);
}

/// Marker component for gizmo meshes created in the render world via the immediate mode API.
#[derive(Component)]
struct Immediate;

/// Extracts gizmo meshes from the immediate mode API.
fn extract_immediate_gizmo_meshes(
    mut commands: Commands,
    gizmo_storage: Extract<Res<GizmoStorage>>,
    mut render_gizmo_instances: ResMut<RenderGizmoInstances>,
) {
    for (mesh_id, transform, color) in &gizmo_storage.meshes {
        let transform = GlobalTransform::from(*transform);
        let entity = commands
            .spawn((
                #[cfg(feature = "bevy_pbr")]
                (gizmo_mesh_pipeline_3d::Gizmo3d, Handle::Weak(*mesh_id)),
                #[cfg(feature = "bevy_sprite")]
                (
                    gizmo_mesh_pipeline_2d::Gizmo2d,
                    bevy_sprite::Mesh2dHandle(Handle::Weak(*mesh_id)),
                ),
                Immediate,
            ))
            .id();

        render_gizmo_instances.insert(
            entity,
            RenderGizmoInstance {
                color: color.as_linear_rgba_f32().into(),
                transform: (&transform.affine()).into(),
                mesh_asset_id: mesh_id.clone(),
            },
        );
    }
}

fn clear_immediate_mode_meshes(mut storage: ResMut<GizmoStorage>) {
    storage.meshes.clear();
}
