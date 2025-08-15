//! This example illustrates how `FontAtlas`'s are populated.
//! Bevy uses `FontAtlas`'s under the hood to optimize text rendering.

use bevy::{color::palettes::basic::YELLOW, prelude::*, text::FontAtlasSets};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .init_resource::<State>()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (text_update_system, atlas_render_system))
        .run();
}

#[derive(Resource)]
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
            timer: Timer::from_seconds(0.05, TimerMode::Repeating),
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
struct SeededRng(ChaCha8Rng);

fn atlas_render_system(
    mut commands: Commands,
    mut state: ResMut<State>,
    font_atlas_sets: Res<FontAtlasSets>,
    images: Res<Assets<Image>>,
) {
    if let Some(set) = font_atlas_sets.get(&state.handle)
        && let Some((_size, font_atlases)) = set.iter().next()
    {
        let x_offset = state.atlas_count as f32;
        if state.atlas_count == font_atlases.len() as u32 {
            return;
        }
        let font_atlas = &font_atlases[state.atlas_count as usize];
        let image = images.get(&font_atlas.texture).unwrap();
        state.atlas_count += 1;
        commands.spawn((
            ImageNode::new(font_atlas.texture.clone()),
            Node {
                position_type: PositionType::Absolute,
                top: Val::ZERO,
                left: Val::Px(image.width() as f32 * x_offset),
                ..default()
            },
        ));
    }
}

fn text_update_system(
    mut state: ResMut<State>,
    time: Res<Time>,
    mut query: Query<&mut Text>,
    mut seeded_rng: ResMut<SeededRng>,
) {
    if !state.timer.tick(time.delta()).just_finished() {
        return;
    }

    for mut text in &mut query {
        let c = seeded_rng.random::<u8>() as char;
        let string = &mut **text;
        if !string.contains(c) {
            string.push(c);
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut state: ResMut<State>) {
    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");
    state.handle = font_handle.clone();
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::ZERO,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("a"),
                TextFont {
                    font: font_handle,
                    font_size: 50.0,
                    ..default()
                },
                TextColor(YELLOW.into()),
            ));
        });
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    commands.insert_resource(SeededRng(ChaCha8Rng::seed_from_u64(19878367467713)));
}
