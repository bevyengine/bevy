//! Showcase how to create diagnostics overlays.
use bevy::{
    color::palettes::tailwind::RED_400,
    dev_tools::diagnostics_overlay::{DiagnosticsOverlay, DiagnosticsOverlayPlugin},
    diagnostic::FrameTimeDiagnosticsPlugin,
    input::common_conditions::input_just_pressed,
    pbr::diagnostic::MaterialAllocatorDiagnosticPlugin,
    prelude::*,
    render::diagnostic::MeshAllocatorDiagnosticPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Make sure we have some diagnostics to display
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
            MeshAllocatorDiagnosticPlugin,
        ))
        // Enable the diagnostics overlay
        .add_plugins(DiagnosticsOverlayPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_fps_diagnostics.run_if(input_just_pressed(KeyCode::KeyF)),
                toggle_mesh_and_material_diagnostics.run_if(input_just_pressed(KeyCode::KeyM)),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Setup a simple scene
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-12.5, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight { ..default() },
        Transform::from_translation(Vec3::X * 15. + Vec3::Y * 20.).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(standard_materials.add(StandardMaterial::from_color(RED_400))),
    ));

    // Instruction text
    commands.spawn((
        Text::new(concat!(
            "Press F to toggle the FPS overlay.\n",
            "Press M to toggle the mesh and material overlay.\n",
        )),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Marker component for the FPS overlay, so we can toggle it on/off
#[derive(Component)]
pub struct FpsOverlay;

/// Marker component for the Mesh and Material overlay, so we can toggle it on/off
#[derive(Component)]
pub struct MeshMaterialOverlay;

fn toggle_fps_diagnostics(
    mut commands: Commands,
    overlay: Query<Entity, (With<DiagnosticsOverlay>, With<FpsOverlay>)>,
    mut enabled: Local<bool>,
) {
    if *enabled {
        for e in overlay {
            commands.entity(e).despawn();
        }
        *enabled = false;
    } else {
        commands.spawn((DiagnosticsOverlay::fps(), FpsOverlay));
        *enabled = true;
    }
}

fn toggle_mesh_and_material_diagnostics(
    mut commands: Commands,
    overlay: Query<Entity, (With<DiagnosticsOverlay>, With<MeshMaterialOverlay>)>,
    mut enabled: Local<bool>,
) {
    if *enabled {
        for e in overlay {
            commands.entity(e).despawn();
        }
        *enabled = false;
    } else {
        commands.spawn((
            DiagnosticsOverlay::mesh_and_standard_material(),
            MeshMaterialOverlay,
        ));
        *enabled = true;
    }
}
