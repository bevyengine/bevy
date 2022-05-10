use bevy::{prelude::*, text::FontAtlasSet};

// TODO: This is now broken. See #1243
/// This example illustrates how `FontAtlas`'s are populated. Bevy uses `FontAtlas`'s under the hood
/// to optimize text rendering.
fn main() {
    App::new()
        .init_resource::<State>()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(text_update_system)
        .add_system(atlas_render_system)
        .run();
}

struct State {
    atlas_count: u32,
    handle: Handle<Font>,
    timer: Timer,
}

impl Default for State {
    fn default() -> Self {
        Self {
            atlas_count: 0,
            handle: Handle::default(),
            timer: Timer::from_seconds(0.05, true),
        }
    }
}

fn atlas_render_system(
    mut commands: Commands,
    mut state: ResMut<State>,
    font_atlas_sets: Res<Assets<FontAtlasSet>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    if let Some(set) = font_atlas_sets.get(&state.handle.as_weak::<FontAtlasSet>()) {
        if let Some((_size, font_atlas)) = set.iter().next() {
            let x_offset = state.atlas_count as f32;
            if state.atlas_count == font_atlas.len() as u32 {
                return;
            }
            let texture_atlas = texture_atlases
                .get(&font_atlas[state.atlas_count as usize].texture_atlas)
                .unwrap();
            state.atlas_count += 1;
            commands.spawn_bundle(ImageBundle {
                image: texture_atlas.texture.clone().into(),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        top: Val::Px(0.0),
                        left: Val::Px(512.0 * x_offset),
                        ..default()
                    },
                    ..default()
                },
                ..default()
            });
        }
    }
}

fn text_update_system(mut state: ResMut<State>, time: Res<Time>, mut query: Query<&mut Text>) {
    if state.timer.tick(time.delta()).finished() {
        for mut text in query.iter_mut() {
            let c = rand::random::<u8>() as char;
            if !text.sections[0].value.contains(c) {
                text.sections[0].value.push(c);
            }
        }

        state.timer.reset();
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut state: ResMut<State>) {
    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");
    state.handle = font_handle.clone();
    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(TextBundle {
        text: Text::with_section(
            "a",
            TextStyle {
                font: font_handle,
                font_size: 60.0,
                color: Color::YELLOW,
            },
            Default::default(),
        ),
        ..default()
    });
}
