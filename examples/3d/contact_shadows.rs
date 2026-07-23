//! Demonstrates contact shadows, also known as screen-space shadows.

use crate::radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};
use crate::theme::basic_example_theme;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::Skybox;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy::{
    camera::Hdr,
    feathers::{theme::UiTheme, FeathersPlugins},
    light::NotShadowReceiver,
    pbr::ContactShadows,
    post_process::bloom::Bloom,
    prelude::*,
    ui_widgets::{radio_self_update, ValueChange},
};

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

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
    Stationary,
    #[default]
    Rotating,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum LightType {
    Directional,
    #[default]
    Point,
    Spot,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum ReceiveShadows {
    #[default]
    Enabled,
    Disabled,
}

/// Each example setting that can be toggled in the UI.
#[derive(Clone, Copy, PartialEq)]
enum ExampleSetting {
    ContactShadows(ContactShadowState),
    ShadowMaps(ShadowMaps),
    LightRotation(LightRotation),
    LightType(LightType),
    ReceiveShadows(ReceiveShadows),
}

impl Default for ExampleSetting {
    fn default() -> Self {
        Self::ContactShadows(ContactShadowState::default())
    }
}

const LIGHT_ROTATION_SPEED: f32 = 0.002;

#[derive(Resource, Default)]
struct AppStatus {
    contact_shadows: ContactShadowState,
    shadow_maps: ShadowMaps,
    light_rotation: LightRotation,
    light_type: LightType,
    receive_shadows: ReceiveShadows,
}

#[derive(Component)]
struct LightContainer;

#[derive(Component)]
struct GroundPlane;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Contact Shadows Example".into(),
                    ..default()
                }),
                ..default()
            }),
            MeshPickingPlugin,
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(basic_example_theme(Color::WHITE)))
        .init_resource::<AppStatus>()
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_light)
        .add_observer(handle_setting_change)
        .add_observer(radio_self_update)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_status: Res<AppStatus>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-0.8, 0.6, -0.8).looking_at(Vec3::new(0.0, 0.35, 0.0), Vec3::Y),
        ContactShadows::default(),
        TemporalAntiAliasing::default(), // Contact shadows and AO benefit from TAA
        // Everything past this point is extra to look pretty.
        Bloom::default(),
        Hdr,
        Skybox {
            brightness: 1000.0,
            image: Some(asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2")),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1000.0,
            ..default()
        },
        ScreenSpaceAmbientOcclusion::default(),
        Msaa::Off,
        Tonemapping::AcesFitted,
        MotionBlur {
            shutter_angle: 2.0, // This is really just for fun when spinning the model
            ..default()
        },
    ));

    let directional_light = commands
        .spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                contact_shadows_enabled: true,
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();

    let point_light = commands
        .spawn((
            PointLight {
                intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT * 0.4,
                shadow_maps_enabled: true,
                contact_shadows_enabled: true,
                ..default()
            },
            Visibility::Visible,
        ))
        .id();

    let spot_light = commands
        .spawn((
            SpotLight {
                intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT * 0.4,
                shadow_maps_enabled: true,
                contact_shadows_enabled: true,
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();

    commands
        .spawn((
            Transform::from_xyz(-0.8, 1.5, 1.2).looking_at(Vec3::ZERO, Vec3::Y),
            Visibility::default(),
            LightContainer,
        ))
        .add_child(directional_light)
        .add_child(point_light)
        .add_child(spot_light);

    commands
        .spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
            )),
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ))
        .observe(
            |event: On<Pointer<Drag>>,
             mut query: Query<&mut Transform, With<WorldAssetRoot>>,
             mut commands: Commands,
             mut window: Query<Entity, With<PrimaryWindow>>| {
                for mut transform in query.iter_mut() {
                    transform.rotate_y(event.delta.x * 0.01);
                }
                commands
                    .entity(window.single_mut().unwrap())
                    .insert(CursorIcon::System(SystemCursorIcon::Grabbing));
            },
        )
        .observe(
            |_: On<Pointer<Over>>,
             mut commands: Commands,
             mut window: Query<Entity, With<PrimaryWindow>>| {
                commands
                    .entity(window.single_mut().unwrap())
                    .insert(CursorIcon::System(SystemCursorIcon::Grab));
            },
        )
        .observe(
            |_: On<Pointer<Out>>,
             mut commands: Commands,
             mut window: Query<Entity, With<PrimaryWindow>>| {
                commands
                    .entity(window.single_mut().unwrap())
                    .insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        )
        .observe(
            |_: On<Pointer<DragEnd>>,
             mut commands: Commands,
             mut window: Query<Entity, With<PrimaryWindow>>| {
                commands
                    .entity(window.single_mut().unwrap())
                    .insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        );

    commands.spawn((
        Mesh3d(asset_server.add(Circle::default().mesh().into())),
        MeshMaterial3d(asset_server.add(StandardMaterial {
            base_color: Color::srgb(0.06, 0.06, 0.06),
            ..default()
        })),
        Transform::from_rotation(Quat::from_axis_angle(Vec3::X, -std::f32::consts::FRAC_PI_2)),
        GroundPlane,
    ));

    spawn_buttons(&mut commands, &app_status);

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            left: px(0.0),
            right: px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Text::new("Drag model to spin"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
        )],
    ));
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

fn spawn_buttons(commands: &mut Commands, app_status: &AppStatus) {
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Children [
            feathers_option_buttons(
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
                (app_status.contact_shadows == ContactShadowState::Enabled).then(|| 0).unwrap_or(1),
            ),
            feathers_option_buttons(
                "Shadow Maps",
                &[
                    (ExampleSetting::ShadowMaps(ShadowMaps::Enabled), "On"),
                    (ExampleSetting::ShadowMaps(ShadowMaps::Disabled), "Off"),
                ],
                (app_status.shadow_maps == ShadowMaps::Enabled).then(|| 0).unwrap_or(1),
            ),
            feathers_option_buttons(
                "Light Rotation",
                &[
                    (ExampleSetting::LightRotation(LightRotation::Rotating), "On"),
                    (
                        ExampleSetting::LightRotation(LightRotation::Stationary),
                        "Off"
                    ),
                ],
                (app_status.light_rotation == LightRotation::Rotating).then(|| 0).unwrap_or(1),
            ),
            feathers_option_buttons(
                "Light Type",
                &[
                    (
                        ExampleSetting::LightType(LightType::Directional),
                        "Directional"
                    ),
                    (ExampleSetting::LightType(LightType::Point), "Point"),
                    (ExampleSetting::LightType(LightType::Spot), "Spot"),
                ],
                {
                    match app_status.light_type {
                        LightType::Directional => 0,
                        LightType::Point => 1,
                        LightType::Spot => 2,
                    }
                },
            ),
            feathers_option_buttons(
                "Receive Shadows",
                &[
                    (
                        ExampleSetting::ReceiveShadows(ReceiveShadows::Enabled),
                        "On"
                    ),
                    (
                        ExampleSetting::ReceiveShadows(ReceiveShadows::Disabled),
                        "Off"
                    ),
                ],
                (app_status.receive_shadows == ReceiveShadows::Enabled).then(|| 0).unwrap_or(1),
            ),
        ]
    });
}

fn handle_setting_change(
    event: On<ValueChange<Entity>>,
    new_value_q: Query<&RadioButtonOptionValue<ExampleSetting>>,
    mut lights: Query<
        (
            &mut Visibility,
            Option<&mut DirectionalLight>,
            Option<&mut PointLight>,
            Option<&mut SpotLight>,
        ),
        Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>,
    >,
    mut ground_plane: Query<Entity, With<GroundPlane>>,
    mut app_status: ResMut<AppStatus>,
    mut commands: Commands,
) {
    let Ok(RadioButtonOptionValue(setting)) = new_value_q.get(event.value) else {
        return;
    };

    match *setting {
        ExampleSetting::ContactShadows(value) => {
            app_status.contact_shadows = value;
            for (_, maybe_directional_light, maybe_point_light, maybe_spot_light) in
                lights.iter_mut()
            {
                if let Some(mut directional_light) = maybe_directional_light {
                    directional_light.contact_shadows_enabled =
                        value == ContactShadowState::Enabled;
                }
                if let Some(mut point_light) = maybe_point_light {
                    point_light.contact_shadows_enabled = value == ContactShadowState::Enabled;
                }
                if let Some(mut spot_light) = maybe_spot_light {
                    spot_light.contact_shadows_enabled = value == ContactShadowState::Enabled;
                }
            }
        }
        ExampleSetting::ShadowMaps(value) => {
            app_status.shadow_maps = value;
            for (_, maybe_directional_light, maybe_point_light, maybe_spot_light) in
                lights.iter_mut()
            {
                if let Some(mut directional_light) = maybe_directional_light {
                    directional_light.shadow_maps_enabled = value == ShadowMaps::Enabled;
                }
                if let Some(mut point_light) = maybe_point_light {
                    point_light.shadow_maps_enabled = value == ShadowMaps::Enabled;
                }
                if let Some(mut spot_light) = maybe_spot_light {
                    spot_light.shadow_maps_enabled = value == ShadowMaps::Enabled;
                }
            }
        }
        ExampleSetting::LightRotation(value) => {
            app_status.light_rotation = value;
        }
        ExampleSetting::LightType(value) => {
            app_status.light_type = value;
            for (mut visibility, maybe_directional_light, maybe_point_light, maybe_spot_light) in
                lights.iter_mut()
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
        ExampleSetting::ReceiveShadows(value) => {
            app_status.receive_shadows = value;
            for entity in ground_plane.iter_mut() {
                match value {
                    ReceiveShadows::Enabled => {
                        commands.entity(entity).remove::<NotShadowReceiver>();
                    }
                    ReceiveShadows::Disabled => {
                        commands.entity(entity).insert(NotShadowReceiver);
                    }
                }
            }
        }
    }
}
