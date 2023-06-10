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

use bevy_app::{Last, Plugin, Update};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    query::Without,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::Mat4;
use bevy_reflect::{
    std_traits::ReflectDefault, FromReflect, Reflect, ReflectFromReflect, TypeUuid,
};
use bevy_render::{
    color::Color,
    mesh::Mesh,
    primitives::Aabb,
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, SpecializedMeshPipelines},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};

#[cfg(feature = "bevy_pbr")]
use bevy_pbr::MeshUniform;
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::{Mesh2dHandle, Mesh2dUniform};

pub mod gizmos;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use gizmos::{GizmoStorage, Gizmos};

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{gizmos::Gizmos, AabbGizmo, AabbGizmoConfig, GizmoConfig};
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
            .add_systems(Last, update_gizmo_meshes)
            .add_systems(
                Update,
                (
                    draw_aabbs,
                    draw_all_aabbs.run_if(|config: Res<GizmoConfig>| config.aabb.draw_all),
                ),
            );

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_systems(ExtractSchedule, extract_gizmo_data);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            render_app
                .add_render_command::<Transparent2d, DrawGizmoLines>()
                .init_resource::<SpecializedMeshPipelines<GizmoLinePipeline>>()
                .add_systems(Render, queue_gizmos_2d.in_set(RenderSet::Queue));
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            render_app
                .add_render_command::<Opaque3d, DrawGizmoLines>()
                .init_resource::<SpecializedMeshPipelines<GizmoPipeline>>()
                .add_systems(Render, queue_gizmos_3d.in_set(RenderSet::Queue));
        }
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        #[cfg(feature = "bevy_sprite")]
        {
            use pipeline_2d::*;

            render_app.init_resource::<GizmoLinePipeline>();
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use pipeline_3d::*;

            render_app.init_resource::<GizmoPipeline>();
        }
    }
}

/// A [`Resource`] that stores configuration for gizmos.
#[derive(Resource, Clone)]
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
    /// Configuration for the [`AabbGizmo`].
    pub aabb: AabbGizmoConfig,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_top: false,
            aabb: Default::default(),
        }
    }
}

/// Configuration for drawing the [`Aabb`] component on entities.
#[derive(Clone, Default)]
pub struct AabbGizmoConfig {
    /// Draws all bounding boxes in the scene when set to `true`.
    ///
    /// To draw a specific entity's bounding box, you can add the [`AabbGizmo`] component.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// The default color for bounding box gizmos.
    ///
    /// A random color is chosen per box if `None`.
    ///
    /// Defaults to `None`.
    pub default_color: Option<Color>,
}

/// Add this [`Component`] to an entity to draw its [`Aabb`] component.
#[derive(Component, Reflect, FromReflect, Default, Debug)]
#[reflect(Component, FromReflect, Default)]
pub struct AabbGizmo {
    /// The color of the box.
    ///
    /// The default color from the [`GizmoConfig`] resource is used if `None`,
    pub color: Option<Color>,
}

fn draw_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform, &AabbGizmo)>,
    config: Res<GizmoConfig>,
    mut gizmos: Gizmos,
) {
    for (entity, &aabb, &transform, gizmo) in &query {
        let color = gizmo
            .color
            .or(config.aabb.default_color)
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.cuboid(aabb_transform(aabb, transform), color);
    }
}

fn draw_all_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform), Without<AabbGizmo>>,
    config: Res<GizmoConfig>,
    mut gizmos: Gizmos,
) {
    for (entity, &aabb, &transform) in &query {
        let color = config
            .aabb
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.cuboid(aabb_transform(aabb, transform), color);
    }
}

fn color_from_entity(entity: Entity) -> Color {
    let hue = entity.to_bits() as f32 * 100_000. % 360.;
    Color::hsl(hue, 1., 0.5)
}

fn aabb_transform(aabb: Aabb, transform: GlobalTransform) -> GlobalTransform {
    transform
        * GlobalTransform::from(
            Transform::from_translation(aabb.center.into())
                .with_scale((aabb.half_extents * 2.).into()),
        )
}

#[derive(Resource)]
struct MeshHandles {
    list: Option<Handle<Mesh>>,
    strip: Option<Handle<Mesh>>,
}

impl FromWorld for MeshHandles {
    fn from_world(_world: &mut World) -> Self {
        MeshHandles {
            list: None,
            strip: None,
        }
    }
}

#[derive(Component)]
struct GizmoMesh;

fn update_gizmo_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    mut handles: ResMut<MeshHandles>,
    mut storage: ResMut<GizmoStorage>,
) {
    if storage.list_positions.is_empty() {
        handles.list = None;
    } else if let Some(handle) = handles.list.as_ref() {
        let list_mesh = meshes.get_mut(handle).unwrap();

        let positions = mem::take(&mut storage.list_positions);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.list_colors);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    } else {
        let mut list_mesh = Mesh::new(PrimitiveTopology::LineList);

        let positions = mem::take(&mut storage.list_positions);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.list_colors);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        handles.list = Some(meshes.add(list_mesh));
    }

    if storage.strip_positions.is_empty() {
        handles.strip = None;
    } else if let Some(handle) = handles.strip.as_ref() {
        let strip_mesh = meshes.get_mut(handle).unwrap();

        let positions = mem::take(&mut storage.strip_positions);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.strip_colors);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    } else {
        let mut strip_mesh = Mesh::new(PrimitiveTopology::LineStrip);

        let positions = mem::take(&mut storage.strip_positions);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.strip_colors);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        handles.strip = Some(meshes.add(strip_mesh));
    }
}

fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<MeshHandles>>,
    config: Extract<Res<GizmoConfig>>,
) {
    if config.is_changed() {
        commands.insert_resource(config.clone());
    }

    if !config.enabled {
        return;
    }

    let transform = Mat4::IDENTITY;
    let inverse_transpose_model = transform.inverse().transpose();
    commands.spawn_batch(
        [handles.list.clone(), handles.strip.clone()]
            .into_iter()
            .flatten()
            .map(move |handle| {
                (
                    GizmoMesh,
                    #[cfg(feature = "bevy_pbr")]
                    (
                        handle.clone_weak(),
                        MeshUniform {
                            flags: 0,
                            transform,
                            previous_transform: transform,
                            inverse_transpose_model,
                        },
                    ),
                    #[cfg(feature = "bevy_sprite")]
                    (
                        Mesh2dHandle(handle),
                        Mesh2dUniform {
                            flags: 0,
                            transform,
                            inverse_transpose_model,
                        },
                    ),
                )
            }),
    );
}
