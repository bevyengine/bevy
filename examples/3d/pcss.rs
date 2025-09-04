//! Demonstrates percentage-closer soft shadows (PCSS).

use std::f32::consts::PI;

use bevy::{
    anti_aliasing::taa::TemporalAntiAliasing,
    camera::{
        primitives::{CubemapFrusta, Frustum},
        visibility::{CubemapVisibleEntities, VisibleMeshEntities},
    },
    core_pipeline::{
        prepass::{DepthPrepass, MotionVectorPrepass},
        Skybox,
    },
    light::ShadowFilteringMethod,
    math::vec3,
    prelude::*,
    render::camera::TemporalJitter,
};

use crate::widgets::{RadioButton, RadioButtonText, WidgetClickEvent, WidgetClickSender};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// The size of the light, which affects the size of the penumbras.
const LIGHT_RADIUS: f32 = 10.0;

/// The intensity of the point and spot lights.
const POINT_LIGHT_INTENSITY: f32 = 1_000_000_000.0;

/// The range in meters of the point and spot lights.
const POINT_LIGHT_RANGE: f32 = 110.0;

/// The depth bias for directional and spot lights. This value is set higher
/// than the default to avoid shadow acne.
const DIRECTIONAL_SHADOW_DEPTH_BIAS: f32 = 0.20;

/// The depth bias for point lights. This value is set higher than the default to
/// avoid shadow acne.
///
/// Unfortunately, there is a bit of Peter Panning with this value, because of
/// the distance and angle of the light. This can't be helped in this scene
/// without increasing the shadow map size beyond reasonable limits.
const POINT_SHADOW_DEPTH_BIAS: f32 = 0.35;

/// The near Z value for the shadow map, in meters. This is set higher than the
/// default in order to achieve greater resolution in the shadow map for point
/// and spot lights.
const SHADOW_MAP_NEAR_Z: f32 = 50.0;

/// The current application settings (light type, shadow filter, and the status
/// of PCSS).
#[derive(Resource)]
struct AppStatus {
    /// The type of light presently in the scene: either directional or point.
    light_type: LightType,
    /// The type of shadow filter: Gaussian or temporal.
    shadow_filter: ShadowFilter,
    /// Whether soft shadows are enabled.
    soft_shadows: bool,
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            light_type: default(),
            shadow_filter: default(),
            soft_shadows: true,
        }
    }
}

/// The type of light presently in the scene: directional, point, or spot.
#[derive(Clone, Copy, Default, PartialEq)]
enum LightType {
    /// A directional light, with a cascaded shadow map.
    #[default]
    Directional,
    /// A point light, with a cube shadow map.
    Point,
    /// A spot light, with a cube shadow map.
    Spot,
}

/// The type of shadow filter.
///
/// Generally, `Gaussian` is preferred when temporal antialiasing isn't in use,
/// while `Temporal` is preferred when TAA is in use. In this example, this
/// setting also turns TAA on and off.
#[derive(Clone, Copy, Default, PartialEq)]
enum ShadowFilter {
    /// The non-temporal Gaussian filter (Castano '13 for directional lights, an
    /// analogous alternative for point and spot lights).
    #[default]
    NonTemporal,
    /// The temporal Gaussian filter (Jimenez '14 for directional lights, an
    /// analogous alternative for point and spot lights).
    Temporal,
}

/// Each example setting that can be toggled in the UI.
#[derive(Clone, Copy, PartialEq)]
enum AppSetting {
    /// The type of light presently in the scene: directional, point, or spot.
    LightType(LightType),
    /// The type of shadow filter.
    ShadowFilter(ShadowFilter),
    /// Whether PCSS is enabled or disabled.
    SoftShadows(bool),
}

/// The example application entry point.
fn main() {
    App::new()
        .init_resource::<AppStatus>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Percentage Closer Soft Shadows Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_event::<WidgetClickEvent<AppSetting>>()
        .add_systems(Startup, setup)
        .add_systems(Update, widgets::handle_ui_interactions::<AppSetting>)
        .add_systems(
            Update,
            update_radio_buttons.after(widgets::handle_ui_interactions::<AppSetting>),
        )
        .add_systems(
            Update,
            (
                handle_light_type_change,
                handle_shadow_filter_change,
                handle_pcss_toggle,
            )
                .after(widgets::handle_ui_interactions::<AppSetting>),
        )
        .run();
}

/// Creates all the objects in the scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_status: Res<AppStatus>) {
    spawn_camera(&mut commands, &asset_server);
    spawn_light(&mut commands, &app_status);
    spawn_gltf_scene(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
}

/// Spawns the camera, with the initial shadow filtering method.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(-12.912 * 0.7, 4.466 * 0.7, -10.624 * 0.7).with_rotation(
                Quat::from_euler(EulerRot::YXZ, -134.76 / 180.0 * PI, -0.175, 0.0),
            ),
        ))
        .insert(ShadowFilteringMethod::Gaussian)
        // `TemporalJitter` is needed for TAA. Note that it does nothing without
        // `TemporalAntiAliasSettings`.
        .insert(TemporalJitter::default())
        // We want MSAA off for TAA to work properly.
        .insert(Msaa::Off)
        // The depth prepass is needed for TAA.
        .insert(DepthPrepass)
        // The motion vector prepass is needed for TAA.
        .insert(MotionVectorPrepass)
        // Add a nice skybox.
        .insert(Skybox {
            image: asset_server.load("environment_maps/sky_skybox.ktx2"),
            brightness: 500.0,
            rotation: Quat::IDENTITY,
        });
}

/// Spawns the initial light.
fn spawn_light(commands: &mut Commands, app_status: &AppStatus) {
    // Because this light can become a directional light, point light, or spot
    // light depending on the settings, we add the union of the components
    // necessary for this light to behave as all three of those.
    commands
        .spawn((
            create_directional_light(app_status),
            Transform::from_rotation(Quat::from_array([
                0.6539259,
                -0.34646285,
                0.36505926,
                -0.5648683,
            ]))
            .with_translation(vec3(57.693, 34.334, -6.422)),
        ))
        // These two are needed for point lights.
        .insert(CubemapVisibleEntities::default())
        .insert(CubemapFrusta::default())
        // These two are needed for spot lights.
        .insert(VisibleMeshEntities::default())
        .insert(Frustum::default());
}

/// Loads and spawns the glTF palm tree scene.
fn spawn_gltf_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(SceneRoot(
        asset_server.load("models/PalmTree/PalmTree.gltf#Scene0"),
    ));
}

/// Spawns all the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    commands
        .spawn(widgets::main_ui_node())
        .with_children(|parent| {
            widgets::spawn_option_buttons(
                parent,
                "Light Type",
                &[
                    (AppSetting::LightType(LightType::Directional), "Directional"),
                    (AppSetting::LightType(LightType::Point), "Point"),
                    (AppSetting::LightType(LightType::Spot), "Spot"),
                ],
            );
            widgets::spawn_option_buttons(
                parent,
                "Shadow Filter",
                &[
                    (AppSetting::ShadowFilter(ShadowFilter::Temporal), "Temporal"),
                    (
                        AppSetting::ShadowFilter(ShadowFilter::NonTemporal),
                        "Non-Temporal",
                    ),
                ],
            );
            widgets::spawn_option_buttons(
                parent,
                "Soft Shadows",
                &[
                    (AppSetting::SoftShadows(true), "On"),
                    (AppSetting::SoftShadows(false), "Off"),
                ],
            );
        });
}

/// Updates the style of the radio buttons that enable and disable soft shadows
/// to reflect whether PCSS is enabled.
fn update_radio_buttons(
    mut widgets: Query<
        (
            Entity,
            Option<&mut BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<AppSetting>,
        ),
        Or<(With<RadioButton>, With<RadioButtonText>)>,
    >,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
) {
    for (entity, image, has_text, sender) in widgets.iter_mut() {
        let selected = match **sender {
            AppSetting::LightType(light_type) => light_type == app_status.light_type,
            AppSetting::ShadowFilter(shadow_filter) => shadow_filter == app_status.shadow_filter,
            AppSetting::SoftShadows(soft_shadows) => soft_shadows == app_status.soft_shadows,
        };

        if let Some(mut bg_color) = image {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

/// Handles requests from the user to change the type of light.
fn handle_light_type_change(
    mut commands: Commands,
    mut lights: Query<Entity, Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>>,
    mut events: EventReader<WidgetClickEvent<AppSetting>>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in events.read() {
        let AppSetting::LightType(light_type) = **event else {
            continue;
        };
        app_status.light_type = light_type;

        for light in lights.iter_mut() {
            let mut light_commands = commands.entity(light);
            light_commands
                .remove::<DirectionalLight>()
                .remove::<PointLight>()
                .remove::<SpotLight>();
            match light_type {
                LightType::Point => {
                    light_commands.insert(create_point_light(&app_status));
                }
                LightType::Spot => {
                    light_commands.insert(create_spot_light(&app_status));
                }
                LightType::Directional => {
                    light_commands.insert(create_directional_light(&app_status));
                }
            }
        }
    }
}

/// Handles requests from the user to change the shadow filter method.
///
/// This system is also responsible for enabling and disabling TAA as
/// appropriate.
fn handle_shadow_filter_change(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut ShadowFilteringMethod)>,
    mut events: EventReader<WidgetClickEvent<AppSetting>>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in events.read() {
        let AppSetting::ShadowFilter(shadow_filter) = **event else {
            continue;
        };
        app_status.shadow_filter = shadow_filter;

        for (camera, mut shadow_filtering_method) in cameras.iter_mut() {
            match shadow_filter {
                ShadowFilter::NonTemporal => {
                    *shadow_filtering_method = ShadowFilteringMethod::Gaussian;
                    commands.entity(camera).remove::<TemporalAntiAliasing>();
                }
                ShadowFilter::Temporal => {
                    *shadow_filtering_method = ShadowFilteringMethod::Temporal;
                    commands
                        .entity(camera)
                        .insert(TemporalAntiAliasing::default());
                }
            }
        }
    }
}

/// Handles requests from the user to toggle soft shadows on and off.
fn handle_pcss_toggle(
    mut lights: Query<AnyOf<(&mut DirectionalLight, &mut PointLight, &mut SpotLight)>>,
    mut events: EventReader<WidgetClickEvent<AppSetting>>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in events.read() {
        let AppSetting::SoftShadows(value) = **event else {
            continue;
        };
        app_status.soft_shadows = value;

        // Recreating the lights is the simplest way to toggle soft shadows.
        for (directional_light, point_light, spot_light) in lights.iter_mut() {
            if let Some(mut directional_light) = directional_light {
                *directional_light = create_directional_light(&app_status);
            }
            if let Some(mut point_light) = point_light {
                *point_light = create_point_light(&app_status);
            }
            if let Some(mut spot_light) = spot_light {
                *spot_light = create_spot_light(&app_status);
            }
        }
    }
}

/// Creates the [`DirectionalLight`] component with the appropriate settings.
fn create_directional_light(app_status: &AppStatus) -> DirectionalLight {
    DirectionalLight {
        shadows_enabled: true,
        soft_shadow_size: if app_status.soft_shadows {
            Some(LIGHT_RADIUS)
        } else {
            None
        },
        shadow_depth_bias: DIRECTIONAL_SHADOW_DEPTH_BIAS,
        ..default()
    }
}

/// Creates the [`PointLight`] component with the appropriate settings.
fn create_point_light(app_status: &AppStatus) -> PointLight {
    PointLight {
        intensity: POINT_LIGHT_INTENSITY,
        range: POINT_LIGHT_RANGE,
        shadows_enabled: true,
        radius: LIGHT_RADIUS,
        soft_shadows_enabled: app_status.soft_shadows,
        shadow_depth_bias: POINT_SHADOW_DEPTH_BIAS,
        shadow_map_near_z: SHADOW_MAP_NEAR_Z,
        ..default()
    }
}

/// Creates the [`SpotLight`] component with the appropriate settings.
fn create_spot_light(app_status: &AppStatus) -> SpotLight {
    SpotLight {
        intensity: POINT_LIGHT_INTENSITY,
        range: POINT_LIGHT_RANGE,
        radius: LIGHT_RADIUS,
        shadows_enabled: true,
        soft_shadows_enabled: app_status.soft_shadows,
        shadow_depth_bias: DIRECTIONAL_SHADOW_DEPTH_BIAS,
        shadow_map_near_z: SHADOW_MAP_NEAR_Z,
        ..default()
    }
}
