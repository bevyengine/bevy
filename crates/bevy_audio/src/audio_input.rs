use std::{
    fmt::Display,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
};

use bevy_ecs::{prelude::*, world::Command};
use bevy_utils::{
    tracing::{error, trace, warn},
    Duration,
};
use rodio::{
    cpal::{
        self,
        traits::{DeviceTrait, HostTrait, StreamTrait},
        BufferSize, InputCallbackInfo, PauseStreamError, PlayStreamError, SampleRate, Stream,
        StreamConfig,
    },
    Device,
};

/// Platform agnostic representation of an audio input device.
pub struct AudioInput {
    device: Device,
    /// Number of channels to record.
    pub channels: Option<u16>,
    /// Sample Rate.
    pub sample_rate: Option<u32>,
    /// Number of samples to buffer before firing an event.
    pub buffer_size: Option<u32>,
    /// Number of events to allow in-flight before dropping samples.
    pub event_capacity: Option<usize>,
}

impl From<Device> for AudioInput {
    fn from(device: Device) -> Self {
        Self {
            device,
            channels: None,
            sample_rate: None,
            buffer_size: None,
            event_capacity: None,
        }
    }
}

impl AudioInput {
    /// Try to get the name of this input device.
    pub fn get_name(&self) -> Option<String> {
        self.device.name().ok()
    }

    /// Get the underlying [`Device`].
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Try to get an [`AudioInputStream`] from this device.
    pub fn get_stream(&self) -> Option<AudioInputStream> {
        let input = &self.device;

        let mut config = input.default_input_config().ok()?.config();

        if let Some(channels) = self.channels {
            config.channels = channels;
        }

        if let Some(sample_rate) = self.sample_rate {
            config.sample_rate = SampleRate(sample_rate);
        }

        if let Some(buffer_size) = self.buffer_size {
            config.buffer_size = BufferSize::Fixed(buffer_size);
        }

        let event_capacity = self.event_capacity.unwrap_or(5);

        trace!("Got Audio Input Device: {:?}: {:?}", input.name(), config);

        // Sync Channel ensures no allocation during transmission of buffers
        let (tx, receiver) = sync_channel(event_capacity);
        let (sender, rx) = sync_channel::<Vec<f32>>(event_capacity);

        let sample_buffer_vec = if let BufferSize::Fixed(size) = config.buffer_size {
            Vec::with_capacity(size as usize)
        } else {
            // Over-allocate 1 second of buffer initially
            Vec::with_capacity(config.channels as usize * config.sample_rate.0 as usize)
        };

        for _ in 0..event_capacity {
            let Ok(()) = sender.try_send(sample_buffer_vec.clone()) else {
                break;
            };
        }

        // Create a clone of the configuration for the callback
        let config_clone = config.clone();

        let stream = input
            .build_input_stream(
                &config,
                move |data: &[f32], info: &InputCallbackInfo| {
                    let Ok(mut sample_buffer) = rx.try_recv() else {
                        warn!("Audio Input Error: Sample Buffer Unavailable");
                        return;
                    };

                    sample_buffer.extend_from_slice(data);

                    let event = AudioInputEvent {
                        samples: sample_buffer,
                        info: info.clone(),
                        config: config_clone.clone(),
                    };

                    if let Err(error) = tx.try_send(event) {
                        warn!("Audio Input Error: {}", error);
                    };
                },
                move |error| {
                    warn!("Audio Input Error: {}", error);
                },
                None,
            )
            .ok()?;

        Some(AudioInputStream {
            config,
            stream,
            receiver,
            sender,
        })
    }
}

/// Collection of possible [`AudioInput`] devices, if any.
pub struct AudioInputOptions {
    default: AudioInput,
    all: Vec<AudioInput>,
}

impl AudioInputOptions {
    /// Try to get the available [`AudioInput`] devices, if any input devices are present.
    pub fn get() -> Option<Self> {
        let host = cpal::default_host();

        let device = host.default_input_device()?;
        let default = device.into();

        let all = host.input_devices().ok()?.map(AudioInput::from).collect();

        Some(Self { default, all })
    }

    /// Get the default [`AudioInput`].
    pub fn default_input(&self) -> &AudioInput {
        &self.default
    }

    /// Get the default [`AudioInput`].
    pub fn default_input_mut(&mut self) -> &mut AudioInput {
        &mut self.default
    }

    /// Iterate over all available [`AudioInputs`](AudioInput).
    pub fn iter(&self) -> impl Iterator<Item = &AudioInput> {
        self.all.iter()
    }

    /// Iterate over all available [`AudioInputs`](AudioInput).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut AudioInput> {
        self.all.iter_mut()
    }
}

/// Error returned if an [`AudioInputStream`] could not start recording.
#[derive(Debug)]
pub struct StartRecordingError(PlayStreamError);

impl From<PlayStreamError> for StartRecordingError {
    fn from(value: PlayStreamError) -> Self {
        Self(value)
    }
}

impl Display for StartRecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Error returned if an [`AudioInputStream`] could not stop recording.
#[derive(Debug)]
pub struct StopRecordingError(PauseStreamError);

impl From<PauseStreamError> for StopRecordingError {
    fn from(value: PauseStreamError) -> Self {
        Self(value)
    }
}

impl Display for StopRecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Used internally to retrieve audio on the current "audio input device". When this is dropped,
/// the underlying stream and event system are also dropped.
pub struct AudioInputStream {
    config: StreamConfig,
    stream: Stream,
    receiver: Receiver<AudioInputEvent>,
    sender: SyncSender<Vec<f32>>,
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

    /// Start recording audio from this input.
    pub fn start(&self) -> Result<(), StartRecordingError> {
        self.stream.play()?;
        Ok(())
    }

    /// Stop recording audio from this input.
    pub fn stop(&self) -> Result<(), StopRecordingError> {
        self.stream.pause()?;
        Ok(())
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

    /// Returns `true` if this input frame recorded no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
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
        for event in stream.iter() {
            let new_buffer = Vec::with_capacity(event.samples.len());
            if let Err(_) = stream.sender.try_send(new_buffer) {
                warn!("Audio Input Error: Could not provide Sample Buffer")
            }
            writer.send(event);
        }
    }
}

/// Requests that an audio input device start recording.
pub struct AudioInputStreamStart;

impl Command for AudioInputStreamStart {
    fn apply(self, world: &mut World) {
        if let Some(stream) = world.get_non_send_resource::<AudioInputStream>() {
            if let Err(error) = stream.start() {
                warn!("Audio Input Error: {}", error);
            }
        } else {
            trace!("Setting up Audio Input with Default Device");

            let Some(options) = AudioInputOptions::get() else {
                error!("No Audio Input Devices Available");
                return;
            };

            let Some(stream) = options.default_input().get_stream() else {
                error!("Default Audio Input Device Unavailable");
                return;
            };

            if let Err(error) = stream.start() {
                warn!("Audio Input Error: {}", error);
            }

            world.insert_non_send_resource(stream);
        }
    }
}

/// Requests that an audio input device stop recording.
pub struct AudioInputStreamStop;

impl Command for AudioInputStreamStop {
    fn apply(self, world: &mut World) {
        let Some(stream) = world.get_non_send_resource::<AudioInputStream>() else {
            trace!("Called Stop on Audio Input Stream when None Available");
            return;
        };

        if let Err(error) = stream.stop() {
            warn!("Audio Input Error: {}", error);
        }
    }
}

/// Provides methods for controlling an [`AudioInputStream`].
pub trait AudioInputStreamCommands {
    /// Requests that an audio input device start recording.
    fn start_recording_audio(&mut self);

    /// Requests that an audio input device stop recording.
    fn stop_recording_audio(&mut self);
}

impl<'w, 's> AudioInputStreamCommands for Commands<'w, 's> {
    fn start_recording_audio(&mut self) {
        self.add(AudioInputStreamStart);
    }

    fn stop_recording_audio(&mut self) {
        self.add(AudioInputStreamStop);
    }
}
