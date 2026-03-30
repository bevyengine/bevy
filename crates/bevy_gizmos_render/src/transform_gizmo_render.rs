//! Mesh-based rendering for the transform gizmo.
//!
//! Uses [`StandardMaterial`] with `unlit: true` and a dedicated overlay camera
//! on a separate [`RenderLayers`] to render gizmo meshes always-on-top.

use bevy_app::{App, Plugin, PostUpdate, Startup};
use bevy_asset::{Assets, Handle};
use bevy_camera::{
    visibility::{RenderLayers, Visibility},
    Camera, Camera3d,
};
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    hierarchy::ChildOf,
    query::{Or, With, Without},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::{
    primitives::{Cone, Cuboid, Cylinder, Torus},
    Quat, Vec3,
};
use bevy_mesh::{Mesh, Mesh3d, MeshBuilder, Meshable};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::{
    components::{GlobalTransform, Transform},
    systems::propagate_transforms_for,
};

use bevy_gizmos::transform_gizmo::{
    TransformGizmoAxis, TransformGizmoCamera, TransformGizmoFocus, TransformGizmoMeshMarker,
    TransformGizmoMode, TransformGizmoRoot, TransformGizmoSettings, TransformGizmoState,
    AXIS_START_OFFSET, COLOR_VIEW, COLOR_X, COLOR_Y, COLOR_Z, CONE_HEIGHT, CONE_RADIUS,
    INACTIVE_ALPHA, ROTATE_RING_RADIUS, SCALE_CUBE_SIZE, SHAFT_LENGTH, SHAFT_RADIUS,
    VIEW_CIRCLE_MAJOR, VIEW_CIRCLE_MINOR, VIEW_RING_MAJOR, VIEW_RING_MINOR,
};

/// The render layer used exclusively for gizmo meshes.
const GIZMO_RENDER_LAYER: usize = 15;

/// Marker for the internal overlay camera that renders gizmo meshes.
#[derive(Component)]
struct GizmoOverlayCamera;

#[derive(Resource)]
struct TransformGizmoMaterials {
    normal_colors: [Color; 4],
    highlight_colors: [Color; 4],
    inactive_colors: [Color; 4],
}

impl TransformGizmoMaterials {
    fn axis_index(axis: TransformGizmoAxis) -> usize {
        match axis {
            TransformGizmoAxis::X => 0,
            TransformGizmoAxis::Y => 1,
            TransformGizmoAxis::Z => 2,
            TransformGizmoAxis::View => 3,
        }
    }

    fn color(&self, axis: TransformGizmoAxis, highlight: bool, inactive: bool) -> Color {
        let i = Self::axis_index(axis);
        if highlight {
            self.highlight_colors[i]
        } else if inactive {
            self.inactive_colors[i]
        } else {
            self.normal_colors[i]
        }
    }
}

/// Plugin that adds mesh-based rendering for the transform gizmo.
///
/// Requires [`bevy_gizmos::transform_gizmo::TransformGizmoPlugin`] to be added first
/// for the interaction logic (hover, drag, state).
pub struct TransformGizmoRenderPlugin;

impl Plugin for TransformGizmoRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            spawn_gizmo_meshes.run_if(
                bevy_ecs::schedule::common_conditions::resource_exists::<TransformGizmoSettings>,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                update_gizmo_meshes,
                propagate_transforms_for::<
                    Or<(
                        With<TransformGizmoRoot>,
                        With<GizmoOverlayCamera>,
                        With<TransformGizmoMeshMarker>,
                    )>,
                >
                    .ambiguous_with_all(),
            )
                .chain()
                .after(bevy_transform::TransformSystems::Propagate)
                .after(bevy_camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
}

fn axis_vec(axis: TransformGizmoAxis) -> Vec3 {
    match axis {
        TransformGizmoAxis::X => Vec3::X,
        TransformGizmoAxis::Y => Vec3::Y,
        TransformGizmoAxis::Z => Vec3::Z,
        TransformGizmoAxis::View => Vec3::ZERO,
    }
}

fn make_unlit_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        cull_mode: None,
        ..Default::default()
    }
}

fn highlight_color(color: Color) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(
        (srgba.red * 1.4).min(1.0),
        (srgba.green * 1.4).min(1.0),
        (srgba.blue * 1.4).min(1.0),
        srgba.alpha,
    )
}

fn inactive_color(color: Color) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(
        srgba.red * INACTIVE_ALPHA,
        srgba.green * INACTIVE_ALPHA,
        srgba.blue * INACTIVE_ALPHA,
        1.0,
    )
}

fn spawn_gizmo_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let gizmo_layer = RenderLayers::layer(GIZMO_RENDER_LAYER);

    let colors = [COLOR_X, COLOR_Y, COLOR_Z, COLOR_VIEW];
    let mat_res = TransformGizmoMaterials {
        normal_colors: colors,
        highlight_colors: colors.map(highlight_color),
        inactive_colors: colors.map(inactive_color),
    };

    // Helper: create a unique unlit material for a given axis
    let mut make_mat = |axis: TransformGizmoAxis| {
        materials.add(make_unlit_material(
            colors[TransformGizmoMaterials::axis_index(axis)],
        ))
    };

    // Pre-create meshes
    let shaft_mesh = meshes.add(Cylinder::new(SHAFT_RADIUS, SHAFT_LENGTH).mesh().build());
    let cone_mesh = meshes.add(Cone::new(CONE_RADIUS, CONE_HEIGHT).mesh().build());
    let scale_cube_mesh = meshes.add(
        Cuboid::new(SCALE_CUBE_SIZE, SCALE_CUBE_SIZE, SCALE_CUBE_SIZE)
            .mesh()
            .build(),
    );
    let rotate_torus_mesh = meshes.add(
        Torus {
            minor_radius: 0.015,
            major_radius: ROTATE_RING_RADIUS,
        }
        .mesh()
        .build(),
    );
    let view_circle_mesh = meshes.add(
        Torus {
            minor_radius: VIEW_CIRCLE_MINOR,
            major_radius: VIEW_CIRCLE_MAJOR,
        }
        .mesh()
        .build(),
    );
    let view_ring_mesh = meshes.add(
        Torus {
            minor_radius: VIEW_RING_MINOR,
            major_radius: VIEW_RING_MAJOR,
        }
        .mesh()
        .build(),
    );

    // Axis rotations: cylinder default is Y-up
    let axis_rotation = |axis: TransformGizmoAxis| -> Quat {
        match axis {
            TransformGizmoAxis::X => Quat::from_rotation_z(-core::f32::consts::FRAC_PI_2),
            TransformGizmoAxis::Y | TransformGizmoAxis::View => Quat::IDENTITY,
            TransformGizmoAxis::Z => Quat::from_rotation_x(core::f32::consts::FRAC_PI_2),
        }
    };

    // Spawn root
    let root_entity = commands
        .spawn((TransformGizmoRoot, Transform::IDENTITY, Visibility::Hidden))
        .id();

    // Helper: spawn a child mesh on the gizmo render layer
    let spawn_child = |commands: &mut Commands,
                       mesh: Handle<Mesh>,
                       material: Handle<StandardMaterial>,
                       transform: Transform,
                       axis: TransformGizmoAxis,
                       mode: TransformGizmoMode| {
        let child = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
                TransformGizmoMeshMarker { axis, mode },
                Visibility::Hidden,
                gizmo_layer.clone(),
            ))
            .id();
        commands.entity(child).insert(ChildOf(root_entity));
    };

    // --- Translate mode ---
    for axis in [
        TransformGizmoAxis::X,
        TransformGizmoAxis::Y,
        TransformGizmoAxis::Z,
    ] {
        let mat = make_mat(axis);
        spawn_child(
            &mut commands,
            shaft_mesh.clone(),
            mat.clone(),
            Transform::from_translation(axis_vec(axis) * (AXIS_START_OFFSET + SHAFT_LENGTH / 2.0))
                .with_rotation(axis_rotation(axis)),
            axis,
            TransformGizmoMode::Translate,
        );
        spawn_child(
            &mut commands,
            cone_mesh.clone(),
            mat,
            Transform::from_translation(
                axis_vec(axis) * (AXIS_START_OFFSET + SHAFT_LENGTH + CONE_HEIGHT / 2.0),
            )
            .with_rotation(axis_rotation(axis)),
            axis,
            TransformGizmoMode::Translate,
        );
    }

    // View-plane circle (translate)
    spawn_child(
        &mut commands,
        view_circle_mesh,
        make_mat(TransformGizmoAxis::View),
        Transform::IDENTITY,
        TransformGizmoAxis::View,
        TransformGizmoMode::Translate,
    );

    // --- Rotate mode ---
    for axis in [
        TransformGizmoAxis::X,
        TransformGizmoAxis::Y,
        TransformGizmoAxis::Z,
    ] {
        let mat = make_mat(axis);
        let torus_rot = match axis {
            TransformGizmoAxis::X => Quat::from_rotation_z(core::f32::consts::FRAC_PI_2),
            TransformGizmoAxis::Y | TransformGizmoAxis::View => Quat::IDENTITY,
            TransformGizmoAxis::Z => Quat::from_rotation_x(core::f32::consts::FRAC_PI_2),
        };
        spawn_child(
            &mut commands,
            rotate_torus_mesh.clone(),
            mat,
            Transform::from_rotation(torus_rot),
            axis,
            TransformGizmoMode::Rotate,
        );
    }

    // View-axis ring (rotate)
    spawn_child(
        &mut commands,
        view_ring_mesh,
        make_mat(TransformGizmoAxis::View),
        Transform::IDENTITY,
        TransformGizmoAxis::View,
        TransformGizmoMode::Rotate,
    );

    // --- Scale mode ---
    for axis in [
        TransformGizmoAxis::X,
        TransformGizmoAxis::Y,
        TransformGizmoAxis::Z,
    ] {
        let mat = make_mat(axis);
        spawn_child(
            &mut commands,
            shaft_mesh.clone(),
            mat.clone(),
            Transform::from_translation(axis_vec(axis) * (AXIS_START_OFFSET + SHAFT_LENGTH / 2.0))
                .with_rotation(axis_rotation(axis)),
            axis,
            TransformGizmoMode::Scale,
        );
        spawn_child(
            &mut commands,
            scale_cube_mesh.clone(),
            mat,
            Transform::from_translation(
                axis_vec(axis) * (AXIS_START_OFFSET + SHAFT_LENGTH + CONE_HEIGHT / 2.0),
            ),
            axis,
            TransformGizmoMode::Scale,
        );
    }

    // --- Overlay camera ---
    // This camera renders only the gizmo layer, after the main camera (order: 1),
    // without clearing the color buffer — so gizmo meshes appear on top of everything.
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            ..Default::default()
        },
        GizmoOverlayCamera,
        RenderLayers::layer(GIZMO_RENDER_LAYER),
        Transform::default(),
    ));

    commands.insert_resource(mat_res);
}

fn update_gizmo_meshes(
    focus: Option<bevy_ecs::system::Single<&GlobalTransform, With<TransformGizmoFocus>>>,
    marked_cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    all_cameras: Query<
        (&Camera, &GlobalTransform),
        (Without<GizmoOverlayCamera>, Without<TransformGizmoRoot>),
    >,
    settings: Option<Res<TransformGizmoSettings>>,
    state: Option<Res<TransformGizmoState>>,
    materials_res: Option<Res<TransformGizmoMaterials>>,
    mut root_query: Query<
        (&mut Transform, &mut Visibility),
        (With<TransformGizmoRoot>, Without<TransformGizmoMeshMarker>),
    >,
    mut handle_query: Query<
        (
            &TransformGizmoMeshMarker,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<TransformGizmoRoot>,
    >,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut overlay_cam: Query<
        &mut Transform,
        (
            With<GizmoOverlayCamera>,
            Without<TransformGizmoRoot>,
            Without<TransformGizmoMeshMarker>,
        ),
    >,
) {
    let (Some(materials_res), Some(settings), Some(state)) = (materials_res, settings, state)
    else {
        return;
    };

    let Ok((mut root_tf, mut root_vis)) = root_query.single_mut() else {
        return;
    };

    let Some(global_tf) = focus else {
        *root_vis = Visibility::Hidden;
        return;
    };
    let Some((_, cam_tf)): Option<(&Camera, &GlobalTransform)> =
        bevy_gizmos::resolve_gizmo_camera!(marked_cameras, all_cameras)
    else {
        *root_vis = Visibility::Hidden;
        return;
    };

    // Copy main camera transform to overlay camera
    if let Ok(mut overlay_tf) = overlay_cam.single_mut() {
        *overlay_tf = cam_tf.compute_transform();
    }

    *root_vis = Visibility::Inherited;
    let pos = global_tf.translation();

    let space = bevy_gizmos::transform_gizmo::effective_space(&settings);
    let rotation = bevy_gizmos::transform_gizmo::gizmo_rotation(*global_tf, space);

    let scale = if settings.screen_scale_factor > 0.0 {
        (cam_tf.translation() - pos).length() * settings.screen_scale_factor
    } else {
        1.0
    };

    root_tf.translation = pos;
    root_tf.rotation = rotation;
    root_tf.scale = Vec3::splat(scale);

    let active_axis = if state.active {
        state.axis
    } else {
        state.hovered_axis
    };
    let dragging = state.active;

    for (handle, mut vis, mat) in &mut handle_query {
        if handle.mode != settings.mode {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;

        // Update the material color in-place (avoids writing MeshMaterial3d)
        let is_active = active_axis == Some(handle.axis);
        let desired_color = materials_res.color(handle.axis, is_active, dragging && !is_active);
        if let Some(mut material) = std_materials.get_mut(&mat.0)
            && material.base_color != desired_color
        {
            material.base_color = desired_color;
        }
    }
}
