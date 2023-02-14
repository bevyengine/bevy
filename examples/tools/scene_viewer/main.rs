//! A simple glTF scene viewer made with Bevy.
//!
//! Just run `cargo run --release --example scene_viewer /path/to/model.gltf`,
//! replacing the path as appropriate.
//! In case of multiple scenes, you can select which to display by adapting the file path: `/path/to/model.gltf#Scene1`.
//! With no arguments it will load the `FlightHelmet` glTF model from the repository assets subdirectory.

use bevy::{
    math::Vec3A,
    prelude::*,
    render::primitives::{Aabb, Sphere},
    window::WindowPlugin,
};

mod camera_controller_plugin;
mod scene_viewer_plugin;

use camera_controller_plugin::{CameraController, CameraControllerPlugin};
use scene_viewer_plugin::{SceneHandle, SceneViewerPlugin};

fn main() {
    let mut app = App::new();
    app.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0 / 5.0f32,
    })
    .add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy scene viewer".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                asset_folder: std::env::var("CARGO_MANIFEST_DIR")
                    .unwrap_or_else(|_| ".".to_string()),
                watch_for_changes: true,
            }),
    )
    .add_plugin(CameraControllerPlugin)
    .add_plugin(SceneViewerPlugin)
    .add_startup_system(setup)
    .add_system(setup_scene_after_load.in_base_set(CoreSet::PreUpdate));

    app.run();
}

fn parse_scene(scene_path: String) -> (String, usize) {
    if scene_path.contains('#') {
        let gltf_and_scene = scene_path.split('#').collect::<Vec<_>>();
        if let Some((last, path)) = gltf_and_scene.split_last() {
            if let Some(index) = last
                .strip_prefix("Scene")
                .and_then(|index| index.parse::<usize>().ok())
            {
                return (path.join("#"), index);
            }
        }
    }
    (scene_path, 0)
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/models/FlightHelmet/FlightHelmet.gltf".to_string());
    info!("Loading {}", scene_path);
    let (file_path, scene_index) = parse_scene(scene_path);

    commands.insert_resource(SceneHandle::new(asset_server.load(file_path), scene_index));
}

fn setup_scene_after_load(
    mut commands: Commands,
    mut setup: Local<bool>,
    mut scene_handle: ResMut<SceneHandle>,
    asset_server: Res<AssetServer>,
    meshes: Query<(&GlobalTransform, Option<&Aabb>), With<Handle<Mesh>>>,
) {
    if scene_handle.is_loaded && !*setup {
        *setup = true;
        // Find an approximate bounding box of the scene from its meshes
        if meshes.iter().any(|(_, maybe_aabb)| maybe_aabb.is_none()) {
            return;
        }

        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        for (transform, maybe_aabb) in &meshes {
            let aabb = maybe_aabb.unwrap();
            // If the Aabb had not been rotated, applying the non-uniform scale would produce the
            // correct bounds. However, it could very well be rotated and so we first convert to
            // a Sphere, and then back to an Aabb to find the conservative min and max points.
            let sphere = Sphere {
                center: Vec3A::from(transform.transform_point(Vec3::from(aabb.center))),
                radius: transform.radius_vec3a(aabb.half_extents),
            };
            let aabb = Aabb::from(sphere);
            min = min.min(aabb.min());
            max = max.max(aabb.max());
        }

        let size = (max - min).length();
        let aabb = Aabb::from_min_max(Vec3::from(min), Vec3::from(max));

        info!("Spawning a controllable 3D perspective camera");
        let mut projection = PerspectiveProjection::default();
        projection.far = projection.far.max(size * 10.0);

        let camera_controller = CameraController::default();

        // Display the controls of the scene viewer
        info!("{}", camera_controller);
        info!("{}", *scene_handle);

        commands.spawn((
            Camera3dBundle {
                projection: projection.into(),
                transform: Transform::from_translation(
                    Vec3::from(aabb.center) + size * Vec3::new(0.5, 0.25, 0.5),
                )
                .looking_at(Vec3::from(aabb.center), Vec3::Y),
                camera: Camera {
                    is_active: false,
                    ..default()
                },
                ..default()
            },
            #[cfg(all(feature = "ktx2", feature = "zstd"))]
            EnvironmentMapLight {
                diffuse_map: asset_server
                    .load("assets/environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server
                    .load("assets/environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            },
            camera_controller,
        ));
        #[cfg(not(all(feature = "ktx2", feature = "zstd")))]
        {
            warn!("feature ktx2 or zstd wasn't enabled.");
            warn!("rerun this example with `--features=\"ktx2 zstd\" to get environment maps for ambient light");
        }

        // Spawn a default light if the scene does not have one
        if !scene_handle.has_light {
            info!("Spawning a directional light");
            commands.spawn(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadows_enabled: false,
                    ..default()
                },
                ..default()
            });

            scene_handle.has_light = true;
        }
    }
}
