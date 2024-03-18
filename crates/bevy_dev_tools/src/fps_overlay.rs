//! Module containing logic for FPS overlay.

use std::any::TypeId;

use bevy_app::{Plugin, Startup, Update};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    component::Component,
    query::With,
    schedule::{common_conditions::resource_changed, IntoSystemConfigs},
    system::{Commands, Query, Res, Resource},
};
use bevy_render::view::Visibility;
use bevy_text::{Font, Text, TextSection, TextStyle};
use bevy_ui::node_bundles::TextBundle;

use crate::{DevTool, DevToolApp, DevToolsStore};

/// A plugin that adds an FPS overlay to the Bevy application.
///
/// This plugin will add the [`FrameTimeDiagnosticsPlugin`] if it wasn't added before.
///
/// Note: It is recommended to use native overlay of rendering statistics when possible for lower overhead and more accurate results.
/// The correct way to do this will vary by platform:
/// - **Metal**: setting env variable `MTL_HUD_ENABLED=1`
#[derive(Default)]
pub struct FpsOverlayPlugin {
    /// Starting configuration of overlay, this can be later be changed through [`FpsOverlayConfig`] resource.
    pub config: FpsOverlayConfig,
}

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // TODO: Use plugin dependencies, see https://github.com/bevyengine/bevy/issues/69
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }
        app.insert_resource(self.config.clone())
            .init_dev_tool::<FpsOverlayConfig>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    customize_text.run_if(resource_changed::<FpsOverlayConfig>),
                    change_visibility.run_if(resource_changed::<DevToolsStore>),
                    update_text.run_if(|dev_tools: Res<DevToolsStore>| {
                        dev_tools
                            .get(&TypeId::of::<FpsOverlayConfig>())
                            .is_some_and(|dev_tool| dev_tool.is_enabled)
                    }),
                ),
            );
    }
}

/// Configuration options for the FPS overlay.
#[derive(Resource, Clone, Debug)]
pub struct FpsOverlayConfig {
    /// Configuration of text in the overlay.
    pub text_config: TextStyle,
}

impl DevTool for FpsOverlayConfig {}

impl Default for FpsOverlayConfig {
    fn default() -> Self {
        FpsOverlayConfig {
            text_config: TextStyle {
                font: Handle::<Font>::default(),
                font_size: 32.0,
                color: Color::WHITE,
            },
        }
    }
}

#[derive(Component)]
struct FpsText;

fn setup(mut commands: Commands, overlay_config: Res<FpsOverlayConfig>) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new("FPS: ", overlay_config.text_config.clone()),
            TextSection::from_style(overlay_config.text_config.clone()),
        ]),
        FpsText,
    ));
}

fn update_text(diagnostic: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[1].value = format!("{value:.2}");
            }
        }
    }
}

fn customize_text(
    overlay_config: Res<FpsOverlayConfig>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    for mut text in &mut query {
        for section in text.sections.iter_mut() {
            section.style = overlay_config.text_config.clone();
        }
    }
}

fn change_visibility(
    mut query: Query<&mut Visibility, With<FpsText>>,
    dev_tools: Res<DevToolsStore>,
) {
    if dev_tools
        .get(&TypeId::of::<FpsOverlayConfig>())
        .is_some_and(|dev_tool| dev_tool.is_enabled)
    {
        for mut visibility in query.iter_mut() {
            *visibility = Visibility::Visible;
        }
    } else {
        for mut visibility in query.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }
}
