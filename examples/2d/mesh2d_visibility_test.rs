//! Shows how to render a polygonal [`Mesh`], generated from a [`Rectangle`] primitive, in a 2D scene.

use bevy::{color::palettes::basic::PURPLE, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, test)
        .run();
}

#[derive(Component)]
pub struct T1;

#[derive(Component)]
pub struct T2;

#[derive(Component)]
pub struct T3;

const BAR_FILL_SHAPE: Rectangle = Rectangle::new(21., 3.);
const BAR_FILL_COLOR: Color = Color::srgb(0., 0.5, 0.);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let sprite = Sprite::from_image(asset_server.load("branding/bevy_bird_dark.png"));

    let fill_mesh = meshes.add(BAR_FILL_SHAPE);
    let fill_material = materials.add(BAR_FILL_COLOR);

    commands.spawn((
        T1,
        sprite.clone(),
        children![
            (T2, sprite, Transform::from_xyz(0., 135., 0.)),
            (
                // Visibility::Visible,
                T3,
                Mesh2d(fill_mesh),
                MeshMaterial2d(fill_material),
                Transform::from_xyz(2., -200., 0.),
            )
        ],
    ));
}

fn test(t1: Single<&Children, With<T1>>, t23: Query<Entity, (With<T2>, With<T3>)>) {
    println!("number of children: {}", t1.len());
    println!("t23: {}", t23.iter().len());
}
