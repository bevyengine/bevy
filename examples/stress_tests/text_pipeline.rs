//! Text pipeline benchmark.
//!
//! Continuously recomputes a large `Text` component with a lot of sections.
//! No rendering.
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{BreakLineOn, TextPipeline, FontAtlasSet, TextSettings, FontAtlasWarning, YAxisOrientation},
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
        .add_system(pipe_text)
        .run();
}

#[derive(Resource)]
pub struct TestText {
    pub sections: Vec<TextSection>,
}

impl FromWorld for TestText {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let mut sections = Vec::new();
        for i in 1 ..= 100 {
            sections.push(TextSection {
                value: "Hello World!".repeat(i),
                style: TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::WHITE,
                },
            });
            sections.push(TextSection {
                value: "So long, Earth?".repeat(i),
                style: TextStyle {
                    font: asset_server.load("fonts/FiraSans-Medium.ttf"),
                    font_size: 25.0,
                    color: Color::RED,
                },
            });
        }
        TestText { sections }
    }
}

pub fn pipe_text(
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