//! Showcases how a pan-orbit-zoom camera can be used to inspect a CAD model.
//!
//! Features:
//! - Smoothly between perspective and orthographic projection (P)
//! - Toggle between free and constrained orbit (should the camera always be "facing up"?) (C)
//! - Snap to 6 axis-aligned views (1-6)
//! - Explode the model, separating its parts for inspection (E)

use std::time::Duration;

use bevy::camera::Hdr;
use bevy::camera_controller::pan_orbit_camera::{
    extensions::{dolly_zoom::DollyZoomTrigger, look_to::LookToTrigger},
    prelude::*,
};
use bevy::math::DVec3;
use bevy::{
    anti_alias::smaa::Smaa, camera::primitives::Aabb, color::palettes::tailwind,
    core_pipeline::tonemapping::Tonemapping, pbr::ScreenSpaceAmbientOcclusion,
    platform::time::Instant, post_process::bloom::Bloom, prelude::*, window::RequestRedraw,
};
use std::f32::consts::FRAC_PI_4;
use std::f32::consts::PI;
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultPanOrbitCameraPlugins,
            MeshPickingPlugin,
        ))
        // The camera controller works with reactive rendering:
        // .insert_resource(bevy::winit::WinitSettings::desktop_app())
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE.into(),
            brightness: 100.0,
            ..default()
        })
        .add_systems(Startup, (spawn_lights, spawn_world, setup).chain())
        .add_systems(
            Update,
            (
                toggle_projection,
                toggle_constraint,
                explode,
                switch_direction,
            )
                .chain(),
        )
        .run();
}

fn spawn_lights(mut commands: Commands) {
    // Main light
    // commands.spawn((
    //     PointLight {
    //         color: Color::from(tailwind::ORANGE_300),
    //         shadow_maps_enabled: true,
    //         ..default()
    //     },
    //     Transform::from_xyz(0.0, 3.0, 0.0),
    // ));
    // // Light behind wall
    // commands.spawn((
    //     PointLight {
    //         color: Color::WHITE,
    //         shadow_maps_enabled: true,
    //         ..default()
    //     },
    //     Transform::from_xyz(-3.5, 3.0, 0.0),
    // ));
    // // Light under floor
    // commands.spawn((
    //     PointLight {
    //         color: Color::from(tailwind::RED_300),
    //         shadow_maps_enabled: true,
    //         ..default()
    //     },
    //     Transform::from_xyz(0.0, -0.5, 0.0),
    // ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
    ));
}

fn spawn_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let sphere = meshes.add(Sphere::new(0.5));
    let wall = meshes.add(Cuboid::new(0.2, 4.0, 3.0));

    let blue_material = materials.add(Color::from(tailwind::BLUE_700));
    let red_material = materials.add(Color::from(tailwind::RED_950));
    let white_material = materials.add(Color::WHITE);

    // Top side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(white_material.clone()),
    ));
    // Under side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(0.0, -0.01, 0.0).with_rotation(Quat::from_rotation_x(PI)),
    ));
    // Blue sphere
    commands.spawn((
        Mesh3d(sphere.clone()),
        MeshMaterial3d(blue_material.clone()),
        Transform::from_xyz(3.0, 1.5, 0.0),
    ));
    // Tall wall
    commands.spawn((
        Mesh3d(wall.clone()),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(-3.0, 2.0, 0.0),
    ));
    // Cube behind wall
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(blue_material.clone()),
        Transform::from_xyz(-4.2, 0.5, 0.0),
    ));
    // Hidden cube under floor
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(red_material.clone()),
        Transform {
            translation: Vec3::new(3.0, -2.0, 0.0),
            rotation: Quat::from_euler(EulerRot::YXZEx, FRAC_PI_4, FRAC_PI_4, 0.0),
            ..default()
        },
    ));
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // let diffuse_map = asset_server.load("environment_maps/diffuse_rgb9e5_zstd.ktx2");
    // let specular_map = asset_server.load("environment_maps/specular_rgb9e5_zstd.ktx2");

    commands.spawn((
        // WorldAssetRoot(asset_server.load("models/PlaneEngine/scene.gltf#Scene0")),
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
    let text = "
        Left Mouse  - Pan
        Right Mouse - Orbit
        Scroll      - Zoom
        P           - Toggle projection
        C           - Toggle orbit constraint
        E           - Toggle explode
        1-6         - Switch direction
    ";
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
        matl.perceptual_roughness = matl.perceptual_roughness.clamp(0.3, 1.0)
    }
}
