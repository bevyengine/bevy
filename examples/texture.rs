use bevy::{asset, prelude::*};

fn main() {
    AppBuilder::new().add_defaults_legacy().setup_world(setup).run();
}

fn setup(world: &mut World) {
    let cube_handle = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        let cube = Mesh::load(MeshType::Cube);
        (mesh_storage.add(cube))
    };

    let texture_handle = {
        let mut texture_storage = world.resources.get_mut::<AssetStorage<Texture>>().unwrap();
        let texture = Texture::load(TextureType::Data(asset::create_texels(256)));
        (texture_storage.add(texture))
    };

    // cube
    world.insert(
        (),
        vec![(
            cube_handle,
            Material::new(Albedo::Texture(texture_handle)),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 1.0),
        )],
    );

    // light
    world.insert(
        (),
        vec![(
            Light {
                color: wgpu::Color {
                    r: 0.8,
                    g: 0.8,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(60.0),
                depth: 0.1..50.0,
                target_view: None,
            },
            LocalToWorld::identity(),
            Translation::new(4.0, -4.0, 5.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0),
        )],
    );

    // camera
    world.insert(
        (),
        vec![(
            Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            ActiveCamera,
            LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        )],
    );
}
