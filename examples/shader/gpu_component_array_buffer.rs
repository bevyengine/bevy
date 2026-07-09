//! Demonstrates use of `GpuComponentArrayBuffer` to store custom
//! per-mesh-instance data.
//!
//! This example repeatedly spawns and despawns randomly colored and textured
//! cubes, in order to demonstrate (and test) that Bevy automatically manages
//! indices of elements within `GpuComponentArrayBuffer`. Bindless is used when
//! supported; if bindless is supported on the platform, you can verify with a
//! debugger that all cubes are drawn in a single drawcall.

use std::time::Duration;

use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    reflect::TypePath,
    render::{
        gpu_component_array_buffer::{
            GpuComponentArray, GpuComponentArrayBuffer, GpuComponentArrayBufferPlugin,
        },
        render_resource::{AsBindGroup, ShaderType},
        storage::ShaderBuffer,
    },
    shader::ShaderRef,
    time::common_conditions::on_timer,
};
use bytemuck::{Pod, Zeroable};
use chacha20::ChaCha8Rng;
use rand::{seq::IndexedRandom, RngExt as _, SeedableRng as _};

/// This example uses a shader source file from the assets subdirectory.
const SHADER_ASSET_PATH: &str = "shaders/gpu_component_array_buffer.wgsl";

/// Data that the example uses.
#[derive(Resource)]
struct AppData {
    /// The cube mesh.
    mesh: Handle<Mesh>,
    /// One of the randomly chosen materials.
    material_light: Handle<CustomMaterial>,
    /// The other one of the randomly chosen materials.
    material_dark: Handle<CustomMaterial>,
    /// The random number generator.
    ///
    /// This is explicitly seeded to maintain consistency between runs of the
    /// example.
    rng: ChaCha8Rng,
}

/// The material that uses the data in the [`GpuComponentArrayBuffer`].
///
/// Bindless textures will be used if supported on the target platform.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bindless(index_table(range(0..4)))]
struct CustomMaterial {
    /// The data that the [`GpuComponentArrayBuffer`] manages.
    ///
    /// This is a single [`ShaderBuffer`] that contains all the data used by
    /// each mesh instance, indexed by the
    /// [`MeshTag`](bevy_mesh::components::MeshTag).
    #[storage(1, read_only, binding_array(4))]
    data: Handle<ShaderBuffer>,

    /// A texture that will be tinted by the [`CustomMaterialData::color`].
    #[texture(2)]
    #[sampler(3)]
    color_texture: Handle<Image>,
}

/// The per-mesh-instance data that will be extracted from the ECS and supplied
/// ot the GPU.
#[derive(Clone, Copy, Component, Debug)]
struct CustomMaterialData {
    /// A tint color to modulate the texture by.
    color: Vec3,
}

/// The GPU version of the per-mesh-instance data.
///
/// This is copied byte-by-byte to the GPU, not processed through
/// [`ShaderType`]. Consequently, we must insert all padding ourselves. We pad
/// the value out to 16 bytes, which is a good conservative practice.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
struct GpuCustomMaterialData {
    /// The tint color to modulate the texture by.
    color: Vec3,
    /// Padding to pad this data out to a multiple of 16 bytes.
    pad: u32,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<CustomMaterial>::default(),
            // Make sure to include the `GpuComponentArrayBufferPlugin`
            // corresponding to our GPU data.
            GpuComponentArrayBufferPlugin::<CustomMaterialData>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // Spawn a new cube every 0.3 s.
                add_cube.run_if(on_timer(Duration::from_millis(300))),
                // Despawn a cube every second.
                remove_cube.run_if(on_timer(Duration::from_millis(1000))),
            ),
        )
        .run();
}

/// Loads our assets and spawns the camera.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bindless_materials: ResMut<Assets<CustomMaterial>>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
    asset_server: Res<AssetServer>,
) {
    // Create our `GpuComponentArray` that contains the data that we're going to
    // make available to the GPU.
    let component_array = GpuComponentArray::<CustomMaterialData>::new(&mut shader_buffers);
    let buffer = component_array.buffer.clone();
    commands.insert_resource(component_array);

    // Create a cube mesh.
    let mesh = meshes.add(Cuboid::default());

    // Load the image for each material below.
    let (texture_dark, texture_light) = (
        asset_server.load("branding/bevy_bird_dark.png"),
        asset_server.load("branding/icon.png"),
    );

    // Load the two materials. We'll randomly pick between the two when spawning
    // each cube.
    let material_light = bindless_materials.add(CustomMaterial {
        data: buffer.clone(),
        color_texture: texture_light,
    });
    let material_dark = bindless_materials.add(CustomMaterial {
        data: buffer.clone(),
        color_texture: texture_dark,
    });

    // Save the assets we just loaded for use later.
    commands.insert_resource(AppData {
        mesh,
        material_light,
        material_dark,
        rng: ChaCha8Rng::seed_from_u64(12345),
    });

    // Spawn a camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 1.25, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// A system that spawns a new cube with a random position, color, and material.
fn add_cube(mut commands: Commands, mut app_data: ResMut<AppData>) {
    // Choose a random position.
    let xz_offset = vec2(
        app_data.rng.random_range((-1.0)..1.0),
        app_data.rng.random_range((-1.0)..1.0),
    );
    // Choose a random color.
    let color = vec3(
        app_data.rng.random_range((0.0)..1.0),
        app_data.rng.random_range((0.0)..1.0),
        app_data.rng.random_range((0.0)..1.0),
    );
    // Choose a random material.
    let material = if app_data.rng.random_bool(0.5) {
        app_data.material_light.clone()
    } else {
        app_data.material_dark.clone()
    };

    // Spawn the cube.
    commands.spawn((
        Mesh3d(app_data.mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_xyz(xz_offset.x, 0.5, xz_offset.y).with_scale(Vec3::splat(0.1)),
        CustomMaterialData { color },
    ));
}

/// A system that despawns a random cube.
fn remove_cube(
    mut commands: Commands,
    mut app_data: ResMut<AppData>,
    cubes: Query<Entity, With<CustomMaterialData>>,
) {
    // Find all cubes in the scene.
    let all_cubes: Vec<Entity> = cubes.iter().collect();
    // Pick one randomly, and despawn it.
    if let Some(&cube_to_despawn) = all_cubes.choose(&mut app_data.rng) {
        commands.entity(cube_to_despawn).despawn();
    }
}

impl GpuComponentArrayBuffer for CustomMaterialData {
    // The query we perform every frame to extract our component to the GPU.
    type QueryData = Read<CustomMaterialData>;
    // The filter we apply to this query. Note that we only extract components
    // that have changed, for efficiency. Typically, you will want to use a
    // `Changed` filter here.
    type QueryFilter = Changed<CustomMaterialData>;
    // The GPU representation of the data.
    type Out = GpuCustomMaterialData;

    // Extracts the data from the ECS and packages it up into a form suitable
    // for the GPU.
    fn extract_component(data: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(GpuCustomMaterialData {
            color: data.color,
            pad: 0,
        })
    }
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
