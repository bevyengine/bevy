//! This example illustrates how to create a texture for use with a
//! `texture_2d_array<f32>` shader uniform variable and then how to sample from
//! that texture in the shader by using a `MeshTag` component on the mesh
//! entity.

use bevy::{
    image::{ImageArrayLayout, ImageLoaderSettings},
    mesh::MeshTag,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// This example uses a shader source file from the assets subdirectory.
const SHADER_ASSET_PATH: &str = "shaders/array_texture.wgsl";

/// Corresponds to the number of layers in the array texture.
const TEXTURE_COUNT: u32 = 4;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<ArrayTextureMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update_mesh_tags)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Load the texture.
    let array_texture = asset_server.load_with_settings(
        "textures/array_texture.png",
        |settings: &mut ImageLoaderSettings| {
            settings.array_layout = Some(ImageArrayLayout::RowCount {
                rows: TEXTURE_COUNT,
            });
        },
    );

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::new(1.5, 0.0, 0.0), Vec3::Y),
    ));

    // Spawn some cubes using the array texture.
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(ArrayTextureMaterial { array_texture });
    for x in -5..=5 {
        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material_handle.clone()),
            // Pass a different mesh tag to allow selecting different layers of
            // the array texture in the shader.
            MeshTag(x as u32 % TEXTURE_COUNT),
            Transform::from_xyz(x as f32 + 0.5, 0.0, 0.0),
        ));
    }
}

fn update_mesh_tags(time: Res<Time>, mut query: Query<&mut MeshTag>, mut timer: Local<Timer>) {
    // Initialize the timer on the first run.
    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(1.5, TimerMode::Repeating);
    }

    timer.tick(time.delta());
    if timer.just_finished() {
        for mut tag in query.iter_mut() {
            // Cycle through the texture layers to demonstrate that we can
            // select different layers of the array texture at runtime.
            tag.0 = (tag.0 + 1) % TEXTURE_COUNT;
        }
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
        SHADER_ASSET_PATH.into()
    }
}
