//! Showcases how a pan-orbit-zoom camera can be used to inspect a CAD model.
//!
//! Features:
//! - Smoothly between perspective and orthographic projection (P)
//! - Toggle between free and constrained orbit (should the camera always be "facing up"?) (C)
//! - Snap to 6 axis-aligned views (1-6)
//! - Explode the model, separating its parts for inspection (E)
//! - Toggling passing thought surface on minimal zool (Z)

use std::time::Duration;

use bevy::camera::Hdr;
use bevy::camera_controller::pan_orbit_camera::{
    extensions::{dolly_zoom::DollyZoomTrigger, look_to::LookToTrigger},
    prelude::*,
};
use bevy::math::DVec3;
use bevy::{
    anti_alias::smaa::Smaa, camera::primitives::Aabb, core_pipeline::tonemapping::Tonemapping,
    pbr::ScreenSpaceAmbientOcclusion, platform::time::Instant, post_process::bloom::Bloom,
    prelude::*, window::RequestRedraw,
};
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultPanOrbitCameraPlugins,
            MeshPickingPlugin,
        ))
        // The camera controller works with reactive rendering:
        // .insert_resource(bevy::winit::WinitSettings::desktop_app())
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_projection,
                toggle_constraint,
                explode,
                switch_direction,
                toggle_zoom,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Original envmaps:
    // let diffuse_map = asset_server.load(get_web_asset_url("bevy_editor_cam_environment_maps/diffuse_rgb9e5_zstd.ktx2"));
    // let specular_map = asset_server.load(get_web_asset_url("bevy_editor_cam_environment_maps/specular_rgb9e5_zstd.ktx2"));
    // let intensity = 1000.0;

    let diffuse_map = asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2");
    let specular_map = asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");
    let intensity = 5000.0;

    let scene = asset_server.load(get_web_asset_url("PlaneEngine/scene.gltf#Scene0"));

    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_scale(Vec3::splat(2.0)),
    ));

    let cam_trans = Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y);
    let camera = commands
        .spawn((
            Camera3d::default(),
            Hdr,
            Camera::default(),
            cam_trans,
            Tonemapping::AcesFitted,
            Bloom::default(),
            EnvironmentMapLight {
                intensity,
                diffuse_map: diffuse_map.clone(),
                specular_map: specular_map.clone(),
                ..Default::default()
            },
            PanOrbitCamera {
                orbit_constraint: OrbitConstraint::Free,
                last_anchor_depth: -cam_trans.translation.length() as f64,
                orthographic: projections::OrthographicSettings {
                    scale_to_near_clip: 1_000_f32, // Needed for SSAO to work in ortho
                    ..Default::default()
                },
                ..Default::default()
            },
            ScreenSpaceAmbientOcclusion::default(),
            Smaa::default(),
            Msaa::Off,
        ))
        .id();

    setup_ui(commands, camera);
}

fn toggle_projection(
    keys: Res<ButtonInput<KeyCode>>,
    mut dolly: MessageWriter<DollyZoomTrigger>,
    cam: Query<Entity, With<PanOrbitCamera>>,
    mut toggled: Local<bool>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        *toggled = !*toggled;
        let target_projection = if *toggled {
            Projection::Orthographic(OrthographicProjection::default_3d())
        } else {
            Projection::Perspective(PerspectiveProjection::default())
        };
        dolly.write(DollyZoomTrigger {
            target_projection,
            camera: cam.single().unwrap(),
        });
    }
}

#[derive(Component)]
struct ZoomStatusIndicator;

fn zoom_status(zoom_through: bool) -> String {
    format!("Zoom Through: {zoom_through}")
}

fn toggle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut cam: Query<&mut PanOrbitCamera>,
    mut text: Query<&mut Text, With<ZoomStatusIndicator>>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        let mut editor = cam.single_mut().unwrap();
        editor.zoom_limits.zoom_through_objects = !editor.zoom_limits.zoom_through_objects;
        text.single_mut().unwrap().0 = zoom_status(editor.zoom_limits.zoom_through_objects);
    }
}

fn toggle_constraint(
    keys: Res<ButtonInput<KeyCode>>,
    mut cam: Query<(Entity, &Transform, &mut PanOrbitCamera)>,
    mut look_to: MessageWriter<LookToTrigger>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        let (entity, transform, mut editor) = cam.single_mut().unwrap();
        match editor.orbit_constraint {
            OrbitConstraint::Fixed { .. } => editor.orbit_constraint = OrbitConstraint::Free,
            OrbitConstraint::Free => {
                editor.orbit_constraint = OrbitConstraint::Fixed {
                    up: DVec3::Y,
                    can_pass_tdc: false,
                };

                look_to.write(LookToTrigger::auto_snap_up_direction(
                    transform.forward().as_dvec3(),
                    entity,
                    &transform.rotation.as_dquat(),
                    editor.as_ref(),
                ));
            }
        };
    }
}

fn switch_direction(
    keys: Res<ButtonInput<KeyCode>>,
    mut look_to: MessageWriter<LookToTrigger>,
    cam: Query<(Entity, &Transform, &PanOrbitCamera)>,
) {
    let (camera, transform, editor) = cam.single().unwrap();
    if keys.just_pressed(KeyCode::Digit1) {
        look_to.write(LookToTrigger::auto_snap_up_direction(
            DVec3::X,
            camera,
            &transform.rotation.as_dquat(),
            editor,
        ));
    }
    if keys.just_pressed(KeyCode::Digit2) {
        look_to.write(LookToTrigger::auto_snap_up_direction(
            DVec3::Z,
            camera,
            &transform.rotation.as_dquat(),
            editor,
        ));
    }
    if keys.just_pressed(KeyCode::Digit3) {
        look_to.write(LookToTrigger::auto_snap_up_direction(
            DVec3::NEG_X,
            camera,
            &transform.rotation.as_dquat(),
            editor,
        ));
    }
    if keys.just_pressed(KeyCode::Digit4) {
        look_to.write(LookToTrigger::auto_snap_up_direction(
            DVec3::NEG_Z,
            camera,
            &transform.rotation.as_dquat(),
            editor,
        ));
    }
    if keys.just_pressed(KeyCode::Digit5) {
        look_to.write(LookToTrigger::auto_snap_up_direction(
            DVec3::Y,
            camera,
            &transform.rotation.as_dquat(),
            editor,
        ));
    }
    if keys.just_pressed(KeyCode::Digit6) {
        look_to.write(LookToTrigger::auto_snap_up_direction(
            DVec3::NEG_Y,
            camera,
            &transform.rotation.as_dquat(),
            editor,
        ));
    }
}

fn setup_ui(mut commands: Commands, camera: Entity) {
    let text = "\
Left Mouse  - Pan
Right Mouse - Orbit
Scroll      - Zoom
Z           - Toggle passing thought surface on minimal zoom
P           - Toggle projection
C           - Toggle orbit constraint
E           - Toggle explode
1-6         - Switch direction";
    commands.spawn((
        children![
            Text::new(text),
            (Text::new(zoom_status(false)), ZoomStatusIndicator),
        ],
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        Node {
            margin: UiRect::all(Val::Px(20.0)),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        Pickable::IGNORE,
        UiTargetCamera(camera),
    ));
}

#[derive(Component)]
struct StartPos(f32);

fn explode(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut toggle: Local<Option<(bool, Instant, f32)>>,
    mut explode_amount: Local<f32>,
    mut redraw: MessageWriter<RequestRedraw>,
    mut parts: Query<(Entity, &mut Transform, &Aabb, Option<&StartPos>), With<Mesh3d>>,
    mut matls: ResMut<Assets<StandardMaterial>>,
) {
    let animation = Duration::from_millis(2000);
    if keys.just_pressed(KeyCode::KeyE) {
        let new = if let Some((last, ..)) = *toggle {
            !last
        } else {
            true
        };
        *toggle = Some((new, Instant::now(), *explode_amount));
    }
    if let Some((toggled, start, start_amount)) = *toggle {
        let goal_amount = toggled as usize as f32;
        let t = (start.elapsed().as_secs_f32() / animation.as_secs_f32()).clamp(0.0, 1.0);
        let progress = CubicSegment::new_bezier_easing((0.25, 0.1), (0.25, 1.0)).ease(t);
        *explode_amount = start_amount + (goal_amount - start_amount) * progress;
        for (part, mut transform, aabb, start) in &mut parts {
            let start = if let Some(start) = start {
                start.0
            } else {
                let start = aabb.max().y;
                commands.entity(part).insert(StartPos(start));
                start
            };
            transform.translation.y = *explode_amount * (start) * 2.0;
        }
        if t < 1.0 {
            redraw.write(RequestRedraw);
        }
    }
    for (_, matl) in matls.iter_mut() {
        matl.perceptual_roughness = matl.perceptual_roughness.clamp(0.3, 1.0);
    }
}

/// Returns the GitHub download URL for the given asset.
///
/// The files are expected to be in the [repository].
///
/// [repository]: https://github.com/bevyengine/bevy_asset_files
fn get_web_asset_url(name: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/bevyengine/bevy_asset_files/refs/heads/main/{}",
        name
    )
}
