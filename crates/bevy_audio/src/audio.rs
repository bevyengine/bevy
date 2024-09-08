use crate::{AudioSource, Decodable};
use bevy_asset::{Asset, Handle};
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_reflect::prelude::*;

/// A volume level equivalent to a non-negative float.
#[derive(Clone, Copy, Deref, Debug, Reflect)]
pub struct Volume(pub(crate) f32);

impl Default for Volume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Volume {
    /// Create a new volume level.
    pub fn new(volume: f32) -> Self {
        debug_assert!(volume >= 0.0);
        Self(f32::max(volume, 0.))
    }
    /// Get the value of the volume level.
    pub fn get(&self) -> f32 {
        self.0
    }

    /// Zero (silent) volume level
    pub const ZERO: Self = Volume(0.0);
}

/// The way Bevy manages the sound playback.
#[derive(Debug, Clone, Copy, Reflect)]
pub enum PlaybackMode {
    /// Play the sound once. Do nothing when it ends.
    Once,
    /// Repeat the sound forever.
    Loop,
    /// Despawn the entity and its children when the sound finishes playing.
    Despawn,
    /// Remove the audio components from the entity, when the sound finishes playing.
    Remove,
}

/// Initial settings to be used when audio starts playing.
/// If you would like to control the audio while it is playing, query for the
/// [`AudioSink`][crate::AudioSink] or [`SpatialAudioSink`][crate::SpatialAudioSink]
/// components. Changes to this component will *not* be applied to already-playing audio.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Default, Component)]
pub struct PlaybackSettings {
    /// The desired playback behavior.
    pub mode: PlaybackMode,
    /// Volume to play at.
    pub volume: Volume,
    /// Speed to play at.
    pub speed: f32,
    /// Create the sink in paused state.
    /// Useful for "deferred playback", if you want to prepare
    /// the entity, but hear the sound later.
    pub paused: bool,
    /// Enables spatial audio for this source.
    ///
    /// See also: [`SpatialListener`].
    ///
    /// Note: Bevy does not currently support HRTF or any other high-quality 3D sound rendering
    /// features. Spatial audio is implemented via simple left-right stereo panning.
    pub spatial: bool,
    /// Optional scale factor applied to the positions of this audio source and the listener,
    /// overriding the default value configured on [`AudioPlugin::default_spatial_scale`](crate::AudioPlugin::default_spatial_scale).
    pub spatial_scale: Option<SpatialScale>,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        // TODO: what should the default be: ONCE/DESPAWN/REMOVE?
        Self::ONCE
    }
}

impl PlaybackSettings {
    /// Will play the associated audio source once.
    pub const ONCE: PlaybackSettings = PlaybackSettings {
        mode: PlaybackMode::Once,
        volume: Volume(1.0),
        speed: 1.0,
        paused: false,
        spatial: false,
        spatial_scale: None,
    };

    /// Will play the associated audio source in a loop.
    pub const LOOP: PlaybackSettings = PlaybackSettings {
        mode: PlaybackMode::Loop,
        ..PlaybackSettings::ONCE
    };

    /// Will play the associated audio source once and despawn the entity afterwards.
    pub const DESPAWN: PlaybackSettings = PlaybackSettings {
        mode: PlaybackMode::Despawn,
        ..PlaybackSettings::ONCE
    };

    /// Will play the associated audio source once and remove the audio components afterwards.
    pub const REMOVE: PlaybackSettings = PlaybackSettings {
        mode: PlaybackMode::Remove,
        ..PlaybackSettings::ONCE
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

    /// Helper to enable or disable spatial audio.
    pub const fn with_spatial(mut self, spatial: bool) -> Self {
        self.spatial = spatial;
        self
    }

    /// Helper to use a custom spatial scale.
    pub const fn with_spatial_scale(mut self, spatial_scale: SpatialScale) -> Self {
        self.spatial_scale = Some(spatial_scale);
        self
    }
}

/// Settings for the listener for spatial audio sources.
///
/// This must be accompanied by `Transform` and `GlobalTransform`.
/// Only one entity with a `SpatialListener` should be present at any given time.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Default, Component)]
pub struct SpatialListener {
    /// Left ear position relative to the `GlobalTransform`.
    pub left_ear_offset: Vec3,
    /// Right ear position relative to the `GlobalTransform`.
    pub right_ear_offset: Vec3,
}

impl Default for SpatialListener {
    fn default() -> Self {
        Self::new(4.)
    }
}

impl SpatialListener {
    /// Creates a new `SpatialListener` component.
    ///
    /// `gap` is the distance between the left and right "ears" of the listener. Ears are
    /// positioned on the x axis.
    pub fn new(gap: f32) -> Self {
        SpatialListener {
            left_ear_offset: Vec3::X * gap / -2.0,
            right_ear_offset: Vec3::X * gap / 2.0,
        }
    }
}

/// Use this [`Resource`] to control the global volume of all audio.
///
/// Note: changing this value will not affect already playing audio.
#[derive(Resource, Default, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct GlobalVolume {
    /// The global volume of all audio.
    pub volume: Volume,
}

impl GlobalVolume {
    /// Create a new [`GlobalVolume`] with the given volume.
    pub fn new(volume: f32) -> Self {
        Self {
            volume: Volume::new(volume),
        }
    }
}

/// A scale factor applied to the positions of audio sources and listeners for
/// spatial audio.
///
/// Default is `Vec3::ONE`.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct SpatialScale(pub Vec3);

impl SpatialScale {
    /// Create a new `SpatialScale` with the same value for all 3 dimensions.
    pub const fn new(scale: f32) -> Self {
        Self(Vec3::splat(scale))
    }

    /// Create a new `SpatialScale` with the same value for `x` and `y`, and `0.0`
    /// for `z`.
    pub const fn new_2d(scale: f32) -> Self {
        Self(Vec3::new(scale, scale, 0.0))
    }
}

impl Default for SpatialScale {
    fn default() -> Self {
        Self(Vec3::ONE)
    }
}

/// The default scale factor applied to the positions of audio sources and listeners for
/// spatial audio. Can be overridden for individual sounds in [`PlaybackSettings`].
///
/// You may need to adjust this scale to fit your world's units.
///
/// Default is `Vec3::ONE`.
#[derive(Resource, Default, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct DefaultSpatialScale(pub SpatialScale);

/// Bundle for playing a standard bevy audio asset
pub type AudioBundle = AudioSourceBundle<AudioSource>;

/// Bundle for playing a sound.
///
/// Insert this bundle onto an entity to trigger a sound source to begin playing.
///
/// If the handle refers to an unavailable asset (such as if it has not finished loading yet),
/// the audio will not begin playing immediately. The audio will play when the asset is ready.
///
/// When Bevy begins the audio playback, an [`AudioSink`][crate::AudioSink] component will be
/// added to the entity. You can use that component to control the audio settings during playback.
#[derive(Bundle)]
pub struct AudioSourceBundle<Source = AudioSource>
where
    Source: Asset + Decodable,
{
    /// Asset containing the audio data to play.
    pub source: Handle<Source>,
    /// Initial settings that the audio starts playing with.
    /// If you would like to control the audio while it is playing,
    /// query for the [`AudioSink`][crate::AudioSink] component.
    /// Changes to this component will *not* be applied to already-playing audio.
    pub settings: PlaybackSettings,
}

impl<T: Asset + Decodable> Clone for AudioSourceBundle<T> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            settings: self.settings,
        }
    }
}

impl<T: Decodable + Asset> Default for AudioSourceBundle<T> {
    fn default() -> Self {
        Self {
            source: Default::default(),
            settings: Default::default(),
        }
    }
}
