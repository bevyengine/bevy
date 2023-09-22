//! Shows how to create a custom [`Decodable`] type by implementing a Sine wave.
use bevy::audio::AddAudioSource;
use bevy::audio::AudioPlugin;
use bevy::audio::Source;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::utils::Duration;

// This struct usually contains the data for the audio being played.
// This is where data read from an audio file would be stored, for example.
// Implementing `TypeUuid` will automatically implement `Asset`.
// This allows the type to be registered as an asset.
#[derive(Asset, TypePath)]
struct SineAudio {
    frequency: f32,
}
// This decoder is responsible for playing the audio,
// and so stores data about the audio being played.
struct SineDecoder {
    // how far along one period the wave is (between 0 and 1)
    current_progress: f32,
    // how much we move along the period every frame
    progress_per_frame: f32,
    // how long a period is
    period: f32,
    sample_rate: u32,
}

impl SineDecoder {
    fn new(frequency: f32) -> Self {
        // standard sample rate for most recordings
        let sample_rate = 44_100;
        SineDecoder {
            current_progress: 0.,
            progress_per_frame: frequency / sample_rate as f32,
            period: std::f32::consts::PI * 2.,
            sample_rate,
        }
    }
}

// The decoder must implement iterator so that it can implement `Decodable`.
impl Iterator for SineDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.current_progress += self.progress_per_frame;
        // we loop back round to 0 to avoid floating point inaccuracies
        self.current_progress %= 1.;
        Some(f32::sin(self.period * self.current_progress))
    }
}
// `Source` is what allows the audio source to be played by bevy.
// This trait provides information on the audio.
impl Source for SineDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// Finally `Decodable` can be implemented for our `SineAudio`.
impl Decodable for SineAudio {
    type Decoder = SineDecoder;

    type DecoderItem = <SineDecoder as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        SineDecoder::new(self.frequency)
    }
}

fn main() {
    let mut app = App::new();
    // register the audio source so that it can be used
    app.add_plugins(DefaultPlugins.set(AudioPlugin {
        global_volume: GlobalVolume::new(0.2),
    }))
    .add_audio_source::<SineAudio>()
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut assets: ResMut<Assets<SineAudio>>, mut commands: Commands) {
    // add a `SineAudio` to the asset server so that it can be played
    let audio_handle = assets.add(SineAudio {
        frequency: 440., //this is the frequency of A4
    });
    commands.spawn(AudioSourceBundle {
        source: audio_handle,
        ..default()
    });
}
