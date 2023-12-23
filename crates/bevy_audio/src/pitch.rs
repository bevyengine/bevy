use crate::{AudioSourceBundle, Decodable};
use bevy_asset::Asset;
use bevy_reflect::TypePath;
use rodio::{source::SineWave, source::TakeDuration, Source};

/// A source of sine wave sound
#[derive(Asset, Debug, Clone, TypePath)]
pub struct Pitch {
    /// Frequency at which sound will be played
    pub frequency: f32,
    /// Duration for which sound will be played
    pub duration: std::time::Duration,
}

impl Pitch {
    /// Creates a new note
    pub fn new(frequency: f32, duration: std::time::Duration) -> Self {
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
pub type PitchBundle = AudioSourceBundle<Pitch>;
