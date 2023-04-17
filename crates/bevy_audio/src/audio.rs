use crate::{AudioSource, Decodable};
use bevy_asset::{Asset, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_transform::prelude::Transform;

/// Defines the volume to play an audio source at.
#[derive(Clone, Copy, Debug)]
pub enum Volume {
    /// A volume level relative to the global volume.
    Relative(VolumeLevel),
    /// A volume level that ignores the global volume.
    Absolute(VolumeLevel),
}

impl Default for Volume {
    fn default() -> Self {
        Self::Relative(VolumeLevel::default())
    }
}

impl Volume {
    /// Create a new volume level relative to the global volume.
    pub fn new_relative(volume: f32) -> Self {
        Self::Relative(VolumeLevel::new(volume))
    }
    /// Create a new volume level that ignores the global volume.
    pub fn new_absolute(volume: f32) -> Self {
        Self::Absolute(VolumeLevel::new(volume))
    }
}

/// A volume level equivalent to a non-negative float.
#[derive(Clone, Copy, Deref, DerefMut, Debug)]
pub struct VolumeLevel(pub(crate) f32);

impl Default for VolumeLevel {
    fn default() -> Self {
        Self(1.0)
    }
}

impl VolumeLevel {
    /// Create a new volume level.
    pub fn new(volume: f32) -> Self {
        debug_assert!(volume >= 0.0);
        Self(volume)
    }
    /// Get the value of the volume level.
    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Initial settings to be used when audio starts playing.
/// If you would like to control the audio while it is playing, query for the
/// [`AudioSink`] or [`SpatialAudioSink`] components.
/// Changes to this component will *not* be applied to already-playing audio.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlaybackSettings {
    /// Repeat/loop the sound.
    pub repeat: bool,
    /// Volume to play at.
    pub volume: Volume,
    /// Speed to play at.
    pub speed: f32,
    /// Create the sink in paused state.
    /// Useful for "deferred playback", if you want to prepare
    /// the entity, but hear the sound later.
    pub paused: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self::ONCE
    }
}

impl PlaybackSettings {
    /// Will play the associated audio source once.
    pub const ONCE: PlaybackSettings = PlaybackSettings {
        repeat: false,
        volume: Volume::Relative(VolumeLevel(1.0)),
        speed: 1.0,
        paused: false,
    };

    /// Will play the associated audio source in a loop.
    pub const LOOP: PlaybackSettings = PlaybackSettings {
        repeat: true,
        volume: Volume::Relative(VolumeLevel(1.0)),
        speed: 1.0,
        paused: false,
    };

    /// Helper to start in a paused state.
    pub const fn paused(mut self) -> Self {
        self.paused = true;
        self
    }

    /// Helper to set the volume from start of playback.
    pub const fn with_volume(mut self, volume: Volume) -> Self {
        self.volume = volume;
        self
    }

    /// Helper to set the speed from start of playback.
    pub const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

/// Settings for playing spatial audio.
///
/// These settings are applied when the sound starts playing. If they are changed after
/// the sound has started playing, they will have no effect. Bevy does not yet support
/// moving positional sound sources.
///
/// Note: Bevy does not currently support HRTF or any other high-quality 3D sound rendering
/// features. Spatial audio is implemented via simple left-right stereo panning.
#[derive(Component, Clone, Debug)]
pub struct SpatialSettings {
    pub(crate) left_ear: [f32; 3],
    pub(crate) right_ear: [f32; 3],
    pub(crate) emitter: [f32; 3],
}

impl SpatialSettings {
    /// Configure spatial audio coming from the `emitter` position and heard by a `listener`.
    ///
    /// The `listener` transform provides the position and rotation where the sound is to be
    /// heard from. `gap` is the distance between the left and right "ears" of the listener.
    /// `emitter` is the position where the sound comes from.
    pub fn new(listener: Transform, gap: f32, emitter: Vec3) -> Self {
        SpatialSettings {
            left_ear: (listener.translation + listener.left() * gap / 2.0).to_array(),
            right_ear: (listener.translation + listener.right() * gap / 2.0).to_array(),
            emitter: emitter.to_array(),
        }
    }
}

/// Use this [`Resource`] to control the global volume of all audio with a [`Volume::Relative`] volume.
///
/// Note: changing this value will not affect already playing audio.
#[derive(Resource, Default, Clone, Copy)]
pub struct GlobalVolume {
    /// The global volume of all audio.
    pub volume: VolumeLevel,
}

impl GlobalVolume {
    /// Create a new [`GlobalVolume`] with the given volume.
    pub fn new(volume: f32) -> Self {
        Self {
            volume: VolumeLevel::new(volume),
        }
    }
}

/// Bundle for playing a sound.
///
/// Insert this bundle onto an entity to trigger a sound source to begin playing.
///
/// If the handle refers to an unavailable asset (such as if it has not finished loading yet),
/// the audio will not begin playing immediately. The audio will play when the asset is ready.
///
/// When Bevy begins the audio playback, an [`AudioSink`] component will be added to the
/// entity. You can use that component to control the audio settings during playback.
#[derive(Bundle, Default)]
pub struct AudioBundle<Source = AudioSource>
where
    Source: Asset + Decodable,
{
    /// Asset containing the audio data to play.
    pub source: Handle<Source>,
    /// Initial settings that the audio starts playing with.
    /// If you would like to control the audio while it is playing,
    /// query for the [`AudioSink`] component.
    /// Changes to this component will *not* be applied to already-playing audio.
    pub settings: PlaybackSettings,
}

impl AudioBundle<AudioSource> {
    /// Create an [`AudioBundle`] from a standard Bevy audio source.
    ///
    /// Use this if you are loading an audio file asset in the formats supported by Bevy.
    pub fn from_audio_source(handle: Handle<AudioSource>) -> Self {
        AudioBundle {
            source: handle,
            settings: Default::default(),
        }
    }
}

impl<Source> AudioBundle<Source>
where
    Source: Asset + Decodable,
{
    /// Create an [`AudioBundle`] from a generic source asset type
    ///
    /// Use this if you have a custom source of audio data (not a regular audio file asset loaded by Bevy).
    ///
    /// Don't forget to register your custom type: `app.add_audio_source::<MySource>()`!
    pub fn from_custom_source(handle: Handle<Source>) -> Self {
        AudioBundle {
            source: handle,
            settings: Default::default(),
        }
    }

    /// Configure the initial playback settings.
    ///
    /// The audio will start playing with these settings, when the source data is available.
    pub fn with_settings(mut self, settings: PlaybackSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Enable 3D spatial audio playback with the given configuration.
    ///
    /// Converts this bundle into a [`SpatialAudioBundle`].
    pub fn with_spatial(self, spatial: SpatialSettings) -> SpatialAudioBundle<Source> {
        SpatialAudioBundle {
            source: self.source,
            settings: self.settings,
            spatial,
        }
    }
}

/// Bundle for playing a sound with a 3D position.
///
/// Insert this bundle onto an entity to trigger a sound source to begin playing.
///
/// If the handle refers to an unavailable asset (such as if it has not finished loading yet),
/// the audio will not begin playing immediately. The audio will play when the asset is ready.
///
/// When Bevy begins the audio playback, a [`SpatialAudioSink`] component will be added to the
/// entity. You can use that component to control the audio settings during playback.
#[derive(Bundle)]
pub struct SpatialAudioBundle<Source = AudioSource>
where
    Source: Asset + Decodable,
{
    /// Asset containing the audio data to play.
    pub source: Handle<Source>,
    /// Initial settings that the audio starts playing with.
    /// If you would like to control the audio while it is playing,
    /// query for the [`SpatialAudioSink`] component.
    /// Changes to this component will *not* be applied to already-playing audio.
    pub settings: PlaybackSettings,
    /// Spatial audio configuration. Specifies the positions of the source and listener.
    pub spatial: SpatialSettings,
}
