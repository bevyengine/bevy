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
use bevy_text::{Font, Text, TextSection, TextStyle};
use bevy_ui::node_bundles::TextBundle;

#[derive(Default)]
/// A plugin that adds an FPS overlay to the Bevy application.
/// Warning: This plugin will add [`FrameTimeDiagnosticsPlugin`] if it wasn't added before.
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
                ),
            );
    }
}

#[derive(Resource, Clone)]
/// Configuration options for the FPS overlay.
pub struct FpsOverlayConfig(pub TextStyle);

impl Default for FpsOverlayConfig {
    fn default() -> Self {
        FpsOverlayConfig(TextStyle {
            font: Handle::<Font>::default(),
            font_size: 32.0,
            color: Color::WHITE,
        })
    }
}

#[derive(Component)]
struct FpsText;

fn setup(mut commands: Commands, overlay_config: Res<FpsOverlayConfig>) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new("FPS: ", overlay_config.0.clone()),
            TextSection::from_style(overlay_config.0.clone()),
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
            section.style = overlay_config.0.clone();
        }
    }
}
