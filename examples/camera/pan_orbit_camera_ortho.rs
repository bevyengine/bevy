//! Demonstrates the use of an orthographic camera with the pan-orbit-zoom controller.
//!
//! While this is supported, there are some special considerations when using an orthographic camera.
//! See comments in the code below!

use bevy::camera_controller::pan_orbit_camera::prelude::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            DefaultPanOrbitCameraPlugins,
        ))
        .add_systems(Startup, (setup, setup_ui))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let diffuse_map = asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2");
    let specular_map = asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");
    let translation = Vec3::new(10.0, 10.0, 10.0);

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.01,
            ..OrthographicProjection::default_3d()
        }),
        EnvironmentMapLight {
            intensity: 5000.0,
            diffuse_map: diffuse_map.clone(),
            specular_map: specular_map.clone(),
            rotation: default(),
            affects_lightmapped_mesh_diffuse: true,
        },
        // This component makes the camera controllable with this plugin.
        //
        // Important: the `with_initial_anchor_depth` is critical for an orthographic camera. Unlike
        // perspective, we can't rely on distant things being small to hide precision artifacts.
        // This means we need to be careful with the near and far plane of the camera, especially
        // because in orthographic, the depth precision is linear.
        //
        // This plugin uses the anchor (the point in space the user is interested in) to set the
        // orthographic scale, as well as the near and far planes. This can be a bit tricky if you
        // are unfamiliar with orthographic projections. Consider using an pseudo-ortho projection
        // (see `pseudo_ortho` example) if you don't need a true ortho projection.
        PanOrbitCamera::default().with_initial_anchor_depth(-translation.length() as f64),
        // This is an extension made specifically for orthographic cameras. Because an ortho camera
        // projection has no field of view, a skybox can't be sensibly rendered, only a single point
        // on the skybox would be visible to the camera at any given time. While this is technically
        // correct to what the camera would see, it is not visually helpful nor appealing. It is
        // common for CAD software to render a skybox with a field of view that is decoupled from
        // the camera field of view.
        bevy::camera_controller::pan_orbit_camera::extensions::independent_skybox::IndependentSkybox::new(
            diffuse_map,
            2500.0,
            default(),
        ),
    ));

    spawn_helmets(27, &asset_server, &mut commands);
}

fn spawn_helmets(n: usize, asset_server: &AssetServer, commands: &mut Commands) {
    let half_width = ((ops::powf(n as f32, 1.0 / 3.0) - 1.0) / 2.0) as i32;
    let scene = asset_server.load("https://raw.githubusercontent.com/bevyengine/bevy_asset_files/refs/heads/main/PlaneEngine/scene.gltf#Scene0");
    let width = -half_width..=half_width;
    for x in width.clone() {
        for y in width.clone() {
            for z in width.clone() {
                commands.spawn((
                    WorldAssetRoot(scene.clone()),
                    Transform::from_translation(IVec3::new(x, y, z).as_vec3() * 2.0)
                        .with_scale(Vec3::splat(1.)),
                ));
            }
        }
    }
}

fn setup_ui(mut commands: Commands) {
    let text = "\
Left Mouse - Pan
Right Mouse - Orbit
Scroll - Zoom";
    commands.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        Node {
            margin: UiRect::all(Val::Px(20.0)),
            ..Default::default()
        },
    ));
}
