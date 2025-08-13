use crate::Volume;
use bevy_ecs::component::Component;
use bevy_math::Vec3;
use bevy_transform::prelude::Transform;
use core::time::Duration;
pub use rodio::source::SeekError;
use rodio::{Sink, SpatialSink};

/// Common interactions with an audio sink.
pub trait AudioSinkPlayback {
    /// Gets the volume of the sound as a [`Volume`].
    ///
    /// If the sink is muted, this returns the managed volume rather than the
    /// sink's actual volume. This allows you to use the returned volume as if
    /// the sink were not muted, because a muted sink has a physical volume of
    /// 0.
    fn volume(&self) -> Volume;

    /// Changes the volume of the sound to the given [`Volume`].
    ///
    /// If the sink is muted, changing the volume won't unmute it, i.e. the
    /// sink's volume will remain "off" / "muted". However, the sink will
    /// remember the volume change and it will be used when
    /// [`unmute`](Self::unmute) is called. This allows you to control the
    /// volume even when the sink is muted.
    fn set_volume(&mut self, volume: Volume);

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

    /// Returns the position of the sound that's being played.
    ///
    /// This takes into account any speedup or delay applied.
    ///
    /// Example: if you [`set_speed(2.0)`](Self::set_speed) and [`position()`](Self::position) returns *5s*,
    /// then the position in the recording is *10s* from its start.
    fn position(&self) -> Duration;

    /// Attempts to seek to a given position in the current source.
    ///
    /// This blocks between 0 and ~5 milliseconds.
    ///
    /// As long as the duration of the source is known, seek is guaranteed to saturate
    /// at the end of the source. For example given a source that reports a total duration
    /// of 42 seconds calling `try_seek()` with 60 seconds as argument will seek to
    /// 42 seconds.
    ///
    /// # Errors
    /// This function will return [`SeekError::NotSupported`] if one of the underlying
    /// sources does not support seeking.
    ///
    /// It will return an error if an implementation ran
    /// into one during the seek.
    ///
    /// When seeking beyond the end of a source, this
    /// function might return an error if the duration of the source is not known.
    fn try_seek(&self, pos: Duration) -> Result<(), SeekError>;

    /// Pauses playback of this sink.
    ///
    /// No effect if already paused.
    /// A paused sink can be resumed with [`play`](Self::play).
    fn pause(&self);

    /// Toggles playback of the sink.
    ///
    /// If the sink is paused, toggling playback resumes it. If the sink is
    /// playing, toggling playback pauses it.
    fn toggle_playback(&self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    /// Returns true if the sink is paused.
    ///
    /// Sinks can be paused and resumed using [`pause`](Self::pause) and [`play`](Self::play).
    fn is_paused(&self) -> bool;

    /// Stops the sink.
    ///
    /// It won't be possible to restart it afterwards.
    fn stop(&self);

    /// Returns true if this sink has no more sounds to play.
    fn empty(&self) -> bool;

    /// Returns true if the sink is muted.
    fn is_muted(&self) -> bool;

    /// Mutes the sink.
    ///
    /// Muting a sink sets the volume to 0. Use [`unmute`](Self::unmute) to
    /// unmute the sink and restore the original volume.
    fn mute(&mut self);

    /// Unmutes the sink.
    ///
    /// Restores the volume to the value it was before it was muted.
    fn unmute(&mut self);

    /// Toggles whether the sink is muted or not.
    fn toggle_mute(&mut self) {
        if self.is_muted() {
            self.unmute();
        } else {
            self.mute();
        }
    }
}

/// Used to control audio during playback.
///
/// Bevy inserts this component onto your entities when it begins playing an audio source.
/// Use [`AudioPlayer`][crate::AudioPlayer] to trigger that to happen.
///
/// You can use this component to modify the playback settings while the audio is playing.
///
/// If this component is removed from an entity, and an [`AudioSource`][crate::AudioSource] is
/// attached to that entity, that [`AudioSource`][crate::AudioSource] will start playing. If
/// that source is unchanged, that translates to the audio restarting.
#[derive(Component)]
pub struct AudioSink {
    pub(crate) sink: Sink,

    /// Managed volume allows the sink to be muted without losing the user's
    /// intended volume setting.
    ///
    /// This is used to restore the volume when [`unmute`](Self::unmute) is
    /// called.
    ///
    /// If the sink is not muted, this is `None`.
    ///
    /// If the sink is muted, this is `Some(volume)` where `volume` is the
    /// user's intended volume setting, even if the underlying sink's volume is
    /// 0.
    pub(crate) managed_volume: Option<Volume>,
}

impl AudioSink {
    /// Create a new audio sink.
    pub fn new(sink: Sink) -> Self {
        Self {
            sink,
            managed_volume: None,
        }
    }
}

impl AudioSinkPlayback for AudioSink {
    fn volume(&self) -> Volume {
        self.managed_volume
            .unwrap_or_else(|| Volume::Linear(self.sink.volume()))
    }

    fn set_volume(&mut self, volume: Volume) {
        if self.is_muted() {
            self.managed_volume = Some(volume);
        } else {
            self.sink.set_volume(volume.to_linear());
        }
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

    fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    fn try_seek(&self, pos: Duration) -> Result<(), SeekError> {
        self.sink.try_seek(pos)
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

    fn is_muted(&self) -> bool {
        self.managed_volume.is_some()
    }

    fn mute(&mut self) {
        self.managed_volume = Some(self.volume());
        self.sink.set_volume(0.0);
    }

    fn unmute(&mut self) {
        if let Some(volume) = self.managed_volume.take() {
            self.sink.set_volume(volume.to_linear());
        }
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

    /// Managed volume allows the sink to be muted without losing the user's
    /// intended volume setting.
    ///
    /// This is used to restore the volume when [`unmute`](Self::unmute) is
    /// called.
    ///
    /// If the sink is not muted, this is `None`.
    ///
    /// If the sink is muted, this is `Some(volume)` where `volume` is the
    /// user's intended volume setting, even if the underlying sink's volume is
    /// 0.
    pub(crate) managed_volume: Option<Volume>,
}

impl SpatialAudioSink {
    /// Create a new spatial audio sink.
    pub fn new(sink: SpatialSink) -> Self {
        Self {
            sink,
            managed_volume: None,
        }
    }
}

impl AudioSinkPlayback for SpatialAudioSink {
    fn volume(&self) -> Volume {
        self.managed_volume
            .unwrap_or_else(|| Volume::Linear(self.sink.volume()))
    }

    fn set_volume(&mut self, volume: Volume) {
        if self.is_muted() {
            self.managed_volume = Some(volume);
        } else {
            self.sink.set_volume(volume.to_linear());
        }
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

    fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    fn try_seek(&self, pos: Duration) -> Result<(), SeekError> {
        self.sink.try_seek(pos)
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

    fn is_muted(&self) -> bool {
        self.managed_volume.is_some()
    }

    fn mute(&mut self) {
        self.managed_volume = Some(self.volume());
        self.sink.set_volume(0.0);
    }

    fn unmute(&mut self) {
        if let Some(volume) = self.managed_volume.take() {
            self.sink.set_volume(volume.to_linear());
        }
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

#[cfg(test)]
mod tests {
    use rodio::Sink;

    use super::*;

    fn test_audio_sink_playback<T: AudioSinkPlayback>(mut audio_sink: T) {
        // Test volume
        assert_eq!(audio_sink.volume(), Volume::Linear(1.0)); // default volume
        audio_sink.set_volume(Volume::Linear(0.5));
        assert_eq!(audio_sink.volume(), Volume::Linear(0.5));
        audio_sink.set_volume(Volume::Linear(1.0));
        assert_eq!(audio_sink.volume(), Volume::Linear(1.0));

        // Test speed
        assert_eq!(audio_sink.speed(), 1.0); // default speed
        audio_sink.set_speed(0.5);
        assert_eq!(audio_sink.speed(), 0.5);
        audio_sink.set_speed(1.0);
        assert_eq!(audio_sink.speed(), 1.0);

        // Test playback
        assert!(!audio_sink.is_paused()); // default pause state
        audio_sink.pause();
        assert!(audio_sink.is_paused());
        audio_sink.play();
        assert!(!audio_sink.is_paused());

        // Test toggle playback
        audio_sink.pause(); // start paused
        audio_sink.toggle_playback();
        assert!(!audio_sink.is_paused());
        audio_sink.toggle_playback();
        assert!(audio_sink.is_paused());

        // Test mute
        assert!(!audio_sink.is_muted()); // default mute state
        audio_sink.mute();
        assert!(audio_sink.is_muted());
        audio_sink.unmute();
        assert!(!audio_sink.is_muted());

        // Test volume with mute
        audio_sink.set_volume(Volume::Linear(0.5));
        audio_sink.mute();
        assert_eq!(audio_sink.volume(), Volume::Linear(0.5)); // returns managed volume even though sink volume is 0
        audio_sink.unmute();
        assert_eq!(audio_sink.volume(), Volume::Linear(0.5)); // managed volume is restored

        // Test toggle mute
        audio_sink.toggle_mute();
        assert!(audio_sink.is_muted());
        audio_sink.toggle_mute();
        assert!(!audio_sink.is_muted());
    }

    #[test]
    fn test_audio_sink() {
        let (sink, _queue_rx) = Sink::new_idle();
        let audio_sink = AudioSink::new(sink);
        test_audio_sink_playback(audio_sink);
    }
}
