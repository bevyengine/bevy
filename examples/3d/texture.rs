use bevy::prelude::*;

/// This example shows various ways to configure texture materials in 3D
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

/// sets up a scene with textured entities
fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // load a texture and retrieve its aspect ratio
    let texture_handle = asset_server.load("branding/bevy_logo_dark_big.png");
    let aspect = 0.25;

    // create a new quad mesh. this is what we will apply the texture to
    let quad_width = 8.0;
    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        quad_width,
        quad_width * aspect,
    ))));

    // this material renders the texture normally
    let material_handle = materials.add(StandardMaterial {
        albedo_texture: Some(texture_handle.clone()),
        shaded: false,
        ..Default::default()
    });

    // this material modulates the texture to make it red (and slightly transparent)
    let red_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(1.0, 0.0, 0.0, 0.5),
        albedo_texture: Some(texture_handle.clone()),
        shaded: false,
    });

    // and lets make this one blue! (and also slightly transparent)
    let blue_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(0.0, 0.0, 1.0, 0.5),
        albedo_texture: Some(texture_handle),
        shaded: false,
    });

    // add entities to the world
    commands
        // textured quad - normal
        .spawn(PbrBundle {
            mesh: quad_handle.clone(),
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.5),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                ..Default::default()
            },
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // textured quad - modulated
        .spawn(PbrBundle {
            mesh: quad_handle.clone(),
            material: red_material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                ..Default::default()
            },
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // textured quad - modulated
        .spawn(PbrBundle {
            mesh: quad_handle,
            material: blue_material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, -1.5),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                ..Default::default()
            },
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(3.0, 5.0, 8.0))
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
