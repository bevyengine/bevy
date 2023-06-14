use bevy::{
    asset::on_loaded::on_loaded,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

/// This example illustrates how to create a texture for use with a `texture_2d_array<f32>` shader
/// uniform variable.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_asset::<ArrayTexture>()
        .add_plugin(MaterialPlugin::<ArrayTextureMaterial>::default())
        .add_systems(Startup, setup)
        // TODO: this was ported to on_loaded for illustrative purposes only
        .add_systems(Update, on_loaded(create_array_texture))
        .run();
}

#[derive(Asset, TypePath, Clone)]
struct ArrayTexture {
    #[dependency]
    handle: Handle<Image>,
}

impl FromWorld for ArrayTexture {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        Self {
            handle: server.load("textures/array_texture.png"),
        }
    }
}

fn setup(mut commands: Commands) {
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 3000.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(-3.0, 2.0, -1.0),
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 3000.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(3.0, 2.0, 1.0),
        ..Default::default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::new(1.5, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

fn create_array_texture(
    In(array_texture): In<ArrayTexture>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
) {
    let image = images.get_mut(&array_texture.handle).unwrap();

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    image.reinterpret_stacked_2d_as_array(array_layers);

    // Spawn some cubes using the array texture
    let mesh_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material_handle = materials.add(ArrayTextureMaterial {
        array_texture: array_texture.handle.clone(),
    });
    for x in -5..=5 {
        commands.spawn(MaterialMeshBundle {
            mesh: mesh_handle.clone(),
            material: material_handle.clone(),
            transform: Transform::from_xyz(x as f32 + 0.5, 0.0, 0.0),
            ..Default::default()
        });
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ArrayTextureMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    array_texture: Handle<Image>,
}

impl Material for ArrayTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/array_texture.wgsl".into()
    }
}
