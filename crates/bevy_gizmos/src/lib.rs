#![allow(clippy::type_complexity)]
#![warn(missing_docs)]

//! This crate adds an immediate mode drawing api to Bevy for visual debugging.
//!
//! # Example
//! ```
//! # use bevy_gizmos::prelude::*;
//! # use bevy_render::prelude::*;
//! # use bevy_math::prelude::*;
//! fn system(mut gizmos: Gizmos) {
//!     gizmos.line(Vec3::ZERO, Vec3::X, Color::GREEN);
//! }
//! # bevy_ecs::system::assert_is_system(system);
//! ```
//!
//! See the documentation on [`Gizmos`](crate::gizmos::Gizmos) for more examples.

use std::mem;

use bevy_app::{Last, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::{Component, DetectChanges},
    schedule::IntoSystemConfigs,
    system::{Commands, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::Mat4;
use bevy_reflect::TypeUuid;
use bevy_render::{
    mesh::Mesh,
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, SpecializedMeshPipelines},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};

#[cfg(feature = "bevy_pbr")]
use bevy_pbr::MeshUniform;
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::{Mesh2dHandle, Mesh2dUniform};

pub mod gizmos;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use crate::gizmos::GizmoStorage;

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{gizmos::Gizmos, GizmoConfig};
}

const LINE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, LINE_SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);

        app.init_resource::<MeshHandles>()
            .init_resource::<GizmoConfig>()
            .init_resource::<GizmoStorage>()
            .add_systems(Last, update_gizmo_meshes);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_systems(ExtractSchedule, extract_gizmo_data);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            render_app
                .add_render_command::<Transparent2d, DrawGizmoLines>()
                .init_resource::<GizmoLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoLinePipeline>>()
                .add_systems(Render, queue_gizmos_2d.in_set(RenderSet::Queue));
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            render_app
                .add_render_command::<Opaque3d, DrawGizmoLines>()
                .init_resource::<GizmoPipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoPipeline>>()
                .add_systems(Render, queue_gizmos_3d.in_set(RenderSet::Queue));
        }
    }
}

/// A [`Resource`] that stores configuration for gizmos.
#[derive(Resource, Clone, Copy)]
pub struct GizmoConfig {
    /// Set to `false` to stop drawing gizmos.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Draw gizmos on top of everything else, ignoring depth.
    ///
    /// This setting only affects 3D. In 2D, gizmos are always drawn on top.
    ///
    /// Defaults to `false`.
    pub on_top: bool,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_top: false,
        }
    }
}

#[derive(Resource)]
struct MeshHandles {
    list: Handle<Mesh>,
    strip: Handle<Mesh>,
}

impl FromWorld for MeshHandles {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();

        MeshHandles {
            list: meshes.add(Mesh::new(PrimitiveTopology::LineList)),
            strip: meshes.add(Mesh::new(PrimitiveTopology::LineStrip)),
        }
    }
}

#[derive(Component)]
struct GizmoMesh;

fn update_gizmo_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<MeshHandles>,
    mut storage: ResMut<GizmoStorage>,
) {
    let list_mesh = meshes.get_mut(&handles.list).unwrap();

    let positions = mem::take(&mut storage.list_positions);
    list_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    let colors = mem::take(&mut storage.list_colors);
    list_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

    let strip_mesh = meshes.get_mut(&handles.strip).unwrap();

    let positions = mem::take(&mut storage.strip_positions);
    strip_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    let colors = mem::take(&mut storage.strip_colors);
    strip_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<MeshHandles>>,
    config: Extract<Res<GizmoConfig>>,
) {
    if config.is_changed() {
        commands.insert_resource(**config);
    }

    if !config.enabled {
        return;
    }

    let transform = Mat4::IDENTITY;
    let inverse_transpose_model = transform.inverse().transpose();
    commands.spawn_batch([&handles.list, &handles.strip].map(|handle| {
        (
            GizmoMesh,
            #[cfg(feature = "bevy_pbr")]
            (
                handle.clone(),
                MeshUniform {
                    flags: 0,
                    transform,
                    previous_transform: transform,
                    inverse_transpose_model,
                },
            ),
            #[cfg(feature = "bevy_sprite")]
            (
                Mesh2dHandle(handle.clone()),
                Mesh2dUniform {
                    flags: 0,
                    transform,
                    inverse_transpose_model,
                },
            ),
        )
    }));
}
