//! Shows the `bevy_inspector` entity tree and details panels inspecting the app's own world.
//!
//! Run with the `bevy_inspector` feature enabled:
//! ```bash
//! cargo run --example local_inspector --features="bevy_inspector,debug"
//! ```

use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    inspector::{
        details_panel::details_panel,
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

/// A component exercising every widget kind the details panel renders.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component, Default)]
struct Showcase {
    enabled: bool,
    health: f32,
    count: u32,
    offset: i16,
    ratio: f64,
    label: String,
    tint: Color,
    mode: Mode,
    bounds: Bounds,
    tags: Vec<u32>,
}

#[derive(Reflect, Default, Clone, Copy, PartialEq)]
enum Mode {
    #[default]
    Idle,
    Walking,
    Running,
}

#[derive(Reflect, Default, Clone, Copy)]
struct Bounds {
    min: Vec2,
    max: Vec2,
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
            --
            Name("Showcase")
            Showcase {
                enabled: true,
                health: 72.5,
                count: 3,
                offset: -4,
                ratio: 0.25,
                label: "hello",
                tint: Color::srgb(0.95, 0.55, 0.2),
                mode: Mode::Walking,
                bounds: Bounds {
                    min: Vec2::new(-1.0, -1.0),
                    max: Vec2::new(2.0, 3.0),
                },
                tags: { vec![1, 2, 3] },
            }
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
        Name::new("Inspector")
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            bottom: px(12),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            column_gap: px(12),
        }
        Children [
            @entity_tree_panel()
            --
            @details_panel()
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
