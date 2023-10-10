use std::sync::mpsc::{channel, Receiver};

use bevy_ecs::prelude::*;
use bevy_utils::{
    tracing::{trace, warn},
    Duration,
};
use rodio::cpal::{
    self,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, Stream, StreamConfig,
};

/// Used internally to retrieve audio on the current "audio input device". When this is dropped,
/// the underlying stream and event system are also dropped.
pub struct AudioInputStream {
    config: StreamConfig,
    stream: Stream,
    receiver: Receiver<AudioInputEvent>,
}

/// A frame of samples from an audio input source.
#[derive(Event, Clone)]
pub struct AudioInputEvent {
    /// Interleaved samples recorded during this frame of audio input. Please check [`config`](`Self::config`)
    /// for channel information and sample rate.
    pub samples: Vec<f32>,
    /// Information about this sample frame as provided by [`cpal`].
    pub info: InputCallbackInfo,
    /// Configuration used for the input device which recorded this sample frame.
    pub config: StreamConfig,
}

impl AudioInputStream {
    /// Try to create a default audio input stream. Returns [`None`] if a default input stream
    /// cannot be created.
    pub fn try_default() -> Option<Self> {
        let host = cpal::default_host();
        let input = host.default_input_device()?;
        let config = input.default_input_config().ok()?.config();

        trace!("Got Audio Input Device: {:?}: {:?}", input.name(), config);

        let (tx, receiver) = channel();

        // Create a clone of the configuration for the callback
        let config_clone = config.clone();

        let stream = input
            .build_input_stream(
                &config,
                move |data: &[f32], info: &InputCallbackInfo| {
                    let event = AudioInputEvent {
                        samples: data.into_iter().cloned().collect(),
                        info: info.clone(),
                        config: config_clone.clone(),
                    };

                    if let Err(error) = tx.send(event) {
                        warn!("Audio Input Error: {}", error)
                    };
                },
                move |error| {
                    warn!("Audio Input Error: {}", error);
                },
                None,
            )
            .ok()?;

        // The term "play" does not mean output to the stream, it instead means "start" listening.
        if let Err(error) = stream.play() {
            warn!("Audio Input Error: {}", error);
        }

        Some(Self {
            config,
            stream,
            receiver,
        })
    }

    /// Returns an [`Iterator`] over newly received [`AudioInputEvent`]'s. This does not block
    /// if new events aren't available. This clears current events.
    pub fn iter(&self) -> impl Iterator<Item = AudioInputEvent> + '_ {
        self.receiver.try_iter()
    }

    /// Returns the [configuration information](`StreamConfig`) of this [`AudioInputStream`].
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }

    /// Provides access to the underlying [`Stream`].
    pub fn stream(&self) -> &Stream {
        &self.stream
    }
}

impl AudioInputEvent {
    /// Iterate over each sample, and each channel within that sample.
    pub fn iter(&self) -> impl Iterator<Item = &[f32]> {
        self.samples.chunks_exact(self.config.channels as usize)
    }

    /// Iterate over all samples for a particular channel.
    pub fn iter_channel(&self, channel: usize) -> impl Iterator<Item = f32> + '_ {
        self.iter().map(move |channels| channels[channel])
    }

    /// Gets the number of samples collected during this input frame.
    pub fn len(&self) -> usize {
        self.samples.len() / self.config.channels as usize
    }

    /// Gets the amount of time this input frame represents.
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.len() as u64).div_f32(self.config.sample_rate.0 as f32)
    }
}

/// Marshals [`AudioInputEvent`] out of [`AudioInputStream`] into an [`EventWriter`].
pub fn handle_input_stream(
    mut writer: EventWriter<AudioInputEvent>,
    stream: Option<NonSendMut<AudioInputStream>>,
) {
    if let Some(stream) = stream {
        writer.send_batch(stream.iter());
    }
}
