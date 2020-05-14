use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

/// sets up a scene with textured entities
fn setup(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResourceMut<Assets<Mesh>>,
    mut textures: ResourceMut<Assets<Texture>>,
    mut materials: ResourceMut<Assets<StandardMaterial>>,
) {
    // load a texture
    let texture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/branding/bevy_logo_dark_big.png"
    );
    let texture = Texture::load(TextureType::Png(texture_path.to_string()));
    let aspect = texture.aspect();
    let texture_handle = textures.add(texture);

    // create a new quad mesh. this is what we will apply the texture to
    let quad_width = 8.0;
    let quad_handle = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::new(quad_width, quad_width * aspect),
    }));

    // this material renders the texture normally
    let material_handle = materials.add(StandardMaterial {
        albedo_texture: Some(texture_handle),
        ..Default::default()
    });

    // this material modulates the texture to make it red
    let modulated_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(1.0, 0.0, 0.0, 0.5),
        albedo_texture: Some(texture_handle),
        ..Default::default()
    });

    // add entities to the world
    command_buffer
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
