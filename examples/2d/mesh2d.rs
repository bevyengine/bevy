//! Shows how to render a polygonal [`Mesh`], generated from a [`Rectangle`] primitive, in a 2D scene.

use bevy::{color::palettes::basic::PURPLE, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut asset_commands: AssetCommands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Mesh2d(asset_commands.spawn_asset(Mesh::from(Rectangle::default()))),
        MeshMaterial2d(asset_commands.spawn_asset(ColorMaterial::from_color(PURPLE))),
        Transform::default().with_scale(Vec3::splat(128.)),
    ));
}
