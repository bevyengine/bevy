//! Demonstrates light textures, which modulate light sources.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, PI};
use std::fmt::{self, Formatter};

use bevy::ui_widgets::radio_self_update;
use bevy::{
    camera::primitives::CubemapLayout,
    color::palettes::css::{SILVER, YELLOW},
    feathers::{
        controls::{FeathersNumberInput, FeathersRadio, NumberInputPrecision, NumberInputValue},
        theme::UiTheme,
        FeathersPlugins,
    },
    input::mouse::AccumulatedMouseMotion,
    light::{DirectionalLightTexture, NotShadowCaster, PointLightTexture, SpotLightTexture},
    pbr::decal,
    prelude::*,
    render::renderer::{RenderAdapter, RenderDevice},
    ui::Checked,
    ui_widgets::ValueChange,
};
use light_consts::lux::{AMBIENT_DAYLIGHT, CLEAR_SUNRISE};
use number_input::number_input_f32;
use ops::{acos, cos, sin};
use radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/number_input.rs"]
mod number_input;

#[path = "../helpers/theme.rs"]
mod theme;

/// The speed at which the cube rotates, in radians per frame.
const CUBE_ROTATION_SPEED: f32 = 0.02;

/// The speed at which the selection can be moved, in spherical coordinate
/// radians per mouse unit.
const MOVE_SPEED: f32 = 0.008;

/// Various settings for the demo.
#[derive(Resource, Default)]
struct AppStatus {
    /// The object that will be moved, scaled, or rotated when the mouse is
    /// dragged.
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
    /// The spotlight, which uses a torch-like light texture
    SpotLight,
    /// The point light, which uses a light texture cubemap constructed from the faces mesh
    PointLight,
    /// The directional light, which uses a caustic-like texture
    DirectionalLight,
}

impl fmt::Display for Selection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Selection::Camera => f.write_str("camera"),
            Selection::SpotLight => f.write_str("spotlight"),
            Selection::PointLight => f.write_str("point light"),
            Selection::DirectionalLight => f.write_str("directional light"),
        }
    }
}

/// Indicates which aspect of the light the `FeathersNumberInput` in the app influences.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum AppNumberInput {
    /// The mouse scales the current selection.
    ///
    /// This only applies to lights, not cameras.
    #[default]
    Scale,
    /// The mouse rotates the current selection around its local Z axis.
    ///
    /// This only applies to lights, not cameras.
    Roll,
}

/// A component that stores the base transformation scale of a light.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
struct BaseScale(Vec3);

/// A marker component for the help text in the top left corner of the window.
#[derive(Clone, Copy, Component)]
struct HelpText;

/// Entry point.
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Light Textures Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .init_resource::<AppStatus>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, hide_shadows)
        .add_observer(handle_selection_change)
        .add_observer(handle_visibility_change)
        .add_observer(radio_self_update)
        .add_observer(handle_value_change_number_input)
        .add_observer(handle_drag_as_movement)
        .add_systems(Update, update_directional_light)
        .add_systems(Update, update_help_text)
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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Error out if clustered decals (and so light textures) aren't supported on the current platform.
    if !decal::clustered::clustered_decals_are_usable(&render_device, &render_adapter) {
        error!("Light textures aren't usable on this platform.");
        commands.write_message(AppExit::error());
    }

    spawn_cubes(&mut commands, &mut meshes, &mut materials);
    spawn_camera(&mut commands);
    spawn_light(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
    spawn_help_text(&mut commands, &app_status);
    spawn_light_textures(&mut commands, &asset_server, &mut meshes, &mut materials);
}

#[derive(Component)]
struct Rotate;

/// Spawns the cube onto which the decals are projected.
fn spawn_cubes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Rotate the cube a bit just to make it more interesting.
    let mut transform = Transform::IDENTITY;
    transform.rotate_y(FRAC_PI_3);

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 3.0, 3.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: SILVER.into(),
            ..default()
        })),
        transform,
        Rotate,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(-13.0, -13.0, -13.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: SILVER.into(),
            ..default()
        })),
        transform,
    ));
}

/// Spawns the directional light.
fn spawn_light(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        Visibility::Hidden,
        Transform::from_xyz(8.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        BaseScale(Vec3::ONE),
        Selection::DirectionalLight,
        children![(
            DirectionalLight {
                illuminance: AMBIENT_DAYLIGHT,
                ..default()
            },
            DirectionalLightTexture {
                image: asset_server.load("lightmaps/caustic_directional_texture.png"),
                tiled: true,
            },
            Visibility::Visible,
        )],
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

fn spawn_light_textures(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 1.0, 0.8),
            intensity: 10e6,
            outer_angle: 0.25,
            inner_angle: 0.25,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(Vec3::new(6.0, 1.0, 2.0)).looking_at(Vec3::ZERO, Vec3::Y),
        BaseScale(Vec3::ONE),
        SpotLightTexture {
            image: asset_server.load("lightmaps/torch_spotlight_texture.png"),
        },
        Visibility::Inherited,
        Selection::SpotLight,
    ));

    commands.spawn((
        Visibility::Hidden,
        Transform::from_translation(Vec3::new(0.0, 1.8, 0.01)).with_scale(Vec3::splat(0.25)),
        BaseScale(Vec3::splat(0.25)),
        Selection::PointLight,
        children![
            WorldAssetRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Faces/faces.glb")),
            ),
            (
                Mesh3d(meshes.add(Sphere::new(1.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    emissive: Color::srgb(0.0, 0.0, 300.0).to_linear(),
                    ..default()
                })),
            ),
            (
                PointLight {
                    color: Color::srgb(0.0, 0.0, 1.0),
                    intensity: 1e6,
                    shadow_maps_enabled: true,
                    ..default()
                },
                PointLightTexture {
                    image: asset_server.load("lightmaps/faces_pointlight_texture_blurred.png"),
                    cubemap_layout: CubemapLayout::CrossVertical,
                },
            )
        ],
    ));
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Children [
            feathers_option_buttons(
                "Drag to Move",
                &[
                    (Selection::Camera, "Camera"),
                    (Selection::SpotLight, "Spotlight"),
                    (Selection::PointLight, "Point Light"),
                    (Selection::DirectionalLight, "Directional Light"),
                ],
            ),

            // Camera's visibility cannot be toggled.
            Visibility::Hidden
            feathers_option_buttons(
                "Visibility",
                &[
                    (Visibility::Inherited, "Show"),
                    (Visibility::Hidden, "Hide"),
                ],
            ),

            // The number inputs start off hidden because Camera is selected first.
            Visibility::Hidden
            number_input_f32("Scale Multiplier", Some(AppNumberInput::Scale), 1.0, NumberInputPrecision(2), 0.01..5.)
            ,

            Visibility::Hidden
            number_input_f32("Roll (-π to π)", Some(AppNumberInput::Roll), 0.0, NumberInputPrecision(2), -PI..PI)
            ,
        ]
    });
}

/// Observer that handles changes to number inputs.
/// The number inputs affect the scale or rotation of the currently selected light, if any.
fn handle_value_change_number_input(
    value_change: On<ValueChange<f32>>,
    mut commands: Commands,
    number_input_q: Query<&AppNumberInput, With<FeathersNumberInput>>,
    app_status: ResMut<AppStatus>,
    mut selections: Query<(
        &mut Transform,
        Option<&mut SpotLight>,
        &BaseScale,
        &Selection,
    )>,
) {
    if app_status.selection == Selection::Camera {
        return;
    }
    if let Ok(app_number_input) = number_input_q.get(value_change.source) {
        for (mut transform, spotlight_option, base_scale, selection) in &mut selections {
            if app_status.selection != *selection {
                continue;
            }
            match app_number_input {
                AppNumberInput::Scale => {
                    transform.scale = base_scale.0 * value_change.value;

                    if let Some(mut spotlight) = spotlight_option {
                        // The spotlight's base outer and inner angle are 0.25.
                        spotlight.outer_angle = (0.25 * value_change.value).clamp(0.01, FRAC_PI_4);
                        spotlight.inner_angle = spotlight.outer_angle;
                    }
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
    new_value_query: Query<&RadioButtonOptionValue<Selection>>,
    mut app_status: ResMut<AppStatus>,
    mut commands: Commands,
    selections: Query<(&Transform, &BaseScale, &Visibility, &Selection)>,
    visibility_radio: Query<
        (Entity, &ChildOf, &RadioButtonOptionValue<Visibility>),
        With<FeathersRadio>,
    >,
    number_inputs: Query<(Entity, &ChildOf, &AppNumberInput)>,
) {
    let Ok(RadioButtonOptionValue(selection)) = new_value_query.get(event.value) else {
        return;
    };
    app_status.selection = *selection;

    // Update the visibility of the visibility-setting radio group.
    if app_status.selection == Selection::Camera
        && let Some((_, child_of, _)) = visibility_radio.iter().next()
    {
        commands
            .entity(child_of.parent())
            .insert(Visibility::Hidden);
    } else if let Some((_, child_of, _)) = visibility_radio.iter().next() {
        commands
            .entity(child_of.parent())
            .insert(Visibility::Inherited);

        // Add the `Checked` component to the correct visibility option for this selected light.
        for (_transform, _base_scale, selected_visibility, _selection) in selections
            .iter()
            .filter(|&(_, _, _, selection)| *selection == app_status.selection)
        {
            for (entity, _, visibility_option_value) in visibility_radio.iter() {
                if visibility_option_value.0 == *selected_visibility {
                    commands.entity(entity).insert(Checked);
                } else {
                    commands.entity(entity).remove::<Checked>();
                }
            }
        }
    }

    // Update the visibility of the scale and roll number inputs so that they aren't visible
    // if the camera is selected.
    for (input_entity, child_of, app_number_input) in number_inputs.iter() {
        match app_status.selection {
            Selection::Camera => {
                commands
                    .entity(child_of.parent())
                    .insert(Visibility::Hidden);
            }
            _ => {
                commands
                    .entity(child_of.parent())
                    .insert(Visibility::Inherited);

                // Update the input values to the correct ones for this light.
                for (transform, base_scale, _, _) in selections
                    .iter()
                    .filter(|&(_, _, _, selection)| *selection == app_status.selection)
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

/// Handles requests from the user to change the selected object to control and expose
/// the appropriate controls.
/// The `radio_self_update` observer handles setting the `Checked` state on the radio buttons.
fn handle_visibility_change(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&RadioButtonOptionValue<Visibility>>,
    app_status: Res<AppStatus>,
    mut visibility_q: Query<(&mut Visibility, &Selection)>,
) {
    let Ok(RadioButtonOptionValue(new_visibility)) = new_value_query.get(event.value) else {
        return;
    };

    for (mut visibility, selection) in visibility_q.iter_mut() {
        if *selection == app_status.selection {
            *visibility = *new_visibility;
        }
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

/// Draws the outlines that show the bounds of the spotlight.
fn draw_gizmos(mut gizmos: Gizmos, spotlight: Query<(&GlobalTransform, &SpotLight, &Visibility)>) {
    if let Ok((global_transform, spotlight, visibility)) = spotlight.single()
        && visibility != Visibility::Hidden
    {
        gizmos.primitive_3d(
            &Cone::new(7.0 * spotlight.outer_angle, 7.0),
            Isometry3d {
                rotation: global_transform.rotation() * Quat::from_rotation_x(FRAC_PI_2),
                translation: global_transform.translation_vec3a() * 0.5,
            },
            YELLOW,
        );
    }
}

/// Rotates the cube a bit every frame.
fn rotate_cube(mut meshes: Query<&mut Transform, With<Rotate>>) {
    for mut transform in &mut meshes {
        transform.rotate_y(CUBE_ROTATION_SPEED);
    }
}

/// Hide shadows on all meshes except the main cube
fn hide_shadows(
    mut commands: Commands,
    meshes: Query<Entity, (With<Mesh3d>, Without<NotShadowCaster>, Without<Rotate>)>,
) {
    for ent in &meshes {
        commands.entity(ent).insert(NotShadowCaster);
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

        // use simple movement for the point light
        if *selection == Selection::PointLight {
            transform.translation +=
                (mouse_motion.delta * Vec2::new(1.0, -1.0) * MOVE_SPEED).extend(0.0);
            return;
        }

        let position = transform.translation;

        // Convert to spherical coordinates.
        let radius = position.length();
        let mut theta = acos(position.y / radius);
        let mut phi = position.z.signum() * acos(position.x * position.xz().length_recip());

        // Camera movement is the inverse of object movement.
        let (phi_factor, theta_factor) = match *selection {
            Selection::Camera => (1.0, -1.0),
            _ => (-1.0, 1.0),
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
/// selection and drag mode.
fn update_help_text(mut help_text: Query<&mut Text, With<HelpText>>, app_status: Res<AppStatus>) {
    for mut text in &mut help_text {
        text.0 = create_help_string(&app_status);
    }
}

fn update_directional_light(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selections: Query<(&Selection, &Visibility)>,
    mut light: Query<(
        Entity,
        &mut DirectionalLight,
        Option<&DirectionalLightTexture>,
    )>,
) {
    let directional_visible = selections
        .iter()
        .filter(|(selection, _)| **selection == Selection::DirectionalLight)
        .any(|(_, visibility)| visibility != Visibility::Hidden);
    let any_texture_light_visible = selections
        .iter()
        .filter(|(selection, _)| {
            **selection == Selection::PointLight || **selection == Selection::SpotLight
        })
        .any(|(_, visibility)| visibility != Visibility::Hidden);

    let (entity, mut light, maybe_texture) = light
        .single_mut()
        .expect("there should be a single directional light");

    if directional_visible {
        light.illuminance = AMBIENT_DAYLIGHT;
        if maybe_texture.is_none() {
            commands.entity(entity).insert(DirectionalLightTexture {
                image: asset_server.load("lightmaps/caustic_directional_texture.png"),
                tiled: true,
            });
        }
    } else if any_texture_light_visible {
        light.illuminance = CLEAR_SUNRISE;
        if maybe_texture.is_some() {
            commands.entity(entity).remove::<DirectionalLightTexture>();
        }
    } else {
        light.illuminance = AMBIENT_DAYLIGHT;
        if maybe_texture.is_some() {
            commands.entity(entity).remove::<DirectionalLightTexture>();
        }
    }
}
