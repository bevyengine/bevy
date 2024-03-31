use std::{
    fmt::Display,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
};

use bevy_ecs::{prelude::*, world::Command};
use bevy_utils::{
    tracing::{error, trace, warn},
    Duration,
};
use rodio::cpal::{
    self,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, InputCallbackInfo, InputStreamTimestamp, PauseStreamError, PlayStreamError,
    SampleRate, Stream, StreamConfig,
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

        // Pre-allocate buffers which samples can be stored in.
        let buffer_size = if let BufferSize::Fixed(size) = config.buffer_size {
            size as usize
        } else {
            // Over-allocate 1 second of buffer initially
            config.channels as usize * config.sample_rate.0 as usize
        };

        // Sending a zero-filled vector to permit use of copy_from_slice
        let sample_buffer_vec = std::iter::repeat(0.).take(buffer_size).collect::<Vec<_>>();

        while let Ok(()) = sender.try_send(sample_buffer_vec.clone()) {
            // Space left intentionally blank
        }

        // Create a clone of the configuration for the callback
        let config_clone = config.clone();
        let mut start = None;

        let stream = input
            .build_input_stream(
                &config,
                move |data: &[f32], info: &InputCallbackInfo| {
                    let Ok(mut sample_buffer) = rx.try_recv() else {
                        warn!("Audio Input Error: Sample Buffer Unavailable");
                        return;
                    };

                    if sample_buffer.len() >= data.len() {
                        // If handed an appropriate length buffer, can use memcpy...
                        sample_buffer.truncate(data.len());
                        sample_buffer.copy_from_slice(data);
                    } else {
                        // ...otherwise clear and use extension.
                        sample_buffer.clear();
                        sample_buffer.extend_from_slice(data);
                    }

                    let InputStreamTimestamp { capture, callback } = info.timestamp();

                    let start = start.unwrap_or(capture);

                    let capture = capture.duration_since(&start).unwrap_or_default();
                    let callback = callback.duration_since(&start).unwrap_or_default();

                    let event = AudioInputEvent {
                        samples: sample_buffer,
                        capture,
                        callback,
                        channels: config_clone.channels,
                        sample_rate: config_clone.sample_rate.0,
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
    /// Interleaved samples recorded during this frame of audio input.
    pub samples: Vec<f32>,
    /// When this sample was captured by the audio input device relative to the first capture instant.
    pub capture: Duration,
    /// When this sample was processed by the audio subsystem relative to the first capture instant.
    pub callback: Duration,
    /// Number of channels recorded.
    pub channels: u16,
    /// Sample Rate.
    pub sample_rate: u32,
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
        self.samples.chunks_exact(self.channels as usize)
    }

    /// Iterate over all samples for a particular channel.
    pub fn iter_channel(&self, channel: usize) -> impl Iterator<Item = f32> + '_ {
        self.iter().map(move |channels| channels[channel])
    }

    /// Gets the number of samples collected during this input frame.
    pub fn len(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    /// Returns `true` if this input frame recorded no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Gets the amount of time this input frame represents.
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.len() as u64).div_f32(self.sample_rate as f32)
    }
}

/// Marshals [`AudioInputEvent`] out of [`AudioInputStream`] into an [`EventWriter`].
pub fn handle_input_stream(
    mut writer: EventWriter<AudioInputEvent>,
    stream: Option<NonSendMut<AudioInputStream>>,
) {
    if let Some(stream) = stream {
        let mut new_buffer_size = 0;

        for event in stream.iter() {
            new_buffer_size = new_buffer_size.max(event.samples.len());
            writer.send(event);
        }

        let new_buffer = std::iter::repeat(0.)
            .take(new_buffer_size)
            .collect::<Vec<_>>();

        while let Ok(()) = stream.sender.try_send(new_buffer.clone()) {
            // Space left intentionally blank
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
