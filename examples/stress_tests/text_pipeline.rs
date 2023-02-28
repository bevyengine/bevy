//! Text pipeline benchmark.
//!
//! Continuously recomputes a large `Text` component with 100 sections.
//! No rendering.
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{
        BreakLineOn, FontAtlasSet, FontAtlasWarning, TextPipeline, TextSettings, YAxisOrientation,
    },
    window::{PresentMode, WindowPlugin},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .init_resource::<TestText>()
        .add_system(text_pipe)
        .run();
}

#[derive(Resource)]
struct TestText {
    pub sections: Vec<TextSection>,
}

impl FromWorld for TestText {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let mut sections = Vec::new();
        for i in 1..=50 {
            sections.push(TextSection {
                value: "Hello World!".repeat(i),
                style: TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: (10 + i % 10) as f32,
                    ..Default::default()
                },
            });
            sections.push(TextSection {
                value: "So long, Earth?".repeat(i),
                style: TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: (11 + i % 15) as f32,
                    ..Default::default()
                },
            });
        }
        TestText { sections }
    }
}

fn text_pipe(
    mut text_pipeline: ResMut<TextPipeline>,
    fonts: Res<Assets<Font>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    test_text: Res<TestText>,
) {
    let _ = text_pipeline.queue_text(
        &fonts,
        &test_text.sections,
        2.5,
        TextAlignment::Center,
        BreakLineOn::AnyCharacter,
        Vec2::new(1000.0, f32::INFINITY),
        &mut font_atlas_set_storage,
        &mut texture_atlases,
        &mut textures,
        text_settings.as_ref(),
        &mut font_atlas_warning,
        YAxisOrientation::BottomToTop,
    );
}
