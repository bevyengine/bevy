use bevy_ecs::component::Component;
use bevy_math::Vec3;
use bevy_transform::prelude::Transform;
use rodio::{Sink, SpatialSink};

/// Common interactions with an audio sink.
pub trait AudioSinkPlayback {
    /// Gets the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than `1.0`
    /// will multiply each sample by this value.
    fn volume(&self) -> f32;

    /// Changes the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than `1.0`
    /// will multiply each sample by this value.
    ///
    /// # Note on Audio Volume
    ///
    /// An increase of 10 decibels (dB) roughly corresponds to the perceived volume doubling in intensity.
    /// As this function scales not the volume but the amplitude, a conversion might be necessary.
    /// For example, to halve the perceived volume you need to decrease the volume by 10 dB.
    /// This corresponds to 20log(x) = -10dB, solving x = 10^(-10/20) = 0.316.
    /// Multiply the current volume by 0.316 to halve the perceived volume.
    fn set_volume(&self, volume: f32);

    /// Gets the speed of the sound.
    ///
    /// The value `1.0` is the "normal" speed (unfiltered input). Any value other than `1.0`
    /// will change the play speed of the sound.
    fn speed(&self) -> f32;

    /// Changes the speed of the sound.
    ///
    /// The value `1.0` is the "normal" speed (unfiltered input). Any value other than `1.0`
    /// will change the play speed of the sound.
    fn set_speed(&self, speed: f32);

    /// Resumes playback of a paused sink.
    ///
    /// No effect if not paused.
    fn play(&self);

    /// Pauses playback of this sink.
    ///
    /// No effect if already paused.
    /// A paused sink can be resumed with [`play`](Self::play).
    fn pause(&self);

    /// Toggles the playback of this sink.
    ///
    /// Will pause if playing, and will be resumed if paused.
    fn toggle(&self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    /// Is this sink paused?
    ///
    /// Sinks can be paused and resumed using [`pause`](Self::pause) and [`play`](Self::play).
    fn is_paused(&self) -> bool;

    /// Stops the sink.
    ///
    /// It won't be possible to restart it afterwards.
    fn stop(&self);

    /// Returns true if this sink has no more sounds to play.
    fn empty(&self) -> bool;
}

/// Used to control audio during playback.
///
/// Bevy inserts this component onto your entities when it begins playing an audio source.
/// Use [`AudioBundle`][crate::AudioBundle] to trigger that to happen.
///
/// You can use this component to modify the playback settings while the audio is playing.
///
/// If this component is removed from an entity, and an [`AudioSource`][crate::AudioSource] is
/// attached to that entity, that [`AudioSource`][crate::AudioSource] will start playing. If
/// that source is unchanged, that translates to the audio restarting.
#[derive(Component)]
pub struct AudioSink {
    pub(crate) sink: Sink,
}

impl AudioSinkPlayback for AudioSink {
    fn volume(&self) -> f32 {
        self.sink.volume()
    }

    fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    fn speed(&self) -> f32 {
        self.sink.speed()
    }

    fn set_speed(&self, speed: f32) {
        self.sink.set_speed(speed);
    }

    fn play(&self) {
        self.sink.play();
    }

    fn pause(&self) {
        self.sink.pause();
    }

    fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    fn stop(&self) {
        self.sink.stop();
    }

    fn empty(&self) -> bool {
        self.sink.empty()
    }
}

/// Used to control spatial audio during playback.
///
/// Bevy inserts this component onto your entities when it begins playing an audio source
/// that's configured to use spatial audio.
///
/// You can use this component to modify the playback settings while the audio is playing.
///
/// If this component is removed from an entity, and a [`AudioSource`][crate::AudioSource] is
/// attached to that entity, that [`AudioSource`][crate::AudioSource] will start playing. If
/// that source is unchanged, that translates to the audio restarting.
#[derive(Component)]
pub struct SpatialAudioSink {
    pub(crate) sink: SpatialSink,
}

impl AudioSinkPlayback for SpatialAudioSink {
    fn volume(&self) -> f32 {
        self.sink.volume()
    }

    fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    fn speed(&self) -> f32 {
        self.sink.speed()
    }

    fn set_speed(&self, speed: f32) {
        self.sink.set_speed(speed);
    }

    fn play(&self) {
        self.sink.play();
    }

    fn pause(&self) {
        self.sink.pause();
    }

    fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    fn stop(&self) {
        self.sink.stop();
    }

    fn empty(&self) -> bool {
        self.sink.empty()
    }
}

impl SpatialAudioSink {
    /// Set the two ears position.
    pub fn set_ears_position(&self, left_position: Vec3, right_position: Vec3) {
        self.sink.set_left_ear_position(left_position.to_array());
        self.sink.set_right_ear_position(right_position.to_array());
    }

    /// Set the listener position, with an ear on each side separated by `gap`.
    pub fn set_listener_position(&self, position: Transform, gap: f32) {
        self.set_ears_position(
            position.translation + position.left() * gap / 2.0,
            position.translation + position.right() * gap / 2.0,
        );
    }

    /// Set the emitter position.
    pub fn set_emitter_position(&self, position: Vec3) {
        self.sink.set_emitter_position(position.to_array());
    }
}
