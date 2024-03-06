//! Bevy logo as a desk toy! Now with Googly Eyes!

use bevy::{
    app::AppExit,
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::{PrimaryWindow, WindowLevel},
};

#[cfg(target_os = "macos")]
use bevy::window::CompositeAlphaMode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Desk Toy".into(),
                transparent: true,
                #[cfg(target_os = "macos")]
                composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .insert_resource(CursorWorldPos(None))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (get_cursor_world_pos, update_cursor_hit_test).chain(),
        )
        .add_systems(
            Update,
            (
                start_drag.run_if(input_just_pressed(MouseButton::Left)),
                end_drag.run_if(input_just_released(MouseButton::Left)),
                drag.run_if(resource_exists::<DragOffset>),
                quit.run_if(input_just_pressed(MouseButton::Right)),
                enable_transparency.run_if(input_just_pressed(KeyCode::Space)),
                update_pupils.after(drag),
            )
                .after(update_cursor_hit_test),
        )
        .run();
}

#[derive(Resource)]
struct CursorWorldPos(Option<Vec2>);

#[derive(Resource)]
struct DragOffset(Vec2);

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct InstructionsText;

#[derive(Component)]
struct BevyLogo;

#[derive(Component)]
struct Pupil {
    eye_radius: f32,
    pupil_radius: f32,
    velocity: Vec2,
}

// based on: branding/icon.png
// Bevy logo radius
const BEVY_LOGO_RADIUS: f32 = 128.0;
// Birds' eyes x y (offset from the origin) and radius
const BIRDS_EYES: [(f32, f32, f32); 3] = [
    (145.0 - 128.0, -(56.0 - 128.0), 12.0),
    (198.0 - 128.0, -(87.0 - 128.0), 10.0),
    (222.0 - 128.0, -(140.0 - 128.0), 8.0),
];

/// Spawn the scene
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2dBundle::default(), MainCamera));

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 30.0,
        color: Color::WHITE,
    };
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "Press Space to play on your desktop!\nRight click Bevy logo to exit.",
                text_style.clone(),
            ),
            transform: Transform::from_xyz(0.0, -300.0, 100.0),
            ..default()
        },
        InstructionsText,
    ));

    let circle = Mesh2dHandle(meshes.add(Circle { radius: 1.0 }));
    let outline_material = materials.add(Color::BLACK);
    let sclera_material = materials.add(Color::WHITE);
    let pupil_material = materials.add(Color::srgb(0.2, 0.2, 0.2));
    let pupil_highlight_material = materials.add(Color::srgba(1.0, 1.0, 1.0, 0.2));

    commands
        .spawn((
            SpriteBundle {
                texture: asset_server.load("branding/icon.png"),
                ..default()
            },
            BevyLogo,
        ))
        .with_children(|commands| {
            for (x, y, radius) in BIRDS_EYES {
                // eye outline
                commands.spawn(MaterialMesh2dBundle {
                    mesh: circle.clone(),
                    material: outline_material.clone(),
                    transform: Transform::from_xyz(x, y - 1.0, 1.0).with_scale(Vec3::new(
                        radius + 2.0,
                        radius + 2.0,
                        1.0,
                    )),
                    ..default()
                });

                // sclera
                commands
                    .spawn(SpatialBundle::from_transform(Transform::from_xyz(
                        x, y, 2.0,
                    )))
                    .with_children(|commands| {
                        // sclera
                        commands.spawn(MaterialMesh2dBundle {
                            mesh: circle.clone(),
                            material: sclera_material.clone(),
                            transform: Transform::from_scale(Vec3::new(radius, radius, 0.0)),
                            ..default()
                        });

                        let pupil_radius = radius * 0.6;
                        let pupil_highlight_radius = radius * 0.3;
                        let pupil_highlight_offset = radius * 0.3;
                        // pupil
                        commands
                            .spawn((
                                SpatialBundle::from_transform(Transform::from_xyz(0.0, 0.0, 1.0)),
                                Pupil {
                                    eye_radius: radius,
                                    pupil_radius,
                                    velocity: Vec2::ZERO,
                                },
                            ))
                            .with_children(|commands| {
                                // pupil main
                                commands.spawn(MaterialMesh2dBundle {
                                    mesh: circle.clone(),
                                    material: pupil_material.clone(),
                                    transform: Transform::from_xyz(0.0, 0.0, 0.0)
                                        .with_scale(Vec3::new(pupil_radius, pupil_radius, 1.0)),
                                    ..default()
                                });

                                // pupil highlight
                                commands.spawn(MaterialMesh2dBundle {
                                    mesh: circle.clone(),
                                    material: pupil_highlight_material.clone(),
                                    transform: Transform::from_xyz(
                                        -pupil_highlight_offset,
                                        pupil_highlight_offset,
                                        1.0,
                                    )
                                    .with_scale(Vec3::new(
                                        pupil_highlight_radius,
                                        pupil_highlight_radius,
                                        1.0,
                                    )),
                                    ..default()
                                });
                            });
                    });
            }
        });
}

/// Project the cursor into the world coordinates and store it in a resource for easy use
fn get_cursor_world_pos(
    mut cursor_world_pos: ResMut<CursorWorldPos>,
    q_primary_window: Query<&Window, With<PrimaryWindow>>,
    q_main_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let primary_window = q_primary_window.single();
    let (main_camera, main_camera_transform) = q_main_camera.single();
    // Get the cursor position in the world
    cursor_world_pos.0 = primary_window
        .cursor_position()
        .and_then(|cursor_pos| main_camera.viewport_to_world_2d(main_camera_transform, cursor_pos));
}

/// Update whether the window is clickable or not
fn update_cursor_hit_test(
    cursor_world_pos: Res<CursorWorldPos>,
    mut q_primary_window: Query<&mut Window, With<PrimaryWindow>>,
    q_bevy_logo: Query<&Transform, With<BevyLogo>>,
) {
    let mut primary_window = q_primary_window.single_mut();
    if primary_window.decorations {
        primary_window.cursor.hit_test = true;
        return;
    }

    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    let bevy_logo_transform = q_bevy_logo.single();
    primary_window.cursor.hit_test =
        (bevy_logo_transform.translation.truncate() - cursor_world_pos).length() < BEVY_LOGO_RADIUS;
}

/// Start the drag operation and record the offset we started dragging from
fn start_drag(
    mut commands: Commands,
    cursor_world_pos: Res<CursorWorldPos>,
    q_bevy_logo: Query<&Transform, With<BevyLogo>>,
) {
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    let bevy_logo_transform = q_bevy_logo.single();
    let drag_offset = bevy_logo_transform.translation.truncate() - cursor_world_pos;
    if drag_offset.length() < BEVY_LOGO_RADIUS {
        commands.insert_resource(DragOffset(drag_offset));
    }
}

/// Stop the current drag operation
fn end_drag(mut commands: Commands) {
    commands.remove_resource::<DragOffset>();
}

/// Drag the Bevy logo
fn drag(
    drag_offset: Res<DragOffset>,
    cursor_world_pos: Res<CursorWorldPos>,
    time: Res<Time>,
    mut q_bevy_logo: Query<&mut Transform, With<BevyLogo>>,
    mut q_pupils: Query<&mut Pupil>,
) {
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    let mut bevy_transform = q_bevy_logo.single_mut();
    let new_translation = cursor_world_pos + drag_offset.0;
    let drag_velocity =
        (new_translation - bevy_transform.translation.truncate()) / time.delta_seconds();
    bevy_transform.translation = new_translation.extend(bevy_transform.translation.z);

    for mut pupil in &mut q_pupils {
        pupil.velocity -= drag_velocity;
    }
}

/// Quit when the user right clicks the Bevy logo
fn quit(
    cursor_world_pos: Res<CursorWorldPos>,
    mut app_exit: EventWriter<AppExit>,
    q_bevy_logo: Query<&Transform, With<BevyLogo>>,
) {
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    let bevy_logo_transform = q_bevy_logo.single();
    if (bevy_logo_transform.translation.truncate() - cursor_world_pos).length() < BEVY_LOGO_RADIUS {
        app_exit.send(AppExit);
    }
}

/// Enable transparency for the window and make it on top
fn enable_transparency(
    mut commands: Commands,
    q_instructions_text: Query<Entity, With<InstructionsText>>,
    mut q_primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let _ = q_instructions_text
        .get_single()
        .map(|entity| commands.entity(entity).despawn_recursive());
    let mut window = q_primary_window.single_mut();
    window.decorations = false;
    window.window_level = WindowLevel::AlwaysOnTop;
    commands.insert_resource(ClearColor(Color::NONE));
}

/// Bounce the pupils around
fn update_pupils(time: Res<Time>, mut q_pupils: Query<(&mut Pupil, &mut Transform)>) {
    for (mut pupil, mut transform) in &mut q_pupils {
        let wiggle_radius = pupil.eye_radius - pupil.pupil_radius;
        let z = transform.translation.z;
        let mut translation = transform.translation.truncate();
        pupil.velocity *= 0.93;
        let mut new_translation = translation + (pupil.velocity * time.delta_seconds());
        if new_translation.length() > wiggle_radius {
            new_translation = new_translation.normalize() * wiggle_radius;
            pupil.velocity *= -1.0;
        }
        translation = new_translation;
        transform.translation = translation.extend(z);
    }
}
