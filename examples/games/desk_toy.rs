//! Bevy logo as a desk toy using transparent windows! Now with Googly Eyes!
//!
//! This example demonstrates:
//! - Transparent windows that can be clicked through.
//! - Drag-and-drop operations in 2D.
//! - Using entity hierarchy, Transform, and Visibility to create simple animations.
//! - Creating simple 2D meshes based on shape primitives.

use bevy::{
    app::AppExit,
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    window::{CursorOptions, PrimaryWindow, WindowLevel},
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
        .insert_resource(ClearColor(WINDOW_CLEAR_COLOR))
        .insert_resource(WindowTransparency(false))
        .insert_resource(CursorWorldPos(None))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                get_cursor_world_pos,
                update_cursor_hit_test,
                (
                    start_drag.run_if(input_just_pressed(MouseButton::Left)),
                    end_drag.run_if(input_just_released(MouseButton::Left)),
                    drag.run_if(resource_exists::<DragOperation>),
                    quit.run_if(input_just_pressed(MouseButton::Right)),
                    toggle_transparency.run_if(input_just_pressed(KeyCode::Space)),
                    move_pupils.after(drag),
                ),
            )
                .chain(),
        )
        .run();
}

/// Whether the window is transparent
#[derive(Resource)]
struct WindowTransparency(bool);

/// The projected 2D world coordinates of the cursor (if it's within primary window bounds).
#[derive(Resource)]
struct CursorWorldPos(Option<Vec2>);

/// The current drag operation including the offset with which we grabbed the Bevy logo.
#[derive(Resource)]
struct DragOperation(Vec2);

/// Marker component for the instructions text entity.
#[derive(Component)]
struct InstructionsText;

/// Marker component for the Bevy logo entity.
#[derive(Component)]
struct BevyLogo;

/// Component for the moving pupil entity (the moving part of the googly eye).
#[derive(Component)]
struct Pupil {
    /// Radius of the eye containing the pupil.
    eye_radius: f32,
    /// Radius of the pupil.
    pupil_radius: f32,
    /// Current velocity of the pupil.
    velocity: Vec2,
}

// Dimensions are based on: assets/branding/icon.png
// Bevy logo radius
const BEVY_LOGO_RADIUS: f32 = 128.0;
// Birds' eyes x y (offset from the origin) and radius
// These values are manually determined from the logo image
const BIRDS_EYES: [(f32, f32, f32); 3] = [
    (145.0 - 128.0, -(56.0 - 128.0), 12.0),
    (198.0 - 128.0, -(87.0 - 128.0), 10.0),
    (222.0 - 128.0, -(140.0 - 128.0), 8.0),
];

const WINDOW_CLEAR_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);

/// Spawn the scene
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Spawn a 2D camera
    commands.spawn(Camera2d);

    // Spawn the text instructions
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextFont {
        font: font.clone(),
        font_size: 25.0,
        ..default()
    };
    commands.spawn((
        Text2d::new("Press Space to play on your desktop! Press it again to return.\nRight click Bevy logo to exit."),
            text_style.clone(),
            Transform::from_xyz(0.0, -300.0, 100.0),
        InstructionsText,
    ));

    // Create a circle mesh. We will reuse this mesh for all our circles.
    let circle = meshes.add(Circle { radius: 1.0 });
    // Create the different materials we will use for each part of the eyes. For this demo they are basic [`ColorMaterial`]s.
    let outline_material = materials.add(Color::BLACK);
    let sclera_material = materials.add(Color::WHITE);
    let pupil_material = materials.add(Color::srgb(0.2, 0.2, 0.2));
    let pupil_highlight_material = materials.add(Color::srgba(1.0, 1.0, 1.0, 0.2));

    // Spawn the Bevy logo sprite
    commands
        .spawn((
            Sprite::from_image(asset_server.load("branding/icon.png")),
            BevyLogo,
        ))
        .with_children(|commands| {
            // For each bird eye
            for (x, y, radius) in BIRDS_EYES {
                let pupil_radius = radius * 0.6;
                let pupil_highlight_radius = radius * 0.3;
                let pupil_highlight_offset = radius * 0.3;
                // eye outline
                commands.spawn((
                    Mesh2d(circle.clone()),
                    MeshMaterial2d(outline_material.clone()),
                    Transform::from_xyz(x, y - 1.0, 1.0)
                        .with_scale(Vec2::splat(radius + 2.0).extend(1.0)),
                ));

                // sclera
                commands.spawn((
                    Transform::from_xyz(x, y, 2.0),
                    Visibility::default(),
                    children![
                        // sclera
                        (
                            Mesh2d(circle.clone()),
                            MeshMaterial2d(sclera_material.clone()),
                            Transform::from_scale(Vec3::new(radius, radius, 0.0)),
                        ),
                        // pupil
                        (
                            Transform::from_xyz(0.0, 0.0, 1.0),
                            Visibility::default(),
                            Pupil {
                                eye_radius: radius,
                                pupil_radius,
                                velocity: Vec2::ZERO,
                            },
                            children![
                                // pupil main
                                (
                                    Mesh2d(circle.clone()),
                                    MeshMaterial2d(pupil_material.clone()),
                                    Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
                                        pupil_radius,
                                        pupil_radius,
                                        1.0,
                                    )),
                                ),
                                // pupil highlight
                                (
                                    Mesh2d(circle.clone()),
                                    MeshMaterial2d(pupil_highlight_material.clone()),
                                    Transform::from_xyz(
                                        -pupil_highlight_offset,
                                        pupil_highlight_offset,
                                        1.0,
                                    )
                                    .with_scale(Vec3::new(
                                        pupil_highlight_radius,
                                        pupil_highlight_radius,
                                        1.0,
                                    )),
                                )
                            ],
                        )
                    ],
                ));
            }
        });
}

/// Project the cursor into the world coordinates and store it in a resource for easy use
fn get_cursor_world_pos(
    mut cursor_world_pos: ResMut<CursorWorldPos>,
    primary_window: Single<&Window, With<PrimaryWindow>>,
    q_camera: Single<(&Camera, &GlobalTransform)>,
) {
    let (main_camera, main_camera_transform) = *q_camera;
    // Get the cursor position in the world
    cursor_world_pos.0 = primary_window.cursor_position().and_then(|cursor_pos| {
        main_camera
            .viewport_to_world_2d(main_camera_transform, cursor_pos)
            .ok()
    });
}

/// Update whether the window is clickable or not
fn update_cursor_hit_test(
    cursor_world_pos: Res<CursorWorldPos>,
    primary_window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    bevy_logo_transform: Single<&Transform, With<BevyLogo>>,
) {
    let (window, mut cursor_options) = primary_window.into_inner();
    // If the window has decorations (e.g. a border) then it should be clickable
    if window.decorations {
        cursor_options.hit_test = true;
        return;
    }

    // If the cursor is not within the window we don't need to update whether the window is clickable or not
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    // If the cursor is within the radius of the Bevy logo make the window clickable otherwise the window is not clickable
    cursor_options.hit_test = bevy_logo_transform
        .translation
        .truncate()
        .distance(cursor_world_pos)
        < BEVY_LOGO_RADIUS;
}

/// Start the drag operation and record the offset we started dragging from
fn start_drag(
    mut commands: Commands,
    cursor_world_pos: Res<CursorWorldPos>,
    bevy_logo_transform: Single<&Transform, With<BevyLogo>>,
) {
    // If the cursor is not within the primary window skip this system
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    // Get the offset from the cursor to the Bevy logo sprite
    let drag_offset = bevy_logo_transform.translation.truncate() - cursor_world_pos;

    // If the cursor is within the Bevy logo radius start the drag operation and remember the offset of the cursor from the origin
    if drag_offset.length() < BEVY_LOGO_RADIUS {
        commands.insert_resource(DragOperation(drag_offset));
    }
}

/// Stop the current drag operation
fn end_drag(mut commands: Commands) {
    commands.remove_resource::<DragOperation>();
}

/// Drag the Bevy logo
fn drag(
    drag_offset: Res<DragOperation>,
    cursor_world_pos: Res<CursorWorldPos>,
    time: Res<Time>,
    mut bevy_transform: Single<&mut Transform, With<BevyLogo>>,
    mut q_pupils: Query<&mut Pupil>,
) {
    // If the cursor is not within the primary window skip this system
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    // Calculate the new translation of the Bevy logo based on cursor and drag offset
    let new_translation = cursor_world_pos + drag_offset.0;

    // Calculate how fast we are dragging the Bevy logo (unit/second)
    let drag_velocity =
        (new_translation - bevy_transform.translation.truncate()) / time.delta_secs();

    // Update the translation of Bevy logo transform to new translation
    bevy_transform.translation = new_translation.extend(bevy_transform.translation.z);

    // Add the cursor drag velocity in the opposite direction to each pupil.
    // Remember pupils are using local coordinates to move. So when the Bevy logo moves right they need to move left to
    // simulate inertia, otherwise they will move fixed to the parent.
    for mut pupil in &mut q_pupils {
        pupil.velocity -= drag_velocity;
    }
}

/// Quit when the user right clicks the Bevy logo
fn quit(
    cursor_world_pos: Res<CursorWorldPos>,
    mut app_exit: MessageWriter<AppExit>,
    bevy_logo_transform: Single<&Transform, With<BevyLogo>>,
) {
    // If the cursor is not within the primary window skip this system
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    // If the cursor is within the Bevy logo radius send the [`AppExit`] event to quit the app
    if bevy_logo_transform
        .translation
        .truncate()
        .distance(cursor_world_pos)
        < BEVY_LOGO_RADIUS
    {
        app_exit.write(AppExit::Success);
    }
}

/// Enable transparency for the window and make it on top
fn toggle_transparency(
    mut commands: Commands,
    mut window_transparency: ResMut<WindowTransparency>,
    mut q_instructions_text: Query<&mut Visibility, With<InstructionsText>>,
    mut primary_window: Single<&mut Window, With<PrimaryWindow>>,
) {
    // Toggle the window transparency resource
    window_transparency.0 = !window_transparency.0;

    // Show or hide the instructions text
    for mut visibility in &mut q_instructions_text {
        *visibility = if window_transparency.0 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }

    // Remove the primary window's decorations (e.g. borders), make it always on top of other desktop windows, and set the clear color to transparent
    // only if window transparency is enabled
    let clear_color;
    (
        primary_window.decorations,
        primary_window.window_level,
        clear_color,
    ) = if window_transparency.0 {
        (false, WindowLevel::AlwaysOnTop, Color::NONE)
    } else {
        (true, WindowLevel::Normal, WINDOW_CLEAR_COLOR)
    };

    // Set the clear color
    commands.insert_resource(ClearColor(clear_color));
}

/// Move the pupils and bounce them around
fn move_pupils(time: Res<Time>, mut q_pupils: Query<(&mut Pupil, &mut Transform)>) {
    for (mut pupil, mut transform) in &mut q_pupils {
        // The wiggle radius is how much the pupil can move within the eye
        let wiggle_radius = pupil.eye_radius - pupil.pupil_radius;
        // Store the Z component
        let z = transform.translation.z;
        // Truncate the Z component to make the calculations be on [`Vec2`]
        let mut translation = transform.translation.truncate();
        // Decay the pupil velocity
        pupil.velocity *= ops::powf(0.04f32, time.delta_secs());
        // Move the pupil
        translation += pupil.velocity * time.delta_secs();
        // If the pupil hit the outside border of the eye, limit the translation to be within the wiggle radius and invert the velocity.
        // This is not physically accurate but it's good enough for the googly eyes effect.
        if translation.length() > wiggle_radius {
            translation = translation.normalize() * wiggle_radius;
            // Invert and decrease the velocity of the pupil when it bounces
            pupil.velocity *= -0.75;
        }
        // Update the entity transform with the new translation after reading the Z component
        transform.translation = translation.extend(z);
    }
}
