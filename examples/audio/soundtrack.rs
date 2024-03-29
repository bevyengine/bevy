//! This example illustrates how to load and play different soundtracks,
//! transitioning between them as the game state changes.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (cycle_game_state, fade_in, fade_out))
        .add_systems(Update, change_track)
        .run();
}

// This resource simulates game states
#[derive(Resource, Default)]
enum GameState {
    #[default]
    Peaceful,
    Battle,
}

// This timer simulates game state changes
#[derive(Resource)]
struct GameStateTimer(Timer);

//  This resource will hold the track list for your soundtrack
#[derive(Resource)]
struct SoundtrackPlayer {
    track_list: Vec<Handle<AudioSource>>,
}

impl SoundtrackPlayer {
    fn new(track_list: Vec<Handle<AudioSource>>) -> Self {
        Self { track_list }
    }
}

// This component will be attached to an entity to fade the audio in
#[derive(Component)]
struct FadeIn;

// This component will be attached to an entity to fade the audio out
#[derive(Component)]
struct FadeOut;

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    // Instantiate the game state resources
    commands.insert_resource(GameState::default());
    commands.insert_resource(GameStateTimer(Timer::from_seconds(
        10.0,
        TimerMode::Repeating,
    )));

    // Create the track list
    let track_1 = asset_server.load::<AudioSource>("sounds/Mysterious acoustic guitar.ogg");
    let track_2 = asset_server.load::<AudioSource>("sounds/Epic orchestra music.ogg");
    let track_list = vec![track_1, track_2];
    commands.insert_resource(SoundtrackPlayer::new(track_list));
}

// Every time the GameState resource changes, this system is run to trigger the song change.
fn change_track(
    mut commands: Commands,
    soundtrack_player: Res<SoundtrackPlayer>,
    soundtrack: Query<Entity, With<AudioSink>>,
    game_state: Res<GameState>,
) {
    if game_state.is_changed() {
        // Fade out all currently running tracks
        for track in soundtrack.iter() {
            commands.entity(track).insert(FadeOut);
        }

        // Spawn a new `AudioBundle` with the appropriate soundtrack based on
        // the game state.
        //
        // Volume is set to start at zero and is then increased by the fade_in system.
        match game_state.as_ref() {
            GameState::Peaceful => {
                commands.spawn((
                    AudioBundle {
                        source: soundtrack_player.track_list.first().unwrap().clone(),
                        settings: PlaybackSettings {
                            mode: bevy::audio::PlaybackMode::Loop,
                            volume: bevy::audio::Volume::ZERO,
                            ..default()
                        },
                    },
                    FadeIn,
                ));
            }
            GameState::Battle => {
                commands.spawn((
                    AudioBundle {
                        source: soundtrack_player.track_list.get(1).unwrap().clone(),
                        settings: PlaybackSettings {
                            mode: bevy::audio::PlaybackMode::Loop,
                            volume: bevy::audio::Volume::ZERO,
                            ..default()
                        },
                    },
                    FadeIn,
                ));
            }
        }
    }
}

// Fade effect duration
const FADE_TIME: f32 = 2.0;

// Fades in the audio of entities that has the FadeIn component. Removes the FadeIn component once
// full volume is reached.
fn fade_in(
    mut commands: Commands,
    mut audio_sink: Query<(&mut AudioSink, Entity), With<FadeIn>>,
    time: Res<Time>,
) {
    for (audio, entity) in audio_sink.iter_mut() {
        audio.set_volume(audio.volume() + time.delta_seconds() / FADE_TIME);
        if audio.volume() >= 1.0 {
            audio.set_volume(1.0);
            commands.entity(entity).remove::<FadeIn>();
        }
    }
}

// Fades out the audio of entities that has the FadeOut component. Despawns the entities once audio
// volume reaches zero.
fn fade_out(
    mut commands: Commands,
    mut audio_sink: Query<(&mut AudioSink, Entity), With<FadeOut>>,
    time: Res<Time>,
) {
    for (audio, entity) in audio_sink.iter_mut() {
        audio.set_volume(audio.volume() - time.delta_seconds() / FADE_TIME);
        if audio.volume() <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

// Every time the timer ends, switches between the "Peaceful" and "Battle" state.
fn cycle_game_state(
    mut timer: ResMut<GameStateTimer>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        match game_state.as_ref() {
            GameState::Battle => *game_state = GameState::Peaceful,
            GameState::Peaceful => *game_state = GameState::Battle,
        }
    }
}
