#[expect(
    deprecated,
    reason = "The deprecated item here (AudioSourceBundle) is only used by a type alias (which itself is deprecated)."
)]
use crate::{AudioSourceBundle, Decodable};
use bevy_asset::Asset;
use bevy_reflect::TypePath;
use rodio::{
    source::{SineWave, TakeDuration},
    Source,
};

/// A source of sine wave sound
#[derive(Asset, Debug, Clone, TypePath)]
pub struct Pitch {
    /// Frequency at which sound will be played
    pub frequency: f32,
    /// Duration for which sound will be played
    pub duration: core::time::Duration,
}

impl Pitch {
    /// Creates a new note
    pub fn new(frequency: f32, duration: core::time::Duration) -> Self {
        Pitch {
            frequency,
            duration,
        }
    }
}

impl Decodable for Pitch {
    type DecoderItem = <SineWave as Iterator>::Item;
    type Decoder = TakeDuration<SineWave>;

    fn decoder(&self) -> Self::Decoder {
        SineWave::new(self.frequency).take_duration(self.duration)
    }
}

/// Bundle for playing a bevy note sound
#[deprecated(
    since = "0.15.0",
    note = "Use the `AudioPlayer<Pitch>` component instead. Inserting it will now also insert a `PlaybackSettings` component automatically."
)]
#[expect(
    deprecated,
    reason = "This is a deprecated alias for a deprecated item."
)]
pub type PitchBundle = AudioSourceBundle<Pitch>;
