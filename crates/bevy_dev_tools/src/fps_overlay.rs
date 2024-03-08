use bevy_app::{Plugin, Startup, Update};
use bevy_asset::AssetServer;
use bevy_color::Color;
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    component::Component,
    query::With,
    schedule::{common_conditions::resource_changed, IntoSystemConfigs},
    system::{Commands, Query, Res, Resource},
};
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_render::view::Visibility;
use bevy_text::{Text, TextSection, TextStyle};
use bevy_ui::node_bundles::TextBundle;

#[derive(Default)]
/// A plugin that adds an FPS overlay to the Bevy application.
pub struct FpsOverlayPlugin {
    /// Starting configuration of overlay, this can be later be changed through `[FpsOverlayConfig]` resource.
    pub config: FpsOverlayConfig,
}

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }
        app.insert_resource(self.config.clone())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    customize_text.run_if(resource_changed::<FpsOverlayConfig>),
                    update_text,
                    toggle_overlay,
                ),
            );
    }
}

#[derive(Resource, Clone)]
/// Configuration options for the FPS overlay.
pub struct FpsOverlayConfig {
    /// File path for the font file. If set to `None`,the default font is used.
    /// Note: The overlay won't be visible if you set it to `None` and run without `default_font` feature.
    pub font_path: Option<String>,
    /// Size of the overlay text.
    pub font_size: f32,
    /// Color of the overlay text.
    pub font_color: Color,
    /// Keybind for toggling on/off the overlay.
    pub keybind: KeyCode,
}

impl Default for FpsOverlayConfig {
    fn default() -> Self {
        FpsOverlayConfig {
            font_path: None,
            font_size: 32.0,
            font_color: Color::WHITE,
            keybind: KeyCode::F1,
        }
    }
}

#[derive(Component)]
struct FpsText;

fn setup(
    mut commands: Commands,
    overlay_config: Res<FpsOverlayConfig>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "FPS: ",
                if let Some(font_path) = &overlay_config.font_path {
                    TextStyle {
                        font_size: overlay_config.font_size,
                        color: overlay_config.font_color,
                        font: asset_server.load(font_path),
                    }
                } else {
                    TextStyle {
                        font_size: overlay_config.font_size,
                        color: overlay_config.font_color,
                        ..Default::default()
                    }
                },
            ),
            TextSection::from_style(if let Some(font_path) = &overlay_config.font_path {
                TextStyle {
                    font_size: overlay_config.font_size,
                    color: overlay_config.font_color,
                    font: asset_server.load(font_path),
                }
            } else {
                TextStyle {
                    font_size: overlay_config.font_size,
                    color: overlay_config.font_color,
                    ..Default::default()
                }
            }),
        ]),
        FpsText,
    ));
}

fn update_text(
    diagnostic: Res<DiagnosticsStore>,
    mut query: Query<(&mut Text, &Visibility), With<FpsText>>,
) {
    for (mut text, visibility) in &mut query {
        if let Visibility::Hidden = *visibility {
            return;
        }
        if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[1].value = format!("{value:.2}");
            }
        }
    }
}

fn customize_text(
    overlay_config: Res<FpsOverlayConfig>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    for mut text in &mut query {
        for section in text.sections.iter_mut() {
            section.style = if let Some(font_path) = &overlay_config.font_path {
                TextStyle {
                    font_size: overlay_config.font_size,
                    color: overlay_config.font_color,
                    font: asset_server.load(font_path),
                }
            } else {
                TextStyle {
                    font_size: overlay_config.font_size,
                    color: overlay_config.font_color,
                    ..Default::default()
                }
            }
        }
    }
}

fn toggle_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Visibility, With<FpsText>>,
    overlay_config: Res<FpsOverlayConfig>,
) {
    if input.just_pressed(overlay_config.keybind) {
        for mut visibility in query.iter_mut() {
            *visibility = if let Visibility::Hidden = *visibility {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        }
    }
}
