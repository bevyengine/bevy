//! Shows how to render simple primitive shapes with a single color.

use bevy::{prelude::*, render::renderer::RenderDevice, sprite::MaterialMesh2dBundle};

use crate::scene_tester::{setup_test, SceneController};

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, scene);
    }
}

fn scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut scene_controller: ResMut<SceneController>,
    mut images: ResMut<Assets<Image>>,
    render_device: Res<RenderDevice>,
) {
    commands.spawn(Camera2dBundle::default());

    // Rectangle
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 100.0)),
            ..default()
        },
        ..default()
    });

    // Circle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(-100., 0., 0.)),
        ..default()
    });

    // Hexagon
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(100., 0., 0.)),
        ..default()
    });

    let render_target = setup_test(
        &mut commands,
        &mut images,
        &render_device,
        &mut scene_controller,
        15,
        String::from("scene_2d_shapes"),
    );

    // camera
    commands.spawn(Camera2dBundle {
        camera: Camera {
            target: render_target,
            ..default()
        },
        ..default()
    });
}
