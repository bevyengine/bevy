//! This example illustrates how to use assets in an async context (through cloning the underlying
//! `Arc`).

use std::sync::Arc;

use bevy::{
    asset::AssetLoader,
    color::palettes::tailwind,
    math::FloatOrd,
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages},
    tasks::AsyncComputeTaskPool,
};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;

fn main() {
    App::new()
        .add_plugins(
            // This just tells the asset server to look in the right examples folder
            DefaultPlugins.set(AssetPlugin {
                file_path: "examples/asset/files".to_string(),
                ..Default::default()
            }),
        )
        .init_asset::<LinearInterpolation>()
        .register_asset_loader(LinearInterpolationLoader)
        .add_systems(Startup, setup)
        .add_systems(Update, (start_mesh_generation, finish_mesh_generation))
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Res<Assets<Mesh>>,
    mut commands: Commands,
) {
    // Spawn a camera.
    commands.spawn((
        Transform::from_translation(Vec3::new(15.0, 15.0, 15.0)).looking_at(Vec3::ZERO, Vec3::Y),
        Camera3d::default(),
    ));

    // Spawn a light.
    commands.spawn((
        Transform::default().looking_to(Dir3::from_xyz(1.0, -1.0, 0.0).unwrap(), Dir3::Y),
        DirectionalLight::default(),
    ));

    // Spawn the mesh. Reserve the handle so we can generate it later.
    let mesh = meshes.reserve_handle();
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: tailwind::SLATE_100.into(),
            ..Default::default()
        })),
    ));

    // Create the parameters for mesh generation.
    commands.insert_resource(MeshGeneration {
        height_interpolation: asset_server.load("access_asset_from_async_heights.li.ron"),
        mesh,
        size: UVec2::new(30, 30),
    });

    // Create the channel we will communicate across.
    let (sender, receiver) = crossbeam_channel::bounded(1);
    commands.insert_resource(MeshGenerationChannel { sender, receiver });
}

#[derive(Resource)]
struct MeshGeneration {
    height_interpolation: Handle<LinearInterpolation>,
    mesh: Handle<Mesh>,
    size: UVec2,
}

#[derive(Resource)]
struct MeshGenerationChannel {
    sender: crossbeam_channel::Sender<Mesh>,
    receiver: crossbeam_channel::Receiver<Mesh>,
}

/// Starts a mesh generation task whenever the height interpolation asset is updated.
fn start_mesh_generation(
    mut asset_events: EventReader<AssetEvent<LinearInterpolation>>,
    linear_interpolations: Res<Assets<LinearInterpolation>>,
    mesh_generation: Res<MeshGeneration>,
    channel: Res<MeshGenerationChannel>,
) {
    // Only recompute if the height interpolation asset has changed.
    let regenerate_id = mesh_generation.height_interpolation.id();
    let mut recompute = false;
    for asset_event in asset_events.read() {
        match asset_event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } if *id == regenerate_id => {
                recompute = true;
            }
            _ => {}
        }
    }

    if !recompute {
        return;
    }

    let task_pool = AsyncComputeTaskPool::get();
    let size = mesh_generation.size;
    // Get an `Arc` of the height interpolation asset to pass to the spawned task.
    let height_interpolation = linear_interpolations
        .get_arc(&mesh_generation.height_interpolation)
        .expect("The asset is loaded");
    let channel = channel.sender.clone();
    // Spawn a task to generate the mesh, then send the resulting mesh across the channel.
    task_pool
        .spawn(async move {
            let mesh = generate_mesh(size, height_interpolation);
            channel.send(mesh).expect("The channel never closes");
        })
        .detach();
}

/// Reads from the mesh generation channel and inserts the mesh asset.
fn finish_mesh_generation(
    mesh_generation: Res<MeshGeneration>,
    channel: Res<MeshGenerationChannel>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(mesh) = channel.receiver.try_recv() else {
        return;
    };
    meshes.insert(&mesh_generation.mesh, mesh);
}

/// A basic linear interpolation curve implementation.
#[derive(Asset, TypePath, Serialize, Deserialize)]
struct LinearInterpolation(Vec<(f32, f32)>);

impl LinearInterpolation {
    /// Samples the linear interpolation at `value`.
    fn sample(&self, value: f32) -> f32 {
        match self.0.iter().position(|(x, _)| value < *x) {
            None => self.0.last().expect("The interpolation is non-empty").1,
            Some(0) => self.0.first().expect("The interpolation is non-empty").1,
            Some(next) => {
                let previous = next - 1;

                let (next_x, next_y) = self.0[next];
                let (previous_x, previous_y) = self.0[previous];

                let alpha = (value - previous_x) / (next_x - previous_x);

                alpha * (next_y - previous_y) + previous_y
            }
        }
    }
}

#[derive(Default)]
struct LinearInterpolationLoader;

#[derive(Debug, Error)]
enum LinearInterpolationLoaderError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    RonSpannedError(#[from] ron::error::SpannedError),
    #[error("The loaded interpolation is empty.")]
    Empty,
    #[error("The loaded interpolation contains duplicate X values")]
    DuplicateXValues,
}

impl AssetLoader for LinearInterpolationLoader {
    type Asset = LinearInterpolation;
    type Settings = ();
    type Error = LinearInterpolationLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut interpolation: LinearInterpolation = ron::de::from_bytes(&bytes)?;
        if interpolation.0.is_empty() {
            return Err(Self::Error::Empty);
        }
        interpolation.0.sort_by_key(|(key, _)| FloatOrd(*key));
        if interpolation
            .0
            .windows(2)
            .any(|window| window[0].0 == window[1].0)
        {
            return Err(Self::Error::DuplicateXValues);
        }
        Ok(interpolation)
    }

    fn extensions(&self) -> &[&str] {
        &["li.ron"]
    }
}

/// Generates the mesh given the interpolation curve and the size of the mesh.
fn generate_mesh(size: UVec2, interpolation: Arc<LinearInterpolation>) -> Mesh {
    let mut rng = rand_chacha::ChaChaRng::seed_from_u64(12345);

    let center = Vec3::new((size.x as f32) / 2.0, 0.0, (size.y as f32) / -2.0);

    let mut vertices = Vec::with_capacity(((size.x + 1) * (size.y + 1)) as usize);
    let mut uvs = Vec::with_capacity(((size.x + 1) * (size.y + 1)) as usize);
    for y in 0..size.y + 1 {
        for x in 0..size.x + 1 {
            let height = interpolation.sample(rng.r#gen());
            vertices.push(Vec3::new(x as f32, height, -(y as f32)) - center);
            uvs.push(Vec2::new(x as f32, -(y as f32)));
        }
    }

    let y_stride = size.x + 1;
    let mut indices = Vec::with_capacity((size.x * size.y * 6) as usize);
    for y in 0..size.y {
        for x in 0..size.x {
            indices.push(x + y * y_stride);
            indices.push(x + 1 + y * y_stride);
            indices.push(x + 1 + (y + 1) * y_stride);
            indices.push(x + y * y_stride);
            indices.push(x + 1 + (y + 1) * y_stride);
            indices.push(x + (y + 1) * y_stride);
        }
    }

    let mut mesh = Mesh::new(
        bevy_render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices));

    mesh.compute_normals();
    mesh.generate_tangents()
        .expect("The tangents are well formed");

    mesh
}
