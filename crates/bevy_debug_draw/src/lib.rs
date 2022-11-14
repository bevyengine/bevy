use std::mem;

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    component::Component,
    entity::Entity,
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
#[cfg(feature = "bevy_pbr")]
use bevy_render::view::NoFrustumCulling;
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::Mesh2dHandle;

pub mod debug_draw;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use crate::debug_draw::DebugDraw;

/// The `bevy_debug_draw` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{debug_draw::DebugDraw, DebugDrawConfig, DebugDrawPlugin};
}

const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

pub struct DebugDrawPlugin;

impl Plugin for DebugDrawPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, SHADER_HANDLE, "debuglines.wgsl", Shader::from_wgsl);

        app.init_resource::<DebugDraw>()
            .init_resource::<DebugDrawConfig>()
            .add_system_to_stage(CoreStage::Last, update);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_system_to_stage(RenderStage::Extract, extract);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            render_app
                .add_render_command::<Transparent2d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            render_app
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

#[derive(Component, Clone, Copy)]
struct DebugDrawMesh {
    topology: PrimitiveTopology,
}

fn update(
    config: Res<DebugDrawConfig>,
    mut debug_draw: ResMut<DebugDraw>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    let mut f = |handle: &mut Option<Handle<Mesh>>,
                 positions: &mut Vec<[f32; 3]>,
                 colors: &mut Vec<[f32; 4]>,
                 topology: PrimitiveTopology| {
        let mesh = handle.as_ref().and_then(|handle| meshes.get_mut(handle));
        match mesh {
            Some(mesh) => {
                if config.enabled {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mem::take(positions));
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, mem::take(colors));
                } else {
                    positions.clear();
                    colors.clear();
                    mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION);
                    mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
                }
            }
            None => {
                if config.enabled {
                    let mut mesh = Mesh::new(topology);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mem::take(positions));
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, mem::take(colors));
                    let mesh_handle = meshes.add(mesh);
                    commands.spawn((
                        SpatialBundle::VISIBLE_IDENTITY,
                        DebugDrawMesh { topology },
                        #[cfg(feature = "bevy_pbr")]
                        (
                            mesh_handle.clone_weak(),
                            NotShadowCaster,
                            NotShadowReceiver,
                            NoFrustumCulling,
                        ),
                        #[cfg(feature = "bevy_sprite")]
                        Mesh2dHandle(mesh_handle.clone_weak()),
                    ));
                    *handle = Some(mesh_handle);
                } else {
                    positions.clear();
                    colors.clear();
                }
            }
        }
    };

    let DebugDraw {
        list_mesh_handle,
        list_positions,
        list_colors,
        ..
    } = &mut *debug_draw;

    f(
        list_mesh_handle,
        list_positions,
        list_colors,
        PrimitiveTopology::LineList,
    );

    let DebugDraw {
        strip_mesh_handle,
        strip_positions,
        strip_colors,
        ..
    } = &mut *debug_draw;

    f(
        strip_mesh_handle,
        strip_positions,
        strip_colors,
        PrimitiveTopology::LineStrip,
    );
}

/// Move the [`DebugDrawMesh`] marker Component and the [`DebugDrawConfig`] Resource to the render context.
fn extract(
    mut commands: Commands,
    query: Extract<Query<(Entity, &DebugDrawMesh)>>,
    config: Extract<Res<DebugDrawConfig>>,
) {
    for (entity, debug_draw) in &query {
        commands.get_or_spawn(entity).insert(*debug_draw);
    }

    if config.is_changed() {
        commands.insert_resource(**config);
    }
}
