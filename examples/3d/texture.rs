use bevy::prelude::*;

/// This example shows various ways to configure texture materials in 3D
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

/// sets up a scene with textured entities
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // load a texture and retrieve its aspect ratio
    let texture_handle = asset_server
        .load_sync(&mut textures, "assets/branding/bevy_logo_dark_big.png")
        .unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let aspect = texture.aspect();

    // create a new quad mesh. this is what we will apply the texture to
    let quad_width = 8.0;
    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        quad_width,
        quad_width * aspect,
    ))));

    // this material renders the texture normally
    let material_handle = materials.add(StandardMaterial {
        albedo_texture: Some(texture_handle),
        shaded: false,
        ..Default::default()
    });

    // this material modulates the texture to make it red (and slightly transparent)
    let red_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(1.0, 0.0, 0.0, 0.5),
        albedo_texture: Some(texture_handle),
        shaded: false,
        ..Default::default()
    });

    // and lets make this one blue! (and also slightly transparent)
    let blue_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(0.0, 0.0, 1.0, 0.5),
        albedo_texture: Some(texture_handle),
        shaded: false,
        ..Default::default()
    });

    // add entities to the world
    commands
        // textured quad - normal
        .spawn(PbrComponents {
            mesh: quad_handle,
            material: material_handle,
            transform: Transform::from_translation_rotation(
                Vec3::new(0.0, 0.0, 1.5),
                Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
            ),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // textured quad - modulated
        .spawn(PbrComponents {
            mesh: quad_handle,
            material: red_material_handle,
            transform: Transform::from_translation_rotation(
                Vec3::new(0.0, 0.0, 0.0),
                Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
            ),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // textured quad - modulated
        .spawn(PbrComponents {
            mesh: quad_handle,
            material: blue_material_handle,
            transform: Transform::from_translation_rotation(
                Vec3::new(0.0, 0.0, -1.5),
                Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
            ),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
