//! Renders two cameras to the same window to accomplish "split screen".

use std::f32::consts::PI;

use bevy::{
    pbr::CascadeShadowConfigBuilder, prelude::*, render::camera::Viewport, window::WindowResized,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (set_camera_viewports, button_system))
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(100.0, 100.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb")),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 2,
            first_cascade_far_bound: 200.0,
            maximum_distance: 280.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // Cameras and their dedicated UI
    for (index, (camera_name, camera_pos)) in [
        ("Player 1", Vec3::new(0.0, 200.0, -150.0)),
        ("Player 2", Vec3::new(150.0, 150., 50.0)),
        ("Player 3", Vec3::new(100.0, 150., -150.0)),
        ("Player 4", Vec3::new(-100.0, 80., 150.0)),
    ]
    .iter()
    .enumerate()
    {
        let camera = commands
            .spawn((
                Camera3dBundle {
                    transform: Transform::from_translation(*camera_pos)
                        .looking_at(Vec3::ZERO, Vec3::Y),
                    camera: Camera {
                        // Renders cameras with different priorities to prevent ambiguities
                        order: index as isize,
                        // Don't clear after the first camera because the first camera already cleared the entire window
                        clear_color: if index > 0 {
                            ClearColorConfig::None
                        } else {
                            ClearColorConfig::default()
                        },
                        ..default()
                    },
                    ..default()
                },
                CameraPosition {
                    pos: UVec2::new((index % 2) as u32, (index / 2) as u32),
                },
            ))
            .id();

        // Set up UI
        commands
            .spawn((
                TargetCamera(camera),
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        padding: UiRect::all(Val::Px(20.)),
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(*camera_name, TextStyle::default()));
                buttons_panel(parent);
            });
    }

    fn buttons_panel(parent: &mut ChildBuilder) {
        parent
            .spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.)),
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                rotate_button(parent, "<", Direction::Left);
                rotate_button(parent, ">", Direction::Right);
            });
    }

    fn rotate_button(parent: &mut ChildBuilder, caption: &str, direction: Direction) {
        parent
            .spawn((
                RotateCamera(direction),
                ButtonBundle {
                    style: Style {
                        width: Val::Px(40.),
                        height: Val::Px(40.),
                        border: UiRect::all(Val::Px(2.)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: Color::WHITE.into(),
                    image: UiImage::default().with_color(Color::srgb(0.25, 0.25, 0.25)),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(caption, TextStyle::default()));
            });
    }
}

#[derive(Component)]
struct CameraPosition {
    pos: UVec2,
}

#[derive(Component)]
struct RotateCamera(Direction);

enum Direction {
    Left,
    Right,
}

fn set_camera_viewports(
    windows: Query<&Window>,
    mut resize_events: EventReader<WindowResized>,
    mut query: Query<(&CameraPosition, &mut Camera)>,
) {
    // We need to dynamically resize the camera's viewports whenever the window size changes
    // so then each camera always takes up half the screen.
    // A resize_event is sent when the window is first created, allowing us to reuse this system for initial setup.
    for resize_event in resize_events.read() {
        let window = windows.get(resize_event.window).unwrap();
        let size = window.physical_size() / 2;

        for (camera_position, mut camera) in &mut query {
            camera.viewport = Some(Viewport {
                physical_position: camera_position.pos * size,
                physical_size: size,
                ..default()
            });
        }
    }
}

#[allow(clippy::type_complexity)]
fn button_system(
    interaction_query: Query<
        (&Interaction, &TargetCamera, &RotateCamera),
        (Changed<Interaction>, With<Button>),
    >,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    for (interaction, target_camera, RotateCamera(direction)) in &interaction_query {
        if let Interaction::Pressed = *interaction {
            // Since TargetCamera propagates to the children, we can use it to find
            // which side of the screen the button is on.
            if let Ok(mut camera_transform) = camera_query.get_mut(target_camera.entity()) {
                let angle = match direction {
                    Direction::Left => -0.1,
                    Direction::Right => 0.1,
                };
                camera_transform.rotate_around(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, angle));
            }
        }
    }
}
