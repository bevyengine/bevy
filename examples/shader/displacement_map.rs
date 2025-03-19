//! This example demonstrates the use of displacement maps,
//! which can be used to create inexpensive fluid effects.

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<FlowingWaterMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, animate_time)
        .run();
}

// Defining a custom material that can be applied to a mesh.
// It accept one texture as a base, and second texture
// that will distort the positions of the vertices in the first.
//
// By scrolling the displacement texture, we can create the
// illusion that light bouncing off of the base texture  is
// being refracted by flowing water.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct FlowingWaterMaterial {
    #[texture(0)]
    #[sampler(1)]
    base_texture: Option<Handle<Image>>,

    #[texture(2)]
    #[sampler(3)]
    displacement_texture: Option<Handle<Image>>,

    // A clock value that will increase regularly.
    // The "uniform" attribute indicates that this value
    // is passed from the CPU to the shader GPU. The details of how
    // the GPU uses these it are determined by the custom shader.
    #[uniform(4)]
    time: f32,

    // Determines how quickly the displacement map will
    // be translated; how fast the water effect will ripple.
    // 1.0 is default. 2.0 is twice as fast, etc.
    #[uniform(5)]
    time_sensitivity: f32,

    // How much the displacement map will affect the
    // final position of the vertices.
    #[uniform(6)]
    displacement_intensity: f32,
}

// The Material trait is very configurable, but comes with sensible defaults
// for all methods. We only need to implement functions for features that
// need non-default behavior.
impl Material for FlowingWaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/apply_displacement_map.wgsl".into()
    }
}

// A system that will smoothly increment the clock value on the custom material
fn animate_time(time: Res<Time>, mut materials: ResMut<Assets<FlowingWaterMaterial>>) {
    for (_, material) in materials.iter_mut() {
        material.time += time.delta_secs();
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FlowingWaterMaterial>>,
) {
    // Loading the texture that will be displaced by the displacement map.
    let base_texture_handle = asset_server.load_with_settings(
        "textures/stone_wall.png",
        |settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        },
    );

    // The brightness of each pixel in this image will determine how
    // much the positions of colors in the base texture will be displaced.
    let displacement_texture_handle = asset_server.load_with_settings(
        // Note: This noise texture is toroidal, meaning that the left/right
        // edges, top/bottom edges blend smoothly together. Using non-toroidal
        // noise can cause visible seams in the displacement effect.
        "textures/toroidal_noise.png",
        |settings: &mut ImageLoaderSettings| {
            // Using a linear color space, so that colors are interpreted
            // correctly, and the overall gray-ness of the image is retained.
            settings.is_srgb = false;
            // Using the linear sampler causes the image to be blurred,
            // allowing low-res displacement textures to be less pixelated.
            // Make it even smoother by using a higher-resolution noise texture.
            settings.sampler = ImageSampler::linear();
        },
    );

    // Creating a new quad mesh that the base texture will be applied to.
    let quad_width = 8.0;
    let quad_height = 8.0;
    let quad_handle = meshes.add(Rectangle::new(quad_width, quad_height));

    // Creating the material that will be applied to the quad.
    let material_handle = materials.add(FlowingWaterMaterial {
        base_texture: Some(base_texture_handle),
        displacement_texture: Some(displacement_texture_handle),
        time: 0.0,
        time_sensitivity: 0.3,
        displacement_intensity: 0.04,
    });

    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 10.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
