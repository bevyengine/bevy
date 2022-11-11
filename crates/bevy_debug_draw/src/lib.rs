use bevy_app::{CoreStage, Plugin};
use bevy_asset::{load_internal_asset, Assets, HandleUntyped};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    prelude::{Mesh, SpatialBundle},
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, SpecializedMeshPipelines},
    Extract, RenderApp, RenderStage,
};

#[cfg(feature = "bevy_pbr")]
use bevy_pbr::{NotShadowCaster, NotShadowReceiver};
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::Mesh2dHandle;

pub mod debug_draw;

#[cfg(feature = "bevy_sprite")]
pub mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
pub mod pipeline_3d;

use crate::debug_draw::DebugDraw;

/// The `bevy_debug_draw` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{debug_draw::DebugDraw, DebugDrawConfig, DebugDrawPlugin};
}

pub const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

pub struct DebugDrawPlugin;

impl Plugin for DebugDrawPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, SHADER_HANDLE, "debuglines.wgsl", Shader::from_wgsl);

        app.init_resource::<DebugDraw>()
            .init_resource::<DebugDrawConfig>()
            .add_system_to_stage(CoreStage::Last, update)
            .sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Extract, extract);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            app.sub_app_mut(RenderApp)
                .add_render_command::<Transparent2d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            app.sub_app_mut(RenderApp)
                .add_render_command::<Opaque3d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct DebugDrawConfig {
    /// Whether debug drawing should be shown.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Whether debug drawing should ignore depth and draw on top of everything else.
    /// 
    /// This setting only affects 3D. In 2D, debug drawing is always on top.
    ///
    /// Defaults to `true`.
    pub always_on_top: bool,
}

impl Default for DebugDrawConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            always_on_top: true,
        }
    }
}

#[derive(Component)]
struct DebugDrawMesh;

fn update(
    config: Res<DebugDrawConfig>,
    mut debug_draw: ResMut<DebugDraw>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    if let Some(mesh) = debug_draw
        .mesh_handle
        .as_ref()
        .and_then(|handle| meshes.get_mut(handle))
    {
        if config.enabled {
            debug_draw.update_mesh(mesh);
        } else {
            debug_draw.clear();
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION);
            mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
        }
    } else if config.enabled {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        debug_draw.update_mesh(&mut mesh);
        let mesh_handle = meshes.add(mesh);
        commands.spawn((
            SpatialBundle::VISIBLE_IDENTITY,
            DebugDrawMesh,
            #[cfg(feature = "bevy_pbr")]
            (mesh_handle.clone_weak(), NotShadowCaster, NotShadowReceiver),
            #[cfg(feature = "bevy_sprite")]
            Mesh2dHandle(mesh_handle.clone_weak()),
        ));
        debug_draw.mesh_handle = Some(mesh_handle);
    } else {
        debug_draw.clear();
    }
}

/// Move the [`DebugDrawMesh`] marker Component and the [`DebugDrawConfig`] Resource to the render context.
fn extract(
    mut commands: Commands,
    query: Extract<Query<Entity, With<DebugDrawMesh>>>,
    config: Extract<Res<DebugDrawConfig>>,
) {
    for entity in &query {
        commands.get_or_spawn(entity).insert(DebugDrawMesh);
    }

    if config.is_changed() {
        commands.insert_resource(**config);
    }
}
