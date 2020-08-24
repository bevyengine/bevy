use bevy::{prelude::*, text::FontAtlasSet};

/// This example illustrates how FontAtlases are populated. Bevy uses FontAtlases under the hood to optimize text rendering.
fn main() {
    App::build()
        .init_resource::<State>()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(text_update_system.system())
        .add_system(atlas_render_system.system())
        .run();
}

struct State {
    added: bool,
    handle: Handle<Font>,
    timer: Timer,
}

impl Default for State {
    fn default() -> Self {
        Self {
            added: false,
            handle: Handle::default(),
            timer: Timer::from_seconds(0.05, true),
        }
    }
}

fn atlas_render_system(
    mut commands: Commands,
    mut state: ResMut<State>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    font_atlas_sets: Res<Assets<FontAtlasSet>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    if state.added {
        return;
    }
    if let Some(set) = font_atlas_sets.get(&state.handle.as_handle::<FontAtlasSet>()) {
        if let Some((_size, font_atlas)) = set.iter().next() {
            state.added = true;
            let texture_atlas = texture_atlases.get(&font_atlas.texture_atlas).unwrap();
            commands.spawn(ImageComponents {
                material: materials.add(texture_atlas.texture.into()),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: Rect {
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }
}

fn text_update_system(mut state: ResMut<State>, time: Res<Time>, mut query: Query<&mut Text>) {
    for mut text in &mut query.iter() {
        state.timer.tick(time.delta_seconds);
        if state.timer.finished {
            text.value = format!("{}", rand::random::<u8>() as char);
            state.timer.reset();
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut state: ResMut<State>) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    state.handle = font_handle;
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                size: Size::new(Val::Px(250.0), Val::Px(60.0)),
                ..Default::default()
            },
            text: Text {
                value: "a".to_string(),
                font: font_handle,
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
}
