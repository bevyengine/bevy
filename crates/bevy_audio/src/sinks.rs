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
#[derive(Component)]
pub struct AudioSink {
    // This field is an Option in order to allow us to have a safe drop that will detach the sink.
    // It will never be None during its life
    pub(crate) sink: Option<Sink>,
}

impl Drop for AudioSink {
    fn drop(&mut self) {
        self.sink.take().unwrap().detach();
    }
}

impl AudioSinkPlayback for AudioSink {
    fn volume(&self) -> f32 {
        self.sink.as_ref().unwrap().volume()
    }

    fn set_volume(&self, volume: f32) {
        self.sink.as_ref().unwrap().set_volume(volume);
    }

    fn speed(&self) -> f32 {
        self.sink.as_ref().unwrap().speed()
    }

    fn set_speed(&self, speed: f32) {
        self.sink.as_ref().unwrap().set_speed(speed);
    }

    fn play(&self) {
        self.sink.as_ref().unwrap().play();
    }

    fn pause(&self) {
        self.sink.as_ref().unwrap().pause();
    }

    fn is_paused(&self) -> bool {
        self.sink.as_ref().unwrap().is_paused()
    }

    fn stop(&self) {
        self.sink.as_ref().unwrap().stop();
    }

    fn empty(&self) -> bool {
        self.sink.as_ref().unwrap().empty()
    }
}

/// Used to control spatial audio during playback.
///
/// Bevy inserts this component onto your entities when it begins playing an audio source.
/// Use [`SpatialAudioBundle`][crate::SpatialAudioBundle] to trigger that to happen.
///
/// You can use this component to modify the playback settings while the audio is playing.
#[derive(Component)]
pub struct SpatialAudioSink {
    // This field is an Option in order to allow us to have a safe drop that will detach the sink.
    // It will never be None during its life
    pub(crate) sink: Option<SpatialSink>,
}

impl Drop for SpatialAudioSink {
    fn drop(&mut self) {
        self.sink.take().unwrap().detach();
    }
}

impl AudioSinkPlayback for SpatialAudioSink {
    fn volume(&self) -> f32 {
        self.sink.as_ref().unwrap().volume()
    }

    fn set_volume(&self, volume: f32) {
        self.sink.as_ref().unwrap().set_volume(volume);
    }

    fn speed(&self) -> f32 {
        self.sink.as_ref().unwrap().speed()
    }

    fn set_speed(&self, speed: f32) {
        self.sink.as_ref().unwrap().set_speed(speed);
    }

    fn play(&self) {
        self.sink.as_ref().unwrap().play();
    }

    fn pause(&self) {
        self.sink.as_ref().unwrap().pause();
    }

    fn is_paused(&self) -> bool {
        self.sink.as_ref().unwrap().is_paused()
    }

    fn stop(&self) {
        self.sink.as_ref().unwrap().stop();
    }

    fn empty(&self) -> bool {
        self.sink.as_ref().unwrap().empty()
    }
}

impl SpatialAudioSink {
    /// Set the two ears position.
    pub fn set_ears_position(&self, left_position: Vec3, right_position: Vec3) {
        let sink = self.sink.as_ref().unwrap();
        sink.set_left_ear_position(left_position.to_array());
        sink.set_right_ear_position(right_position.to_array());
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
        self.sink
            .as_ref()
            .unwrap()
            .set_emitter_position(position.to_array());
    }
}
