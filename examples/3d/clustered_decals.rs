//! Demonstrates clustered decals, which affix decals to surfaces.

use std::f32::consts::{FRAC_PI_3, PI};
use std::fmt::{self, Formatter};

use bevy::{
    color::palettes::css::{LIME, ORANGE_RED, SILVER},
    input::mouse::AccumulatedMouseMotion,
    light::ClusteredDecal,
    pbr::{decal, ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        render_resource::AsBindGroup,
        renderer::{RenderAdapter, RenderDevice},
    },
    shader::ShaderRef,
    window::{CursorIcon, SystemCursorIcon},
};
use ops::{acos, cos, sin};
use widgets::{
    WidgetClickEvent, WidgetClickSender, BUTTON_BORDER, BUTTON_BORDER_COLOR,
    BUTTON_BORDER_RADIUS_SIZE, BUTTON_PADDING,
};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// The custom material shader that we use to demonstrate how to use the decal
/// `tag` field.
const SHADER_ASSET_PATH: &str = "shaders/custom_clustered_decal.wgsl";

/// The speed at which the cube rotates, in radians per frame.
const CUBE_ROTATION_SPEED: f32 = 0.02;

/// The speed at which the selection can be moved, in spherical coordinate
/// radians per mouse unit.
const MOVE_SPEED: f32 = 0.008;
/// The speed at which the selection can be scaled, in reciprocal mouse units.
const SCALE_SPEED: f32 = 0.05;
/// The speed at which the selection can be scaled, in radians per mouse unit.
const ROLL_SPEED: f32 = 0.01;

/// Various settings for the demo.
#[derive(Resource, Default)]
struct AppStatus {
    /// The object that will be moved, scaled, or rotated when the mouse is
    /// dragged.
    selection: Selection,
    /// What happens when the mouse is dragged: one of a move, rotate, or scale
    /// operation.
    drag_mode: DragMode,
}

/// The object that will be moved, scaled, or rotated when the mouse is dragged.
#[derive(Clone, Copy, Component, Default, PartialEq)]
enum Selection {
    /// The camera.
    ///
    /// The camera can only be moved, not scaled or rotated.
    #[default]
    Camera,
    /// The first decal, which an orange bounding box surrounds.
    DecalA,
    /// The second decal, which a lime green bounding box surrounds.
    DecalB,
}

impl fmt::Display for Selection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Selection::Camera => f.write_str("camera"),
            Selection::DecalA => f.write_str("decal A"),
            Selection::DecalB => f.write_str("decal B"),
        }
    }
}

/// What happens when the mouse is dragged: one of a move, rotate, or scale
/// operation.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum DragMode {
    /// The mouse moves the current selection.
    #[default]
    Move,
    /// The mouse scales the current selection.
    ///
    /// This only applies to decals, not cameras.
    Scale,
    /// The mouse rotates the current selection around its local Z axis.
    ///
    /// This only applies to decals, not cameras.
    Roll,
}

impl fmt::Display for DragMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            DragMode::Move => f.write_str("move"),
            DragMode::Scale => f.write_str("scale"),
            DragMode::Roll => f.write_str("roll"),
        }
    }
}

/// A marker component for the help text in the top left corner of the window.
#[derive(Clone, Copy, Component)]
struct HelpText;

/// A shader extension that demonstrates how to use the `tag` field to customize
/// the appearance of your decals.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct CustomDecalExtension {}

impl MaterialExtension for CustomDecalExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

/// Entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Clustered Decals Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, CustomDecalExtension>,
        >::default())
        .init_resource::<AppStatus>()
        .add_event::<WidgetClickEvent<Selection>>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, widgets::handle_ui_interactions::<Selection>)
        .add_systems(
            Update,
            (handle_selection_change, update_radio_buttons)
                .after(widgets::handle_ui_interactions::<Selection>),
        )
        .add_systems(Update, process_move_input)
        .add_systems(Update, process_scale_input)
        .add_systems(Update, process_roll_input)
        .add_systems(Update, switch_drag_mode)
        .add_systems(Update, update_help_text)
        .add_systems(Update, update_button_visibility)
        .run();
}

/// Creates the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    app_status: Res<AppStatus>,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, CustomDecalExtension>>>,
) {
    // Error out if clustered decals aren't supported on the current platform.
    if !decal::clustered::clustered_decals_are_usable(&render_device, &render_adapter) {
        error!("Clustered decals aren't usable on this platform.");
        commands.write_event(AppExit::error());
    }

    spawn_cube(&mut commands, &mut meshes, &mut materials);
    spawn_camera(&mut commands);
    spawn_light(&mut commands);
    spawn_decals(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
    spawn_help_text(&mut commands, &app_status);
}

/// Spawns the cube onto which the decals are projected.
fn spawn_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, CustomDecalExtension>>,
) {
    // Rotate the cube a bit just to make it more interesting.
    let mut transform = Transform::IDENTITY;
    transform.rotate_y(FRAC_PI_3);

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 3.0, 3.0))),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: SILVER.into(),
                ..default()
            },
            extension: CustomDecalExtension {},
        })),
        transform,
    ));
}

/// Spawns the directional light.
fn spawn_light(commands: &mut Commands) {
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Spawns the camera.
fn spawn_camera(commands: &mut Commands) {
    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(0.0, 2.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y))
        // Tag the camera with `Selection::Camera`.
        .insert(Selection::Camera);
}

/// Spawns the actual clustered decals.
fn spawn_decals(commands: &mut Commands, asset_server: &AssetServer) {
    let image = asset_server.load("branding/icon.png");

    commands.spawn((
        ClusteredDecal {
            image: image.clone(),
            // Tint with red.
            tag: 1,
        },
        calculate_initial_decal_transform(vec3(1.0, 3.0, 5.0), Vec3::ZERO, Vec2::splat(1.1)),
        Selection::DecalA,
    ));

    commands.spawn((
        ClusteredDecal {
            image: image.clone(),
            // Tint with blue.
            tag: 2,
        },
        calculate_initial_decal_transform(vec3(-2.0, -1.0, 4.0), Vec3::ZERO, Vec2::splat(2.0)),
        Selection::DecalB,
    ));
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    // Spawn the radio buttons that allow the user to select an object to
    // control.
    commands
        .spawn(widgets::main_ui_node())
        .with_children(|parent| {
            widgets::spawn_option_buttons(
                parent,
                "Drag to Move",
                &[
                    (Selection::Camera, "Camera"),
                    (Selection::DecalA, "Decal A"),
                    (Selection::DecalB, "Decal B"),
                ],
            );
        });

    // Spawn the drag buttons that allow the user to control the scale and roll
    // of the selected object.
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            right: px(10),
            bottom: px(10),
            column_gap: px(6),
            ..default()
        })
        .with_children(|parent| {
            spawn_drag_button(parent, "Scale").insert(DragMode::Scale);
            spawn_drag_button(parent, "Roll").insert(DragMode::Roll);
        });
}

/// Spawns a button that the user can drag to change a parameter.
fn spawn_drag_button<'a>(
    commands: &'a mut ChildSpawnerCommands,
    label: &str,
) -> EntityCommands<'a> {
    let mut kid = commands.spawn(Node {
        border: BUTTON_BORDER,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: BUTTON_PADDING,
        ..default()
    });
    kid.insert((
        Button,
        BackgroundColor(Color::BLACK),
        BorderRadius::all(BUTTON_BORDER_RADIUS_SIZE),
        BUTTON_BORDER_COLOR,
    ))
    .with_children(|parent| {
        widgets::spawn_ui_text(parent, label, Color::WHITE);
    });
    kid
}

/// Spawns the help text at the top of the screen.
fn spawn_help_text(commands: &mut Commands, app_status: &AppStatus) {
    commands.spawn((
        Text::new(create_help_string(app_status)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        HelpText,
    ));
}

/// Draws the outlines that show the bounds of the clustered decals.
fn draw_gizmos(
    mut gizmos: Gizmos,
    decals: Query<(&GlobalTransform, &Selection), With<ClusteredDecal>>,
) {
    for (global_transform, selection) in &decals {
        let color = match *selection {
            Selection::Camera => continue,
            Selection::DecalA => ORANGE_RED,
            Selection::DecalB => LIME,
        };

        gizmos.primitive_3d(
            &Cuboid {
                // Since the clustered decal is a 1×1×1 cube in model space, its
                // half-size is half of the scaling part of its transform.
                half_size: global_transform.scale() * 0.5,
            },
            Isometry3d {
                rotation: global_transform.rotation(),
                translation: global_transform.translation_vec3a(),
            },
            color,
        );
    }
}

/// Calculates the initial transform of the clustered decal.
fn calculate_initial_decal_transform(start: Vec3, looking_at: Vec3, size: Vec2) -> Transform {
    let direction = looking_at - start;
    let center = start + direction * 0.5;
    Transform::from_translation(center)
        .with_scale((size * 0.5).extend(direction.length()))
        .looking_to(direction, Vec3::Y)
}

/// Rotates the cube a bit every frame.
fn rotate_cube(mut meshes: Query<&mut Transform, With<Mesh3d>>) {
    for mut transform in &mut meshes {
        transform.rotate_y(CUBE_ROTATION_SPEED);
    }
}

/// Updates the state of the radio buttons when the user clicks on one.
fn update_radio_buttons(
    mut widgets: Query<(
        Entity,
        Option<&mut BackgroundColor>,
        Has<Text>,
        &WidgetClickSender<Selection>,
    )>,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
) {
    for (entity, maybe_bg_color, has_text, sender) in &mut widgets {
        let selected = app_status.selection == **sender;
        if let Some(mut bg_color) = maybe_bg_color {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

/// Changes the selection when the user clicks a radio button.
fn handle_selection_change(
    mut events: EventReader<WidgetClickEvent<Selection>>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in events.read() {
        app_status.selection = **event;
    }
}

/// Process a drag event that moves the selected object.
fn process_move_input(
    mut selections: Query<(&mut Transform, &Selection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process drags when movement is selected.
    if !mouse_buttons.pressed(MouseButton::Left) || app_status.drag_mode != DragMode::Move {
        return;
    }

    for (mut transform, selection) in &mut selections {
        if app_status.selection != *selection {
            continue;
        }

        let position = transform.translation;

        // Convert to spherical coordinates.
        let radius = position.length();
        let mut theta = acos(position.y / radius);
        let mut phi = position.z.signum() * acos(position.x * position.xz().length_recip());

        // Camera movement is the inverse of object movement.
        let (phi_factor, theta_factor) = match *selection {
            Selection::Camera => (1.0, -1.0),
            Selection::DecalA | Selection::DecalB => (-1.0, 1.0),
        };

        // Adjust the spherical coordinates. Clamp the inclination to (0, π).
        phi += phi_factor * mouse_motion.delta.x * MOVE_SPEED;
        theta = f32::clamp(
            theta + theta_factor * mouse_motion.delta.y * MOVE_SPEED,
            0.001,
            PI - 0.001,
        );

        // Convert spherical coordinates back to Cartesian coordinates.
        transform.translation =
            radius * vec3(sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi));

        // Look at the center, but preserve the previous roll angle.
        let roll = transform.rotation.to_euler(EulerRot::YXZ).2;
        transform.look_at(Vec3::ZERO, Vec3::Y);
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

/// Processes a drag event that scales the selected target.
fn process_scale_input(
    mut selections: Query<(&mut Transform, &Selection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process drags when the scaling operation is selected.
    if !mouse_buttons.pressed(MouseButton::Left) || app_status.drag_mode != DragMode::Scale {
        return;
    }

    for (mut transform, selection) in &mut selections {
        if app_status.selection == *selection {
            transform.scale *= 1.0 + mouse_motion.delta.x * SCALE_SPEED;
        }
    }
}

/// Processes a drag event that rotates the selected target along its local Z
/// axis.
fn process_roll_input(
    mut selections: Query<(&mut Transform, &Selection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process drags when the rolling operation is selected.
    if !mouse_buttons.pressed(MouseButton::Left) || app_status.drag_mode != DragMode::Roll {
        return;
    }

    for (mut transform, selection) in &mut selections {
        if app_status.selection != *selection {
            continue;
        }

        let (yaw, pitch, mut roll) = transform.rotation.to_euler(EulerRot::YXZ);
        roll += mouse_motion.delta.x * ROLL_SPEED;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

/// Creates the help string at the top left of the screen.
fn create_help_string(app_status: &AppStatus) -> String {
    format!(
        "Click and drag to {} {}",
        app_status.drag_mode, app_status.selection
    )
}

/// Changes the drag mode when the user hovers over the "Scale" and "Roll"
/// buttons in the lower right.
///
/// If the user is hovering over no such button, this system changes the drag
/// mode back to its default value of [`DragMode::Move`].
fn switch_drag_mode(
    mut commands: Commands,
    mut interactions: Query<(&Interaction, &DragMode)>,
    mut windows: Query<Entity, With<Window>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut app_status: ResMut<AppStatus>,
) {
    if mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    for (interaction, drag_mode) in &mut interactions {
        if *interaction != Interaction::Hovered {
            continue;
        }

        app_status.drag_mode = *drag_mode;

        // Set the cursor to provide the user with a nice visual hint.
        for window in &mut windows {
            commands
                .entity(window)
                .insert(CursorIcon::from(SystemCursorIcon::EwResize));
        }
        return;
    }

    app_status.drag_mode = DragMode::Move;

    for window in &mut windows {
        commands.entity(window).remove::<CursorIcon>();
    }
}

/// Updates the help text in the top left of the screen to reflect the current
/// selection and drag mode.
fn update_help_text(mut help_text: Query<&mut Text, With<HelpText>>, app_status: Res<AppStatus>) {
    for mut text in &mut help_text {
        text.0 = create_help_string(&app_status);
    }
}

/// Updates the visibility of the drag mode buttons so that they aren't visible
/// if the camera is selected.
fn update_button_visibility(
    mut nodes: Query<&mut Visibility, With<DragMode>>,
    app_status: Res<AppStatus>,
) {
    for mut visibility in &mut nodes {
        *visibility = match app_status.selection {
            Selection::Camera => Visibility::Hidden,
            Selection::DecalA | Selection::DecalB => Visibility::Visible,
        };
    }
}
