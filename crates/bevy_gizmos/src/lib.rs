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

use bevy_app::{First, Last, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::{Bundle, Component, DetectChanges},
    schedule::IntoSystemConfigs,
    storage::SparseSet,
    system::{Commands, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::{Mat4, Quat, Vec2, Vec3};
use bevy_pbr::NotShadowCaster;
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::UniformComponentPlugin,
    mesh::Mesh,
    prelude::{
        shape::{self, Circle, Icosphere},
        Color,
    },
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, ShaderType, SpecializedMeshPipelines},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};

#[cfg(feature = "bevy_pbr")]
use bevy_pbr::MeshUniform;
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::{Mesh2dHandle, Mesh2dUniform};
use bevy_transform::components::Transform;

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

const GIZMO_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, GIZMO_SHADER_HANDLE, "gizmo.wgsl", Shader::from_wgsl);

        app.add_plugin(UniformComponentPlugin::<GizmoUniform>::default())
            .init_resource::<MeshHandles>()
            .init_resource::<GizmoConfig>()
            .init_resource::<GizmoStorage>()
            .add_systems(Last, update_gizmo_meshes)
            .add_systems(First, |mut storage: ResMut<GizmoStorage>| {
                storage.meshes.clear();
                storage.discs.clear();
                storage.spheres.clear();
                storage.rectangles.clear();
                storage.cubes.clear();
            });

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_systems(ExtractSchedule, extract_gizmo_data);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            render_app
                .add_render_command::<Transparent2d, DrawGizmoLines>()
                .init_resource::<GizmoPipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoPipeline>>()
                .add_systems(
                    Render,
                    (queue_gizmos_2d, queue_gizmo_bind_group_2d).in_set(RenderSet::Queue),
                );
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            render_app
                .add_render_command::<Opaque3d, DrawGizmoLines>()
                .init_resource::<GizmoPipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoPipeline>>()
                .add_systems(
                    Render,
                    (queue_gizmos_3d, queue_gizmo_bind_group_3d).in_set(RenderSet::Queue),
                );
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

#[derive(Resource, Clone)]
struct MeshHandles {
    list: Handle<Mesh>,
    strip: Handle<Mesh>,
    discs: SparseSet<usize, Handle<Mesh>>,
    spheres: SparseSet<usize, Handle<Mesh>>,
    cube: Handle<Mesh>,
    rectangle: Handle<Mesh>,
}

impl FromWorld for MeshHandles {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();

        MeshHandles {
            list: meshes.add(Mesh::new(PrimitiveTopology::LineList)),
            strip: meshes.add(Mesh::new(PrimitiveTopology::LineStrip)),
            discs: SparseSet::new(),
            spheres: SparseSet::new(),
            cube: meshes.add(shape::Box::new(1., 1., 1.).into()),
            rectangle: meshes.add(shape::Quad::new(Vec2::splat(1.)).into()),
        }
    }
}

#[derive(Component, ShaderType, Clone)]
struct GizmoUniform {
    color: Color,
}

fn update_gizmo_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    mut handles: ResMut<MeshHandles>,
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

    for (_position, _normal, _radius, _color, segments) in storage.discs.iter().copied() {
        let mesh = Circle {
            radius: 1.,
            vertices: segments,
        }
        .try_into()
        .unwrap();

        handles
            .discs
            .get_or_insert_with(segments, || meshes.add(mesh));
    }

    for (_position, _radius, _color, subdivisions) in storage.spheres.iter().copied() {
        let mesh = Icosphere {
            radius: 1.,
            subdivisions,
        }
        .try_into()
        .unwrap();

        handles
            .spheres
            .get_or_insert_with(subdivisions, || meshes.add(mesh));
    }
}

fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<MeshHandles>>,
    config: Extract<Res<GizmoConfig>>,
    storage: Extract<Res<GizmoStorage>>,
) {
    if config.is_changed() {
        commands.insert_resource(**config);
    }

    if !config.enabled {
        return;
    }

    commands.spawn_batch([&handles.list, &handles.strip].map(|handle| {
        GizmoBundle::new(
            &handle,
            GizmoUniform {
                color: Color::WHITE,
            },
            Mat4::IDENTITY,
        )
    }));

    for (mesh, transform, color) in storage.meshes.iter() {
        commands.spawn(GizmoBundle::new(
            &mesh,
            GizmoUniform { color: *color },
            transform.compute_matrix(),
        ));
    }

    for (translation, normal, radius, color, segments) in storage.discs.iter().copied() {
        let handle = handles.discs.get(segments).unwrap().clone_weak();

        let transform = Transform {
            translation,
            rotation: Quat::from_rotation_arc(Vec3::Z, normal),
            scale: Vec3::splat(radius),
        }
        .compute_matrix();

        commands.spawn(GizmoBundle::new(&handle, GizmoUniform { color }, transform));
    }

    for (position, radius, color, subdivisions) in storage.spheres.iter().copied() {
        let handle = handles.spheres.get(subdivisions).unwrap().clone_weak();

        let transform = Transform::from_translation(position)
            .with_scale(Vec3::splat(radius))
            .compute_matrix();

        commands.spawn(GizmoBundle::new(&handle, GizmoUniform { color }, transform));
    }

    for (translation, rotation, size, color) in storage.rectangles.iter().copied() {
        let transform = Transform {
            translation,
            rotation,
            scale: size.extend(1.),
        }
        .compute_matrix();

        commands.spawn(GizmoBundle::new(
            &handles.rectangle,
            GizmoUniform { color },
            transform,
        ));
    }

    for (transform, color) in storage.cubes.iter().copied() {
        commands.spawn(GizmoBundle::new(
            &handles.cube,
            GizmoUniform { color },
            transform.compute_matrix(),
        ));
    }
}

#[derive(Bundle)]
struct GizmoBundle {
    gizmo: GizmoUniform,

    #[cfg(feature = "bevy_pbr")]
    mesh: Handle<Mesh>,
    #[cfg(feature = "bevy_pbr")]
    mesh_uniform: MeshUniform,
    #[cfg(feature = "bevy_pbr")]
    not_shadow_caster: NotShadowCaster,

    #[cfg(feature = "bevy_sprite")]
    mesh_2d: Mesh2dHandle,
    #[cfg(feature = "bevy_sprite")]
    mesh_2d_uniform: Mesh2dUniform,
}

impl GizmoBundle {
    fn new(mesh: &Handle<Mesh>, gizmo: GizmoUniform, transform: Mat4) -> Self {
        let inverse_transpose_model = transform.inverse().transpose();
        GizmoBundle {
            gizmo,

            #[cfg(feature = "bevy_pbr")]
            mesh: mesh.clone_weak(),
            #[cfg(feature = "bevy_pbr")]
            mesh_uniform: MeshUniform {
                flags: 0,
                transform,
                inverse_transpose_model,
                previous_transform: transform,
            },
            #[cfg(feature = "bevy_pbr")]
            not_shadow_caster: NotShadowCaster,

            #[cfg(feature = "bevy_sprite")]
            mesh_2d: Mesh2dHandle(mesh.clone_weak()),
            #[cfg(feature = "bevy_sprite")]
            mesh_2d_uniform: Mesh2dUniform {
                flags: 0,
                transform,
                inverse_transpose_model,
            },
        }
    }
}
