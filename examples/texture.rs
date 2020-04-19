use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .run();
}

/// sets up a scene with textured entities
fn setup(world: &mut World, resources: &mut Resources) {
    // load a texture
    let mut texture_storage = resources.get_mut::<AssetStorage<Texture>>().unwrap();
    let texture = Texture::load(TextureType::Png(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/bevy_logo_dark_big.png").to_string(),
    ));
    let aspect = texture.height as f32 / texture.width as f32;
    let texture_handle = texture_storage.add(texture);

    // create a new quad mesh. this is what we will apply the texture to
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let quad_width = 8.0;
    let quad_handle = mesh_storage.add(Mesh::from(shape::Quad {
        size: Vec2::new(quad_width, quad_width * aspect),
    }));

    // create materials that use our new texture
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();

    // this material renders the texture normally
    let material_handle = material_storage.add(StandardMaterial {
        albedo_texture: Some(texture_handle),
        ..Default::default()
    });

    // this material modulates the texture to make it red
    let modulated_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgba(1.0, 0.0, 0.0, 0.5),
        albedo_texture: Some(texture_handle),
        ..Default::default()
    });

    // add entities to the world
    world
        .build()
        // textured quad - normal
        .add_entity(MeshEntity {
            mesh: quad_handle,
            material: material_handle,
            translation: Translation::new(0.0, 0.0, 0.0),
            rotation: Rotation::from_euler_angles(0.0, std::f32::consts::PI / 3.0, 0.0),
            ..Default::default()
        })
        // textured quad - modulated
        .add_entity(MeshEntity {
            mesh: quad_handle,
            material: modulated_material_handle,
            translation: Translation::new(0.0, 1.5, 0.0),
            rotation: Rotation::from_euler_angles(0.0, std::f32::consts::PI / 3.0, 0.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(0.0, -5.0, 0.0),
            ..Default::default()
        })
        // camera
        .add_entity(CameraEntity {
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, -8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
