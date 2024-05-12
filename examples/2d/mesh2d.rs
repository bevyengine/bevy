//! Shows how to render a polygonal [`Mesh`], generated from a [`Rectangle`] primitive, in a 2D scene.

use bevy::{color::palettes::basic::PURPLE, prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::default()).into(),
        transform: Transform::default().with_scale(Vec3::splat(128.)),
        material: materials.add(Color::from(PURPLE)),
        ..default()
    });
}
