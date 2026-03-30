//! Test skinned mesh picking.

use bevy::{
    camera::RenderTarget,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::{BLUE, RED, WHITE},
    picking::pointer::PointerLocation,
    prelude::*,
    scene::SceneInstanceReady,
    window::PrimaryWindow,
};

fn main() {
    App::new()
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
            ..default()
        })
        .add_plugins((DefaultPlugins, MeshPickingPlugin, FreeCameraPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_intersections, keyboard_control))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    commands.spawn((
        Text::new("space: play / pause\n"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    commands.spawn((
        Camera3d::default(),
        FreeCamera::default(),
        Transform::from_xyz(0.8, 1.8, 1.2).looking_at(Vec3::new(-0.1, 1.3, 0.0), Vec3::Y),
    ));

    const GLTF: &str = "models/animated/Fox.glb";

    let (graph, index) =
        AnimationGraph::from_clip(asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF)));

    let graph_handle = graphs.add(graph);

    commands
        .spawn((
            PendingAnimation {
                graph_handle,
                index,
            },
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF))),
            // Use a non-zero translation to make sure we're accounting for the
            // mesh's transform.
            Transform::from_xyz(0.0, 1.0, 0.0)
                // "Fox.glb" uses centimeters instead of meters, so scale to compensate.
                .with_scale(Vec3::splat(0.01)),
        ))
        .observe(on_scene_ready);
}

#[derive(Component)]
struct PendingAnimation {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

fn on_scene_ready(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations: Query<&PendingAnimation>,
    mut players: Query<&mut AnimationPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if let Ok(animation) = animations.get(scene_ready.entity) {
        for child in children.iter_descendants(scene_ready.entity) {
            if let Ok(mut player) = players.get_mut(child) {
                player.play(animation.index).repeat().set_speed(0.25);

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation.graph_handle.clone()));
            }
        }
    }

    // "Fox.glb" lacks tangents by default, so add them here.
    for (_, mesh) in meshes.iter_mut() {
        mesh.generate_tangents().expect("Should always succeed.");
    }
}

fn draw_intersections(
    mut ray_cast: MeshRayCast,
    mut gizmos: Gizmos,
    cameras: Query<(&Camera, &RenderTarget, &GlobalTransform)>,
    pointers: Query<&PointerLocation>,
    primary_window_entity: Query<Entity, With<PrimaryWindow>>,
) {
    for pointer in &pointers {
        for (camera, render_target, camera_transform) in &cameras {
            if !camera.is_active {
                continue;
            }

            let Some(location) = pointer.location().filter(|location| {
                location.is_in_viewport(camera, render_target, &primary_window_entity)
            }) else {
                continue;
            };

            let Some(hit) = camera
                .viewport_to_world(camera_transform, location.position)
                .ok()
                .and_then(|ray| {
                    ray_cast
                        .cast_ray(ray, &MeshRayCastSettings::default())
                        .first()
                        .map(|(_, hit)| hit)
                })
            else {
                continue;
            };

            gizmos.arrow(hit.point, hit.point + (hit.normal.normalize() * 0.1), BLUE);

            if let Some(tangent) = hit.tangent {
                gizmos.arrow(hit.point, hit.point + (tangent.normalize() * 0.1), RED);
            }

            if let Some(triangle) = hit.triangle {
                // Bias the triangle slightly away from the mesh.
                let bias = hit.normal.normalize() * 0.001;

                for i in 0..3usize {
                    gizmos.line(
                        triangle[i] + bias,
                        triangle[(i + 1).rem_euclid(3)] + bias,
                        WHITE,
                    );
                }
            }
        }
    }
}

fn keyboard_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<&mut AnimationPlayer>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for mut player in &mut animation_players {
            if player.all_paused() {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
    }
}
