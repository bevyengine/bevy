//! Audio support for the game engine Bevy
//!
//! ```no_run
//! # use bevy_ecs::prelude::*;
//! # use bevy_audio::{AudioBundle, AudioPlugin, PlaybackSettings};
//! # use bevy_asset::{AssetPlugin, AssetServer};
//! # use bevy_app::{App, AppExit, NoopPluginGroup as MinimalPlugins, Startup};
//! fn main() {
//!    App::new()
//!         .add_plugins((MinimalPlugins, AssetPlugin::default(), AudioPlugin::default()))
//!         .add_systems(Startup, play_background_audio)
//!         .run();
//! }
//!
//! fn play_background_audio(asset_server: Res<AssetServer>, mut commands: Commands) {
//!     commands.spawn(AudioBundle {
//!         source: asset_server.load("background_audio.ogg"),
//!         settings: PlaybackSettings::LOOP,
//!     });
//! }
//! ```
//!
//! # Fundamentals of working with audio
//! 
//! This section is of interest to anybody working with the lowest levels of the audio engine.
//! 
//! If you're looking for using effects that bevy already provides, or are working within Bevy's
//! ECS, then these guidelines do not apply. This concerns code that is going to run in tandem
//! with the audio driver, that is code directly talking to audio I/O or while implementing audio
//! effects.
//! 
//! This section applies to the equivalent in abstraction level to working with nodes in the render
//! graph, and not manipulating entities with meshes and materials.
//! 
//! Note that these guidelines are general to any audio programming application, and not just Bevy.
//!
//! ## Under the trunk
//!
//! Some parts of the audio engine run on a high-priority thread, synchronized with the audio driver
//! as it requests audio data (or, in the case of input, notifies of new capture data).
//!
//! How often these callbacks are run by the driver depends on the audio stream settings; namely the
//! **sample rate** and the **buffer size**. These parameters are passed in as configuration when
//! creating a stream.
//!
//! Typical values for buffer size and sample rate are 512 samples at 48 kHz. This means that every 512
//! samples of audio the driver is going to send to the sound card the output callback function is run
//! in this high-priority audio thread. Every second, as dictated by the sample rate, the sound card
//! needs 48 000 samples of audio data. This means that we can expect the callback function to be run
//! every `512/(48000 Hz)` or 10.666... ms.
//!
//! This figure is also the latency of the audio engine, that is, how much time it takes between a
//! user interaction and hearing the effects out the speakers. Therefore, there is a "tug of war"
//! between decreasing the buffer size for latency reasons, and increasing it for performance reasons.
//! The threshold for instantaneity in audio is around 15 ms, which is why 512 is a good value for
//! interactive applications.
//!
//! ## Real-time programming
//!
//! The parts of the code running in the audio thread have exactly `buffer_size/samplerate` seconds to
//! complete, beyond which the audio driver outputs silence (or worse, the previous buffer output, or
//! garbage data), which the user perceives as a glitch and severely deteriorates the quality of the
//! audio output of the engine. It is therefore critical to work with code that is guaranteed to finish
//! in that time.
//!
//! One step to achieving this is making sure that all machines across the spectrum of supported CPUs can
//! reliably perform the computations needed for the game in that amount of time, and play around with the
//! buffer size to find the best compromise between latency and performance. Another is to conditionally
//! enable certain effects for more powerful CPUs, when that is possible.
//!
//! But the main step is to write code run in the audio thread following real-time programming guidelines.
//! Real-time programming is a set of constraints on code and structures that guarantees the code completes
//! at some point, ie. it cannot be stuck in an infinite loop nor can it trigger a deadlock situation.
//!
//! Practically, the main components of real-time programming are about using wait-free and lock-free
//! structures. Examples of things that are *not* correct in real-time programming are:
//!
//! - Allocating anything on the heap (that is, no direct or indirect creation of a `Vec`, `Box`, or any
//!   standard collection, as they are not designed with real-time programming in mind)
//! - Locking a mutex
//! - Generally, any kind of system call gives the OS the opportunity to pause the thread, which is an
//!   unbounded operation as we don't know how long the thread is going to be paused for
//! - Waiting by looping until some condition is met (also called a spinloop or a spinlock)
//!
//! Writing wait-free and lock-free structures is a hard task, and difficult to get correct; however many
//! structures already exists, and can be directly used. There are crates for most replacements of standard
//! collections.
//!
//! ## Where in the code should real-time programming principles be applied?
//!
//! Any code that is directly or indirectly called by audio threads, needs to be real-time safe.
//!
//! For the Bevy engine, that is:
//!
//! - In the callback of `cpal::Stream::build_input_stream` and `cpal::Stream::build_output_stream`, and all
//!   functions called from them
//! - In implementations of the [`Source`] trait, and all functions called from it
//!
//! Code that is run in Bevy systems do not need to be real-time safe, as they are not run in the audio thread,
//! but in the main game loop thread.
//!
//! ## Communication with the audio thread
//!
//! To be able to to anything useful with audio, the thread has to be able to communicate with the rest of
//! the system, ie. update parameters, send/receive audio data, etc., and all of that needs to be done within
//! the constraints of real-time programming, of course.
//!
//! ### Audio parameters
//!
//! In most cases, audio parameters can be represented by an atomic floating point value, where the game loop
//! updates the parameter, and it gets picked up when processing the next buffer. The downside to this approach
//! is that the audio only changes once per audio callback, and results in a noticeable "stair-step " motion
//! of the parameter. The latter can be mitigated by "smoothing" the change over time, using a tween or
//! linear/exponential smoothing.
//!
//! Precise timing for non-interactive events (ie. on the beat) need to be setup using a clock backed by the
//! audio driver -- that is, counting the number of samples processed, and deriving the time elapsed by diving
//! by the sample rate to get the number of seconds elapsed. The precise sample at which the parameter needs to
//! be changed can then be computed.
//!
//! Both interactive and precise events are hard to do, and need very low latency (ie. 64 or 128 samples for ~2
//! ms of latency). It is fundamentally impossible to react to user event the very moment it is registered.
//!
//! ### Audio data
//!
//! Audio data is generally transferred between threads with circular buffers, as they are simple to implement,
//! fast enough for 99% of use-cases, and are both wait-free and lock-free. The only difficulty in using
//! circular buffers is how big they should be; however even going for 1 s of audio costs ~50 kB of memory,
//! which is small enough to not be noticeable even with potentially 100s of those buffers.
//!
//! ## Additional resources for audio programming
//!
//! More in-depth article about audio programming: <http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing>
//!
//! Awesome Audio DSP: <https://github.com/BillyDM/awesome-audio-dsp>

#![forbid(unsafe_code)]

mod audio;
mod audio_output;
mod audio_source;
mod pitch;
mod sinks;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AudioBundle, AudioSink, AudioSinkPlayback, AudioSource, AudioSourceBundle, Decodable,
        GlobalVolume, Pitch, PitchBundle, PlaybackSettings, SpatialAudioSink, SpatialListener,
    };
}

pub use audio::*;
pub use audio_source::*;
pub use pitch::*;

pub use rodio::cpal::Sample as CpalSample;
pub use rodio::source::Source;
pub use rodio::Sample;
pub use sinks::*;

use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp};
use bevy_ecs::prelude::*;
use bevy_transform::TransformSystem;

use audio_output::*;

/// Set for the audio playback systems, so they can share a run condition
#[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct AudioPlaySet;

/// Adds support for audio playback to a Bevy Application
///
/// Insert an [`AudioBundle`] onto your entities to play audio.
#[derive(Default)]
pub struct AudioPlugin {
    /// The global volume for all audio entities.
    pub global_volume: GlobalVolume,
    /// The scale factor applied to the positions of audio sources and listeners for
    /// spatial audio.
    pub default_spatial_scale: SpatialScale,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Volume>()
            .register_type::<GlobalVolume>()
            .register_type::<SpatialListener>()
            .register_type::<DefaultSpatialScale>()
            .register_type::<PlaybackMode>()
            .register_type::<PlaybackSettings>()
            .insert_resource(self.global_volume)
            .insert_resource(DefaultSpatialScale(self.default_spatial_scale))
            .configure_sets(
                PostUpdate,
                AudioPlaySet
                    .run_if(audio_output_available)
                    .after(TransformSystem::TransformPropagate), // For spatial audio transforms
            )
            .add_systems(
                PostUpdate,
                (update_emitter_positions, update_listener_positions).in_set(AudioPlaySet),
            )
            .init_resource::<AudioOutput>();

        #[cfg(any(feature = "mp3", feature = "flac", feature = "wav", feature = "vorbis"))]
        {
            app.add_audio_source::<AudioSource>();
            app.init_asset_loader::<AudioLoader>();
        }

        app.add_audio_source::<Pitch>();
    }
}

impl AddAudioSource for App {
    fn add_audio_source<T>(&mut self) -> &mut Self
    where
        T: Decodable + Asset,
        f32: rodio::cpal::FromSample<T::DecoderItem>,
    {
        self.init_asset::<T>().add_systems(
            PostUpdate,
            (play_queued_audio_system::<T>, cleanup_finished_audio::<T>).in_set(AudioPlaySet),
        );
        self
    }
}
