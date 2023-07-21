use crate::{AudioSourceBundle, Decodable, SpatialAudioSourceBundle};
use bevy_reflect::{TypePath, TypeUuid};
use rodio::{source::SineWave, source::TakeDuration, Source};

/// A source of sine wave sound
#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "cbc63be3-b0b9-4d2c-a03c-88b58f1a19ef"]
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

/// Bundle for playing a bevy note sound with a 3D position
pub type SpatialPitchBundle = SpatialAudioSourceBundle<Pitch>;
