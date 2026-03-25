use crate::{AudioSource, Decodable, Volume};
use bevy_asset::{Asset, Handle};
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_reflect::prelude::*;
use bevy_transform::components::Transform;

/// The way Bevy manages the sound playback.
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Clone)]
pub enum PlaybackMode {
    /// Play the sound once. Do nothing when it ends.
    ///
    /// Note: It is not possible to reuse an [`AudioPlayer`] after it has finished playing and
    /// the underlying [`AudioSink`](crate::AudioSink) or [`SpatialAudioSink`](crate::SpatialAudioSink) has been drained.
    ///
    /// To replay a sound, the audio components provided by [`AudioPlayer`] must be removed and
    /// added again.
    Once,
    /// Repeat the sound forever.
    Loop,
    /// Despawn the entity and its children when the sound finishes playing.
    Despawn,
    /// Remove the audio components from the entity, when the sound finishes playing.
    Remove,
}

/// Initial settings to be used when audio starts playing.
///
/// If you would like to control the audio while it is playing, query for the
/// [`AudioSink`](crate::AudioSink) or [`SpatialAudioSink`](crate::SpatialAudioSink)
/// components. Changes to this component will *not* be applied to already-playing audio.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Clone, Default, Component, Debug)]
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
    /// Whether to create the sink in muted state or not.
    ///
    /// This is useful for audio that should be initially muted. You can still
    /// set the initial volume and it is applied when the audio is unmuted.
    pub muted: bool,
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
    /// The point in time in the audio clip where playback should start. If set to `None`, it will
    /// play from the beginning of the clip.
    ///
    /// If the playback mode is set to `Loop`, each loop will start from this position.
    pub start_position: Option<core::time::Duration>,
    /// How long the audio should play before stopping. If set, the clip will play for at most
    /// the specified duration. If set to `None`, it will play for as long as it can.
    ///
    /// If the playback mode is set to `Loop`, each loop will last for this duration.
    pub duration: Option<core::time::Duration>,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self::ONCE
    }
}

impl PlaybackSettings {
    /// Will play the associated audio source once.
    ///
    /// Note: It is not possible to reuse an [`AudioPlayer`] after it has finished playing and
    /// the underlying [`AudioSink`](crate::AudioSink) or [`SpatialAudioSink`](crate::SpatialAudioSink) has been drained.
    ///
    /// To replay a sound, the audio components provided by [`AudioPlayer`] must be removed and
    /// added again.
    pub const ONCE: PlaybackSettings = PlaybackSettings {
        mode: PlaybackMode::Once,
        volume: Volume::Linear(1.0),
        speed: 1.0,
        paused: false,
        muted: false,
        spatial: false,
        spatial_scale: None,
        start_position: None,
        duration: None,
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

    /// Helper to start muted.
    pub const fn muted(mut self) -> Self {
        self.muted = true;
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

    /// Helper to use a custom playback start position.
    pub const fn with_start_position(mut self, start_position: core::time::Duration) -> Self {
        self.start_position = Some(start_position);
        self
    }

    /// Helper to use a custom playback duration.
    pub const fn with_duration(mut self, duration: core::time::Duration) -> Self {
        self.duration = Some(duration);
        self
    }
}

/// Settings for the listener for spatial audio sources.
///
/// This is accompanied by [`Transform`] and [`GlobalTransform`](bevy_transform::prelude::GlobalTransform).
/// Only one entity with a [`SpatialListener`] should be present at any given time.
#[derive(Component, Clone, Debug, Reflect)]
#[require(Transform)]
#[reflect(Clone, Default, Component, Debug)]
pub struct SpatialListener {
    /// Left ear position relative to the [`GlobalTransform`](bevy_transform::prelude::GlobalTransform).
    pub left_ear_offset: Vec3,
    /// Right ear position relative to the [`GlobalTransform`](bevy_transform::prelude::GlobalTransform).
    pub right_ear_offset: Vec3,
}

impl Default for SpatialListener {
    fn default() -> Self {
        Self::new(4.)
    }
}

impl SpatialListener {
    /// Creates a new [`SpatialListener`] component.
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

/// A scale factor applied to the positions of audio sources and listeners for
/// spatial audio.
///
/// Default is `Vec3::ONE`.
#[derive(Clone, Copy, Debug, Reflect)]
#[reflect(Clone, Default)]
pub struct SpatialScale(pub Vec3);

impl SpatialScale {
    /// Create a new [`SpatialScale`] with the same value for all 3 dimensions.
    pub const fn new(scale: f32) -> Self {
        Self(Vec3::splat(scale))
    }

    /// Create a new [`SpatialScale`] with the same value for `x` and `y`, and `0.0`
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
#[reflect(Resource, Default, Clone)]
pub struct DefaultSpatialScale(pub SpatialScale);

/// A component for playing a sound.
///
/// Insert this component onto an entity to trigger an audio source to begin playing.
///
/// If the handle refers to an unavailable asset (such as if it has not finished loading yet),
/// the audio will not begin playing immediately. The audio will play when the asset is ready.
///
/// When Bevy begins the audio playback, an [`AudioSink`](crate::AudioSink) component will be
/// added to the entity. You can use that component to control the audio settings during playback.
///
/// Playback can be configured using the [`PlaybackSettings`] component. Note that changes to the
/// [`PlaybackSettings`] component will *not* affect already-playing audio.
#[derive(Component, Reflect)]
#[reflect(Component, Clone)]
#[require(PlaybackSettings)]
pub struct AudioPlayer<Source = AudioSource>(pub Handle<Source>)
where
    Source: Asset + Decodable;

impl<Source> Clone for AudioPlayer<Source>
where
    Source: Asset + Decodable,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl AudioPlayer<AudioSource> {
    /// Creates a new [`AudioPlayer`] with the given [`Handle<AudioSource>`].
    ///
    /// For convenience reasons, this hard-codes the [`AudioSource`] type. If you want to
    /// initialize an [`AudioPlayer`] with a different type, just initialize it directly using normal
    /// tuple struct syntax.
    pub fn new(source: Handle<AudioSource>) -> Self {
        Self(source)
    }
}
