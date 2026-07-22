//! Demonstrates clustered decals, which affix decals to surfaces.

use std::f32::consts::{FRAC_PI_3, PI};
use std::fmt::{self, Formatter};

use bevy::feathers::dark_theme::create_dark_theme;
use bevy::ui_widgets::radio_self_update;
use bevy::{
    color::palettes::css::{LIME, ORANGE_RED, SILVER},
    feathers::{
        controls::{FeathersNumberInput, NumberInputPrecision, NumberInputValue},
        theme::UiTheme,
        FeathersPlugins,
    },
    input::mouse::AccumulatedMouseMotion,
    light::ClusteredDecal,
    pbr::{decal, ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        render_resource::AsBindGroup,
        renderer::{RenderAdapter, RenderDevice},
    },
    shader::ShaderRef,
    ui_widgets::ValueChange,
};
use ops::{acos, cos, sin};

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/number_input.rs"]
mod number_input;

use number_input::number_input_f32;

/// The custom material shader that we use to demonstrate how to use the decal
/// `tag` field.
const SHADER_ASSET_PATH: &str = "shaders/custom_clustered_decal.wgsl";

/// The speed at which the cube rotates, in radians per frame.
const CUBE_ROTATION_SPEED: f32 = 0.02;

/// The speed at which the selection can be moved, in spherical coordinate
/// radians per mouse unit.
const MOVE_SPEED: f32 = 0.008;

/// Various settings for the demo.
#[derive(Resource, Default)]
struct AppStatus {
    /// The object that will be moved, scaled, or rotated when the
    /// mouse is dragged.
    selection: Selection,
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

/// Indicates which aspect of the decal the `FeathersNumberInput` in the app influences.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum AppNumberInput {
    /// The scale (size) of the selected decal.
    #[default]
    Scale,
    /// The roll (rotation) of the selected decal.
    Roll,
}

/// A component that stores the base scale of a clustered decal.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
struct BaseScale(Vec3);

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
        .insert_resource(UiTheme(create_dark_theme()))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Clustered Decals Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
        ))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, CustomDecalExtension>,
        >::default())
        .init_resource::<AppStatus>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, update_help_text)
        .add_observer(handle_drag_as_movement)
        .add_observer(handle_selection_change)
        .add_observer(handle_value_change_number_input)
        .add_observer(radio_self_update)
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
        commands.write_message(AppExit::error());
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
    let base_color_texture = asset_server.load("branding/icon.png");

    commands.spawn((
        ClusteredDecal {
            base_color_texture: Some(base_color_texture.clone()),
            // Tint with red.
            tag: 1,
            ..ClusteredDecal::default()
        },
        calculate_initial_decal_transform(vec3(1.0, 3.0, 5.0), Vec3::ZERO, Vec2::splat(1.1)),
        Selection::DecalA,
    ));

    commands.spawn((
        ClusteredDecal {
            base_color_texture: Some(base_color_texture.clone()),
            // Tint with blue.
            tag: 2,
            ..ClusteredDecal::default()
        },
        calculate_initial_decal_transform(vec3(-2.0, -1.0, 4.0), Vec3::ZERO, Vec2::splat(2.0)),
        Selection::DecalB,
    ));
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    // Spawn the radio buttons that allow the user to select an object to
    // control, and the number inputs that allow the user to alter additional
    // aspects of clustered decals.
    commands.spawn_scene(bsn! {
        radio::main_ui_node_scene()
        Children [
            radio::feathers_option_buttons("Drag to Move",
            &[
                (Selection::Camera, "Camera"),
                (Selection::DecalA, "Decal A"),
                (Selection::DecalB, "Decal B"),
            ]),

            // The number inputs start off hidden because Camera is selected first.
            Visibility::Hidden
            number_input_f32("Scale Multiplier", Some(AppNumberInput::Scale), 1.0, NumberInputPrecision(2), 0.05..10.)
            ,

            Visibility::Hidden
            number_input_f32("Roll (-π to π)", Some(AppNumberInput::Roll), 0.0, NumberInputPrecision(2), -PI..PI)
            ,
        ]
    });
}

/// Observer that handles changes to number inputs.
/// The number inputs affect the scale or rotation of the currently selected decal, if any.
fn handle_value_change_number_input(
    value_change: On<ValueChange<f32>>,
    mut commands: Commands,
    number_input_q: Query<&AppNumberInput, With<FeathersNumberInput>>,
    app_status: ResMut<AppStatus>,
    mut selections: Query<(&mut Transform, &BaseScale, &Selection)>,
) {
    if app_status.selection == Selection::Camera {
        return;
    }
    if let Ok(app_number_input) = number_input_q.get(value_change.source) {
        for (mut transform, base_scale, selection) in &mut selections {
            if app_status.selection != *selection {
                continue;
            }
            match app_number_input {
                AppNumberInput::Scale => {
                    transform.scale = base_scale.0 * value_change.value;
                }
                AppNumberInput::Roll => {
                    let (yaw, pitch, mut _roll) = transform.rotation.to_euler(EulerRot::YXZ);
                    // Keep yaw and pitch the same, but change the roll.
                    transform.rotation =
                        Quat::from_euler(EulerRot::YXZ, yaw, pitch, value_change.value);
                }
            }
        }
        commands
            .entity(value_change.source)
            .insert(NumberInputValue::F32(value_change.value));
    }
}

/// Handles requests from the user to change the selected object to control and expose
/// the appropriate controls.
/// The `radio_self_update` observer handles setting the `Checked` state on the radio buttons.
fn handle_selection_change(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&radio::RadioButtonOptionValue<Selection>>,
    mut app_status: ResMut<AppStatus>,
    mut commands: Commands,
    selections: Query<(&Transform, &BaseScale, &Selection)>,
    number_inputs: Query<(Entity, &ChildOf, &AppNumberInput)>,
) {
    let Ok(radio::RadioButtonOptionValue(selection)) = new_value_query.get(event.value) else {
        return;
    };
    app_status.selection = *selection;

    // Update the visibility of the scale and roll number inputs so that they aren't visible
    // if the camera is selected.
    for (input_entity, child_of, app_number_input) in number_inputs.iter() {
        match app_status.selection {
            Selection::Camera => {
                commands
                    .entity(child_of.parent())
                    .insert(Visibility::Hidden);
            }
            Selection::DecalA | Selection::DecalB => {
                commands
                    .entity(child_of.parent())
                    .insert(Visibility::Inherited);

                // Update the input values to the correct ones for this decal.
                for (transform, base_scale, _selection) in selections
                    .iter()
                    .filter(|&(_, _, selection)| *selection == app_status.selection)
                {
                    if AppNumberInput::Scale == *app_number_input {
                        // Scale should be uniformly multiplied.
                        let scale_multiplier = transform.scale.x / base_scale.0.x;
                        commands
                            .entity(input_entity)
                            .insert(NumberInputValue::F32(scale_multiplier));
                    } else {
                        let roll = transform.rotation.to_euler(EulerRot::YXZ).2;
                        commands
                            .entity(input_entity)
                            .insert(NumberInputValue::F32(roll));
                    }
                }
            }
        };
    }
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
fn calculate_initial_decal_transform(start: Vec3, looking_at: Vec3, size: Vec2) -> impl Bundle {
    let direction = looking_at - start;
    let center = start + direction * 0.5;
    let base_scale = (size * 0.5).extend(direction.length());
    (
        Transform::from_translation(center)
            .with_scale(base_scale)
            .looking_to(direction, Vec3::Y),
        BaseScale(base_scale),
    )
}

/// Rotates the cube a bit every frame.
fn rotate_cube(mut meshes: Query<&mut Transform, With<Mesh3d>>) {
    for mut transform in &mut meshes {
        transform.rotate_y(CUBE_ROTATION_SPEED);
    }
}

/// Process a drag event that moves the selected object.
fn handle_drag_as_movement(
    event: On<Pointer<Drag>>,
    parent_q: Query<&ChildOf>,
    number_input_q: Query<(), With<FeathersNumberInput>>,
    mut selections: Query<(&mut Transform, &Selection)>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // If we are currently dragging the number input, do not interpret it as movement
    // of the selection.
    if parent_q
        .iter_ancestors(event.entity)
        .any(|parent| number_input_q.contains(parent))
    {
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

/// Creates the help string at the top left of the screen.
fn create_help_string(app_status: &AppStatus) -> String {
    if app_status.selection == Selection::Camera {
        format!("Click and drag to move {}.", app_status.selection)
    } else {
        format!(
            "Click and drag to move/scale/rotate {}.\n\
            To scale/rotate, start the drag within the corresponding number input.\n\
            To move, start the drag anywhere else in the example.",
            app_status.selection
        )
    }
}

/// Updates the help text in the top left of the screen to reflect the current
/// selection.
fn update_help_text(mut help_text: Query<&mut Text, With<HelpText>>, app_status: Res<AppStatus>) {
    for mut text in &mut help_text {
        text.0 = create_help_string(&app_status);
    }
}
