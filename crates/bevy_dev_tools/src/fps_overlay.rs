use bevy_app::{Plugin, Startup, Update};
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    component::Component,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_text::{Text, TextSection, TextStyle};
use bevy_ui::node_bundles::TextBundle;

pub struct FpsOverlayPlugin;

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }
        app.add_systems(Startup, setup)
            .add_systems(Update, update_text);
    }
}

#[derive(Component)]
struct FpsText;

fn setup(mut commands: Commands) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "FPS: ",
                TextStyle {
                    font_size: 32.0,
                    ..Default::default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size: 32.0,
                ..Default::default()
            }),
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
