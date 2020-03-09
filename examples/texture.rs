use bevy::prelude::*;

fn main() {
    AppBuilder::new().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));

    let mut texture_storage = resources.get_mut::<AssetStorage<Texture>>().unwrap();
    let texture = Texture::load(TextureType::Png(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/temp_bevy_logo.png").to_string(),
    ));
    let texture_handle = texture_storage.add(texture);

    world
        .build()
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: StandardMaterial {
                albedo: texture_handle.into(),
            },
            translation: Translation::new(0.0, 0.0, 0.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, 4.0, 5.0),
            ..Default::default()
        })
        // camera
        .add_entity(CameraEntity {
            camera: Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            active_camera: ActiveCamera,
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        })
        .build();
}
