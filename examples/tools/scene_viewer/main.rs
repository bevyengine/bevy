//! A simple glTF scene viewer made with Bevy.
//!
//! Just run `cargo run --release --example scene_viewer /path/to/model.gltf`,
//! replacing the path as appropriate.
//! In case of multiple scenes, you can select which to display by adapting the file path: `/path/to/model.gltf#Scene1`.
//! With no arguments it will load the `FlightHelmet` glTF model from the repository assets subdirectory.
//! Pass `--help` to see all the supported arguments.
//!
//! If you want to hot reload asset changes, enable the `file_watcher` cargo feature.

use argh::FromArgs;
use bevy::{
    asset::UnapprovedPathMode,
    camera::primitives::{Aabb, Sphere},
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    gltf::GltfPlugin,
    pbr::DefaultOpaqueRendererMethod,
    prelude::*,
    render::experimental::occlusion_culling::OcclusionCulling,
};

#[path = "../../helpers/camera_controller.rs"]
mod camera_controller;

#[cfg(feature = "animation")]
mod animation_plugin;
mod morph_viewer_plugin;
mod scene_viewer_plugin;

use camera_controller::{CameraController, CameraControllerPlugin};
use morph_viewer_plugin::MorphViewerPlugin;
use scene_viewer_plugin::{SceneHandle, SceneViewerPlugin};

/// A simple glTF scene viewer made with Bevy
#[derive(FromArgs, Resource)]
struct Args {
    /// the path to the glTF scene
    #[argh(
        positional,
        default = "\"assets/models/FlightHelmet/FlightHelmet.gltf\".to_string()"
    )]
    scene_path: String,
    /// enable a depth prepass
    #[argh(switch)]
    depth_prepass: Option<bool>,
    /// enable occlusion culling
    #[argh(switch)]
    occlusion_culling: Option<bool>,
    /// enable deferred shading
    #[argh(switch)]
    deferred: Option<bool>,
    /// spawn a light even if the scene already has one
    #[argh(switch)]
    add_light: Option<bool>,
    /// enable `GltfPlugin::use_model_forward_direction`
    #[argh(switch)]
    use_model_forward_direction: Option<bool>,
}

impl Args {
    fn rotation(&self) -> Quat {
        if self.use_model_forward_direction == Some(true) {
            // If the scene is converted then rotate everything else to match. This
            // makes comparisons easier - the scene will always face the same way
            // relative to the camera.
            Quat::from_xyzw(0.0, 1.0, 0.0, 0.0)
        } else {
            Quat::IDENTITY
        }
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args: Args = Args::from_args(&[], &[]).unwrap();

    let deferred = args.deferred;

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy scene viewer".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
                // Allow scenes to be loaded from anywhere on disk
                unapproved_path_mode: UnapprovedPathMode::Allow,
                ..default()
            })
            .set(GltfPlugin {
                use_model_forward_direction: args.use_model_forward_direction.unwrap_or(false),
                ..default()
            }),
        CameraControllerPlugin,
        SceneViewerPlugin,
        MorphViewerPlugin,
    ))
    .insert_resource(args)
    .add_systems(Startup, setup)
    .add_systems(PreUpdate, setup_scene_after_load);

    // If deferred shading was requested, turn it on.
    if deferred == Some(true) {
        app.insert_resource(DefaultOpaqueRendererMethod::deferred());
    }

    #[cfg(feature = "animation")]
    app.add_plugins(animation_plugin::AnimationManipulationPlugin);

    app.run();
}

fn parse_scene(scene_path: String) -> (String, usize) {
    if scene_path.contains('#') {
        let gltf_and_scene = scene_path.split('#').collect::<Vec<_>>();
        if let Some((last, path)) = gltf_and_scene.split_last()
            && let Some(index) = last
                .strip_prefix("Scene")
                .and_then(|index| index.parse::<usize>().ok())
        {
            return (path.join("#"), index);
        }
    }
    (scene_path, 0)
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    let scene_path = &args.scene_path;
    info!("Loading {}", scene_path);
    let (file_path, scene_index) = parse_scene((*scene_path).clone());

    commands.insert_resource(SceneHandle::new(asset_server.load(file_path), scene_index));
}

fn setup_scene_after_load(
    mut commands: Commands,
    mut setup: Local<bool>,
    mut scene_handle: ResMut<SceneHandle>,
    asset_server: Res<AssetServer>,
    args: Res<Args>,
    meshes: Query<(&GlobalTransform, Option<&Aabb>), With<Mesh3d>>,
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

        let walk_speed = size * 3.0;
        let camera_controller = CameraController {
            walk_speed,
            run_speed: 3.0 * walk_speed,
            ..default()
        };

        // Display the controls of the scene viewer
        info!("{}", camera_controller);
        info!("{}", *scene_handle);

        let mut camera = commands.spawn((
            Camera3d::default(),
            Projection::from(projection),
            Transform::from_translation(
                Vec3::from(aabb.center) + size * (args.rotation() * Vec3::new(0.5, 0.25, 0.5)),
            )
            .looking_at(Vec3::from(aabb.center), Vec3::Y),
            Camera {
                is_active: false,
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server
                    .load("assets/environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server
                    .load("assets/environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
                intensity: 150.0,
                rotation: args.rotation(),
                ..default()
            },
            camera_controller,
        ));

        // If occlusion culling was requested, include the relevant components.
        // The Z-prepass is currently required.
        if args.occlusion_culling == Some(true) {
            camera.insert((DepthPrepass, OcclusionCulling));
        }

        // If the depth prepass was requested, include it.
        if args.depth_prepass == Some(true) {
            camera.insert(DepthPrepass);
        }

        // If deferred shading was requested, include the prepass.
        if args.deferred == Some(true) {
            camera
                .insert(Msaa::Off)
                .insert(DepthPrepass)
                .insert(DeferredPrepass);
        }

        // Spawn a default light if the scene does not have one
        if !scene_handle.has_light || args.add_light == Some(true) {
            info!("Spawning a directional light");
            let mut light = commands.spawn((
                DirectionalLight::default(),
                Transform::from_translation(args.rotation() * Vec3::new(1.0, 1.0, 0.0))
                    .looking_at(Vec3::ZERO, Vec3::Y),
            ));
            if args.occlusion_culling == Some(true) {
                light.insert(OcclusionCulling);
            }

            scene_handle.has_light = true;
        }
    }
}
