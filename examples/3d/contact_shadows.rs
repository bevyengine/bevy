//! Demonstrates contact shadows, also known as screen-space shadows.

use crate::widgets::{RadioButton, RadioButtonText, WidgetClickEvent, WidgetClickSender};
use bevy::core_pipeline::Skybox;
use bevy::post_process::bloom::Bloom;
use bevy::{ecs::message::MessageReader, pbr::ContactShadows, prelude::*};
use bevy_render::view::Hdr;

#[path = "../helpers/widgets.rs"]
mod widgets;

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum ContactShadowState {
    #[default]
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum ShadowMaps {
    #[default]
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum LightRotation {
    #[default]
    Stationary,
    Rotating,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum LightType {
    #[default]
    Directional,
    Point,
    Spot,
}

/// Each example setting that can be toggled in the UI.
#[derive(Clone, Copy, PartialEq)]
enum ExampleSetting {
    ContactShadows(ContactShadowState),
    ShadowMaps(ShadowMaps),
    LightRotation(LightRotation),
    LightType(LightType),
}

const LIGHT_ROTATION_SPEED: f32 = 0.005;

#[derive(Resource, Default)]
struct AppStatus {
    contact_shadows: ContactShadowState,
    shadow_maps: ShadowMaps,
    light_rotation: LightRotation,
    light_type: LightType,
}

#[derive(Component)]
struct LightContainer;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Contact Shadows Example".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<AppStatus>()
        .add_message::<WidgetClickEvent<ExampleSetting>>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_light)
        .add_systems(
            Update,
            (
                widgets::handle_ui_interactions::<ExampleSetting>,
                update_radio_buttons.after(widgets::handle_ui_interactions::<ExampleSetting>),
                handle_setting_change.after(widgets::handle_ui_interactions::<ExampleSetting>),
            ),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.45, 0.6, 0.45).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
        ContactShadows::default(),
        Bloom::default(),
        Hdr::default(),
        Skybox {
            brightness: 500.0,
            image: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 300.0,
            ..default()
        },
        AmbientLight {
            brightness: 0.0,
            ..default()
        },
    ));

    let directional_light = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            Visibility::Visible,
        ))
        .id();

    let point_light = commands
        .spawn((
            PointLight {
                shadows_enabled: true,
                intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT * 0.25,
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();

    let spot_light = commands
        .spawn((
            SpotLight {
                shadows_enabled: true,
                intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT * 0.25,
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();

    commands
        .spawn((
            Transform::from_xyz(-0.8, 2.0, 1.2).looking_at(Vec3::ZERO, Vec3::Y),
            Visibility::default(),
            LightContainer,
        ))
        .add_child(directional_light)
        .add_child(point_light)
        .add_child(spot_light);

    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
    )));

    spawn_buttons(&mut commands);
}

fn rotate_light(
    mut lights: Query<&mut Transform, With<LightContainer>>,
    app_status: Res<AppStatus>,
) {
    if app_status.light_rotation != LightRotation::Rotating {
        return;
    }

    for mut transform in lights.iter_mut() {
        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(LIGHT_ROTATION_SPEED));
    }
}

fn spawn_buttons(commands: &mut Commands) {
    commands.spawn((
        widgets::main_ui_node(),
        children![
            widgets::option_buttons(
                "Contact Shadows",
                &[
                    (
                        ExampleSetting::ContactShadows(ContactShadowState::Enabled),
                        "On"
                    ),
                    (
                        ExampleSetting::ContactShadows(ContactShadowState::Disabled),
                        "Off"
                    ),
                ],
            ),
            widgets::option_buttons(
                "Shadow Maps",
                &[
                    (ExampleSetting::ShadowMaps(ShadowMaps::Enabled), "On"),
                    (ExampleSetting::ShadowMaps(ShadowMaps::Disabled), "Off"),
                ],
            ),
            widgets::option_buttons(
                "Light Rotation",
                &[
                    (ExampleSetting::LightRotation(LightRotation::Rotating), "On"),
                    (
                        ExampleSetting::LightRotation(LightRotation::Stationary),
                        "Off"
                    ),
                ],
            ),
            widgets::option_buttons(
                "Light Type",
                &[
                    (
                        ExampleSetting::LightType(LightType::Directional),
                        "Directional"
                    ),
                    (ExampleSetting::LightType(LightType::Point), "Point"),
                    (ExampleSetting::LightType(LightType::Spot), "Spot"),
                ],
            ),
        ],
    ));
}

fn update_radio_buttons(
    mut widgets: Query<
        (
            Entity,
            Option<&mut BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<ExampleSetting>,
        ),
        Or<(With<RadioButton>, With<RadioButtonText>)>,
    >,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
) {
    for (entity, background_color, has_text, sender) in widgets.iter_mut() {
        let selected = match **sender {
            ExampleSetting::ContactShadows(value) => value == app_status.contact_shadows,
            ExampleSetting::ShadowMaps(value) => value == app_status.shadow_maps,
            ExampleSetting::LightRotation(value) => value == app_status.light_rotation,
            ExampleSetting::LightType(value) => value == app_status.light_type,
        };

        if let Some(mut background_color) = background_color {
            widgets::update_ui_radio_button(&mut background_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

fn handle_setting_change(
    mut cameras: Query<&mut ContactShadows>,
    mut lights: Query<
        (
            &mut Visibility,
            Option<&mut DirectionalLight>,
            Option<&mut PointLight>,
            Option<&mut SpotLight>,
        ),
        Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>,
    >,
    mut events: MessageReader<WidgetClickEvent<ExampleSetting>>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in events.read() {
        match **event {
            ExampleSetting::ContactShadows(value) => {
                app_status.contact_shadows = value;
                for mut contact_shadows in cameras.iter_mut() {
                    contact_shadows.linear_steps = if value == ContactShadowState::Enabled {
                        16
                    } else {
                        0
                    };
                }
            }
            ExampleSetting::ShadowMaps(value) => {
                app_status.shadow_maps = value;
                for (_, maybe_directional_light, maybe_point_light, maybe_spot_light) in
                    lights.iter_mut()
                {
                    if let Some(mut directional_light) = maybe_directional_light {
                        directional_light.shadows_enabled = value == ShadowMaps::Enabled;
                    }
                    if let Some(mut point_light) = maybe_point_light {
                        point_light.shadows_enabled = value == ShadowMaps::Enabled;
                    }
                    if let Some(mut spot_light) = maybe_spot_light {
                        spot_light.shadows_enabled = value == ShadowMaps::Enabled;
                    }
                }
            }
            ExampleSetting::LightRotation(value) => {
                app_status.light_rotation = value;
            }
            ExampleSetting::LightType(value) => {
                app_status.light_type = value;
                for (
                    mut visibility,
                    maybe_directional_light,
                    maybe_point_light,
                    maybe_spot_light,
                ) in lights.iter_mut()
                {
                    let is_visible = match value {
                        LightType::Directional => maybe_directional_light.is_some(),
                        LightType::Point => maybe_point_light.is_some(),
                        LightType::Spot => maybe_spot_light.is_some(),
                    };
                    *visibility = if is_visible {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
    }
}
