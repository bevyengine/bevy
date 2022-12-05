use std::iter;

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::Component,
    system::{Commands, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    mesh::Mesh,
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, SpecializedMeshPipelines},
    Extract, RenderApp, RenderStage,
};

#[cfg(feature = "bevy_sprite")]
use bevy_sprite::Mesh2dHandle;

use once_cell::sync::Lazy;

pub mod gizmos;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use crate::gizmos::GizmoDraw;

/// The `bevy_debug_draw` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{GizmoConfig, GIZMO};
}

const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

pub struct DebugDrawPlugin;

impl Plugin for DebugDrawPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);

        app.init_resource::<MeshHandles>()
            .init_resource::<GizmoConfig>()
            .add_system_to_stage(CoreStage::Last, system);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_system_to_stage(RenderStage::Extract, extract);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            render_app
                .add_render_command::<Transparent2d, DrawGizmoLines>()
                .init_resource::<GizmoLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            render_app
                .add_render_command::<Opaque3d, DrawGizmoLines>()
                .init_resource::<GizmoLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct GizmoConfig {
    /// Whether debug drawing should be shown.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Whether debug drawing should ignore depth and draw on top of everything else.
    ///
    /// This setting only affects 3D. In 2D, debug drawing is always on top.
    ///
    /// Defaults to `true`.
    pub on_top: bool,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_top: true,
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
struct GizmoDrawMesh;

type PositionItem = [f32; 3];
type ColorItem = [f32; 4];

enum SendItem {
    Single(([PositionItem; 2], [ColorItem; 2])),
    List((Vec<PositionItem>, Vec<ColorItem>)),
    Strip((Vec<PositionItem>, Vec<ColorItem>)),
}

pub static GIZMO: Lazy<GizmoDraw> = Lazy::new(GizmoDraw::new);

fn system(mut meshes: ResMut<Assets<Mesh>>, handles: Res<MeshHandles>) {
    let mut list_positions = Vec::new();
    let mut list_colors = Vec::new();
    let mut strip_positions = Vec::new();
    let mut strip_colors = Vec::new();

    for item in GIZMO.receiver.try_iter() {
        match item {
            SendItem::Single((positions, colors)) => {
                list_positions.extend(positions);
                list_colors.extend(colors);
            }
            SendItem::List((positions, colors)) => {
                list_positions.extend(positions);
                list_colors.extend(colors);
            }
            SendItem::Strip((positions, colors)) => {
                strip_positions.extend(positions.into_iter().chain(iter::once([f32::NAN; 3])));
                strip_colors.extend(colors.into_iter().chain(iter::once([f32::NAN; 4])));
            }
        }
    }

    let list_mesh = meshes.get_mut(&handles.list).unwrap();

    list_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, list_positions);
    list_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, list_colors);

    let strip_mesh = meshes.get_mut(&handles.strip).unwrap();

    strip_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, strip_positions);
    strip_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, strip_colors);
}

fn extract(
    mut commands: Commands,
    handles: Extract<Res<MeshHandles>>,
    config: Extract<Res<GizmoConfig>>,
) {
    if config.is_changed() {
        commands.insert_resource(**config);
    }

    commands.spawn_batch([&handles.list, &handles.strip].map(|handle| {
        (
            GizmoDrawMesh,
            #[cfg(feature = "bevy_pbr")]
            handle.clone(),
            #[cfg(feature = "bevy_sprite")]
            Mesh2dHandle(handle.clone()),
        )
    }));
}
