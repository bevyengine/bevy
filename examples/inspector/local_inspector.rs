//! Shows the `bevy_inspector` entity tree panel inspecting the app's own world.
//!
//! Run with the `bevy_inspector` feature enabled:
//! ```bash
//! cargo run --example local_inspector --features="bevy_inspector"
//! ```

use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    inspector::{
        entity_tree::{entity_tree_panel, InspectorUi},
        InspectorPlugin, InspectorSelection,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins, InspectorPlugin))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, (demo_scene.spawn(), inspector_ui.spawn()))
        .add_systems(Update, log_selection)
        .run();
}

fn demo_scene() -> impl SceneList {
    bsn_list! {
        Name("Camera")
        Camera3d
        Transform::from_xyz(0.0, 3.5, 8.0).looking_at(Vec3::ZERO, Vec3::Y)
        --
        Name("Sun")
        DirectionalLight {
            shadow_maps_enabled: true,
        }
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y)
        --
        Name("Ground")
        Mesh3d(asset_value(Circle::new(6.0)))
        MeshMaterial3d::<StandardMaterial>(asset_value(Color::srgb(0.25, 0.28, 0.3)))
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        --
        Name("Props")
        Transform::default()
        Visibility::default()
        Children [
            Name("Left Cube")
            Mesh3d(asset_value(Cuboid::new(1.0, 1.0, 1.0)))
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::srgb(0.8, 0.3, 0.3)))
            Transform::from_xyz(-1.5, 0.5, 0.0)
            --
            Name("Right Cube")
            Mesh3d(asset_value(Cuboid::new(1.0, 1.0, 1.0)))
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::srgb(0.3, 0.5, 0.8)))
            Transform::from_xyz(1.5, 0.5, 0.0)
            Children [
                Name("Marker")
                Transform::from_xyz(0.0, 1.2, 0.0)
                Visibility::default()
            ]
        ]
        --
        Mesh3d(asset_value(Sphere::new(0.5)))
        MeshMaterial3d::<StandardMaterial>(asset_value(Color::srgb(0.9, 0.8, 0.3)))
        Transform::from_xyz(0.0, 1.8, -2.0)
    }
}

fn inspector_ui() -> impl Scene {
    bsn! {
        InspectorUi
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            bottom: px(12),
        }
        Children [
            @entity_tree_panel()
        ]
    }
}

fn log_selection(selection: Res<InspectorSelection>, names: Query<&Name>) {
    if !selection.is_changed() {
        return;
    }
    match selection.0 {
        Some(entity) => info!(
            "selected {entity}: {}",
            names
                .get(entity)
                .map(Name::as_str)
                .unwrap_or("<unnamed entity>")
        ),
        None => info!("selection cleared"),
    }
}
