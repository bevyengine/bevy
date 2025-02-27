//! blah
use bevy::{
    color::palettes::tailwind::CYAN_300,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
};
// use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // .add_plugins(WorldInspectorPlugin::new())
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn((
            Mesh2d(meshes.add(RegularPolygon::new(100.0, 6)).clone()),
            MeshMaterial2d(materials.add(Color::srgb(0.3, 0.5, 0.3)).clone()),
            Transform::from_xyz(100.0, 100., 1.0),
            RenderLayers::from_layers(&[1]),
            Sprite::sized(Vec2::new(100., 100.)),
        ))
        .observe(on_hover)
        .observe(on_out);

    commands
        .spawn((
            Mesh2d(meshes.add(RegularPolygon::new(50., 5))),
            MeshMaterial2d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            Transform::from_xyz(-100.0, 0.5, 2.0),
            RenderLayers::from_layers(&[0]),
            Sprite::sized(Vec2::new(50., 50.)),
        ))
        .observe(on_hover)
        .observe(on_out);

    // Main Camera
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedHorizontal {
        viewport_width: 300.,
    };
    commands.spawn((
        Camera2d,
        Transform::from_xyz(-2.0, 2.5, 5.0),
        Projection::from(projection),
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::from_layers(&[0]),
    ));

    // Overlay camera
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::WindowSize;
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        Projection::from(projection),
        RenderLayers::from_layers(&[1]),
        Transform::from_xyz(-2.0, 2.5, 10.0),
    ));
}

fn on_hover(
    hover: Trigger<Pointer<Over>>,
    mut mesh: Query<&MeshMaterial2d<ColorMaterial>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok(material) = mesh.get_mut(hover.target) {
        info!("found material");
        if let Some(material) = materials.get_mut(material) {
            material.color = Color::WHITE;
        }
    }
}
fn on_out(
    hover: Trigger<Pointer<Out>>,
    mut mesh: Query<&MeshMaterial2d<ColorMaterial>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    info!("observiing");
    if let Ok(material) = mesh.get_mut(hover.target) {
        if let Some(material) = materials.get_mut(material) {
            material.color = Color::from(CYAN_300);
        }
    }
}
