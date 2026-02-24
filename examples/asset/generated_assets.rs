//! Shows how to generate and store assets at runtime.

use std::{thread::sleep, time::Duration};

use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    time::common_conditions::once_after_delay,
};
use rand::{rngs::StdRng, RngExt, SeedableRng};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            generate_image.run_if(once_after_delay(Duration::from_secs(4))),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 0.0),
        FreeCamera::default(),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Dir3::new(Vec3::new(-1.0, -1.0, -1.0)).unwrap(), Dir3::Y),
    ));

    commands.spawn((
        // `add_async` creates a task that runs your async function. Once it completes, the asset is
        // added to the `Assets`. This is "deferred" meaning that the asset may take a frame to be
        // added after the task completes.
        Mesh3d(asset_server.add_async(generate_mesh(UVec2::new(100, 100), 1234))),
        // Another way to generate an asset is to add it directly to the `Assets`.
        MeshMaterial3d(materials.add(StandardMaterial::default())),
    ));

    // The last way to generate assets is to reserve a handle, and then use `Assets::insert` to
    // populate the asset later. In this example, the `generate_image` system runs to populate the
    // image
    let image_handle = images.reserve_handle();
    commands.insert_resource(HandleToGenerate(image_handle.clone()));
    commands.spawn(ImageNode::new(image_handle));
}

async fn generate_mesh(size: UVec2, seed: u64) -> Result<Mesh, std::io::Error> {
    // This mesh could take a while to generate!
    sleep(Duration::from_secs(3));

    let mut rng = StdRng::seed_from_u64(seed);
    let mut positions = vec![];
    for y in 0..size.y {
        for x in 0..size.x {
            positions.push(Vec3::new(
                x as f32 - size.x as f32 / 2.0,
                rng.random::<f32>() * 2.0,
                y as f32 - size.y as f32 / 2.0,
            ));
        }
    }

    let compute_index = |(x, y): (u32, u32)| x + y * size.x;
    let mut indices = vec![];
    for y in 0..(size.y - 1) {
        for x in 0..(size.x - 1) {
            indices.extend(
                [
                    (x, y),
                    (x, y + 1),
                    (x + 1, y),
                    (x + 1, y),
                    (x, y + 1),
                    (x + 1, y + 1),
                ]
                .into_iter()
                .map(compute_index),
            );
        }
    }

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices))
    .with_computed_normals())
}

#[derive(Resource)]
struct HandleToGenerate(Handle<Image>);

/// This system runs after a delay to populate the handle in [`HandleToGenerate`].
///
/// This generates a runtime image. Since it's a system, it can use other data in the world to
/// generate the asset!
fn generate_image(handle_to_generate: Res<HandleToGenerate>, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 300,
            height: 300,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..image.height() {
        for x in 0..image.width() {
            image
                .set_color_at(
                    x,
                    y,
                    Color::Srgba(Srgba::new(
                        x as f32 / (image.width() - 1) as f32,
                        y as f32 / (image.height() - 1) as f32,
                        0.0,
                        1.0,
                    )),
                )
                .unwrap();
        }
    }
    images.insert(&handle_to_generate.0, image).unwrap();
}
