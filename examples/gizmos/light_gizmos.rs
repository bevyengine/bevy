//! This example demonstrates how to visualize lights properties through the gizmo API.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    color::palettes::css::{DARK_CYAN, GOLD, GRAY, ORANGE, PURPLE},
    feathers::{
        controls::{FeathersCheckbox, FeathersSlider},
        display::label,
        theme::UiTheme,
        FeathersPlugins,
    },
    prelude::*,
    ui_widgets::{
        checkbox_self_update, radio_self_update, SliderPrecision, SliderStep, SliderValue,
        ValueChange,
    },
};

use checkbox::feathers_option_checkbox;
use radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};

#[path = "../helpers/checkbox.rs"]
mod checkbox;

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_observer(update_radio_button)
        .add_observer(handle_value_change_checkbox)
        .add_observer(checkbox_self_update)
        .add_observer(radio_self_update)
        .run();
}

fn gizmo_color_text(config: &LightGizmoConfigGroup) -> String {
    match config.color {
        LightGizmoColor::Manual(color) => format!("Manual {}", Srgba::from(color).to_hex()),
        LightGizmoColor::Varied => "Random from entity".to_owned(),
        LightGizmoColor::MatchLightColor => "Match light color".to_owned(),
        LightGizmoColor::ByLightType => {
            format!(
                "Point {}, Spot {}, Directional {}, Rect {}",
                Srgba::from(config.point_light_color).to_hex(),
                Srgba::from(config.spot_light_color).to_hex(),
                Srgba::from(config.directional_light_color).to_hex(),
                Srgba::from(config.rect_light_color).to_hex()
            )
        }
    }
}

#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum CheckboxInput {
    #[default]
    GizmoDepthMode,
    HideLightGizmo,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let (_, light_config) = config_store.config_mut::<LightGizmoConfigGroup>();
    light_config.draw_all = true;

    let label_for = |color: LightGizmoColor| -> String {
        gizmo_color_text(&LightGizmoConfigGroup {
            color,
            ..light_config.clone()
        })
    };

    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Children [
            (
                feathers_option_checkbox("Toggle drawing gizmos on top of everything else in the scene", Some(CheckboxInput::GizmoDepthMode))
            ),
            (
                feathers_option_checkbox("Hide light gizmos", Some(CheckboxInput::HideLightGizmo))
            ),
            (
                feathers_option_buttons("Gizmo color mode",
                    &[
                        (LightGizmoColor::Varied, label_for(LightGizmoColor::Varied).as_str()),
                        (LightGizmoColor::MatchLightColor, label_for(LightGizmoColor::MatchLightColor).as_str()),
                        (LightGizmoColor::ByLightType, label_for(LightGizmoColor::ByLightType).as_str()),
                        (LightGizmoColor::Manual(GRAY.into()), label_for(LightGizmoColor::Manual(GRAY.into())).as_str()),
                    ], 1)
            ),
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(4)
                }
                Children [
                    (
                        label("Change the line width of the gizmos")
                    ),
                    (
                        @FeathersSlider{
                            @max: 50.,
                            @min: 0.,
                        }
                        SliderValue(2.)
                        SliderPrecision(2)
                        SliderStep(1.)
                        on(slider_update)
                    )
                ]
            )
        ]
    });
    // Circular base.
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
    ));

    // Cubes.
    {
        let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let material = materials.add(Color::srgb_u8(124, 144, 255));
        for x in [-2.0, 0.0, 2.0] {
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(x, 0.5, 0.0),
            ));
        }
    }

    // Lights.
    {
        commands.spawn((
            PointLight {
                shadow_maps_enabled: true,
                range: 2.0,
                color: DARK_CYAN.into(),
                ..default()
            },
            Transform::from_xyz(0.0, 1.5, 0.0),
        ));
        commands.spawn((
            SpotLight {
                shadow_maps_enabled: true,
                range: 3.5,
                color: PURPLE.into(),
                outer_angle: PI / 4.0,
                inner_angle: PI / 4.0 * 0.8,
                ..default()
            },
            Transform::from_xyz(4.0, 2.0, 0.0).looking_at(Vec3::X * 1.5, Vec3::Y),
        ));
        commands.spawn((
            DirectionalLight {
                color: GOLD.into(),
                illuminance: DirectionalLight::default().illuminance * 0.05,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(-4.0, 2.0, 0.0).looking_at(Vec3::NEG_X * 1.5, Vec3::Y),
        ));
        commands.spawn((
            RectLight {
                color: ORANGE.into(),
                intensity: 200_000.0,
                width: 1.5,
                height: 0.8,
                range: 20.0,
            },
            Transform::from_xyz(0.0, 3.0, -3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }

    // Camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn rotate_camera(mut transform: Single<&mut Transform, With<Camera>>, time: Res<Time>) {
    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_secs() / 2.));
}

fn handle_value_change_checkbox(
    event: On<ValueChange<bool>>,
    mut config_store: ResMut<GizmoConfigStore>,
    checkbox_input_q: Query<&CheckboxInput, With<FeathersCheckbox>>,
) {
    let (config, _) = config_store.config_mut::<LightGizmoConfigGroup>();
    if let Ok(checkbox_input) = checkbox_input_q.get(event.source) {
        match checkbox_input {
            CheckboxInput::GizmoDepthMode => {
                config.depth_bias = if event.value { -1. } else { 0. };
            }
            CheckboxInput::HideLightGizmo => {
                config.enabled = !event.value;
            }
        }
    }
}

fn update_radio_button(
    event: On<ValueChange<Entity>>,
    color_mode_query: Query<&RadioButtonOptionValue<LightGizmoColor>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let (_, light_config) = config_store.config_mut::<LightGizmoConfigGroup>();
    if let Ok(RadioButtonOptionValue(color)) = color_mode_query.get(event.value) {
        light_config.color = *color;
    }
}

fn slider_update(
    value_change: On<ValueChange<f32>>,
    mut config_store: ResMut<GizmoConfigStore>,
    mut commands: Commands,
) {
    commands
        .entity(value_change.source)
        .insert(SliderValue(value_change.value));

    let (config, _) = config_store.config_mut::<LightGizmoConfigGroup>();
    config.line.width = value_change.value;
}
