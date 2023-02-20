use crate::{Audio, AudioSource, Decodable, SpatialAudioSink, SpatialSettings};
use bevy_asset::{Asset, Assets};
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_utils::tracing::warn;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source, SpatialSink};
use std::marker::PhantomData;

use crate::AudioSink;

/// Used internally to play audio on the current "audio device"
///
/// ## Note
///
/// Initializing this resource will leak [`rodio::OutputStream`](rodio::OutputStream)
/// using [`std::mem::forget`].
/// This is done to avoid storing this in the struct (and making this `!Send`)
/// while preventing it from dropping (to avoid halting of audio).
///
/// This is fine when initializing this once (as is default when adding this plugin),
/// since the memory cost will be the same.
/// However, repeatedly inserting this resource into the app will **leak more memory**.
#[derive(Resource)]
pub struct AudioOutput<Source = AudioSource>
where
    Source: Decodable,
{
    stream_handle: Option<OutputStreamHandle>,
    phantom: PhantomData<Source>,
}

impl<Source> Default for AudioOutput<Source>
where
    Source: Decodable,
{
    fn default() -> Self {
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            // We leak `OutputStream` to prevent the audio from stopping.
            std::mem::forget(stream);
            Self {
                stream_handle: Some(stream_handle),
                phantom: PhantomData,
            }
        } else {
            warn!("No audio device found.");
            Self {
                stream_handle: None,
                phantom: PhantomData,
            }
        }
    }
}

impl<Source> AudioOutput<Source>
where
    Source: Asset + Decodable,
    f32: rodio::cpal::FromSample<Source::DecoderItem>,
{
    fn play_source(&self, audio_source: &Source, repeat: bool) -> Option<Sink> {
        self.stream_handle
            .as_ref()
            .and_then(|stream_handle| match Sink::try_new(stream_handle) {
                Ok(sink) => {
                    if repeat {
                        sink.append(audio_source.decoder().repeat_infinite());
                    } else {
                        sink.append(audio_source.decoder());
                    }
                    Some(sink)
                }
                Err(err) => {
                    warn!("Error playing sound: {err:?}");
                    None
                }
            })
    }

    fn play_spatial_source(
        &self,
        audio_source: &Source,
        repeat: bool,
        spatial: SpatialSettings,
    ) -> Option<SpatialSink> {
        self.stream_handle.as_ref().and_then(|stream_handle| {
            match SpatialSink::try_new(
                stream_handle,
                spatial.emitter,
                spatial.left_ear,
                spatial.right_ear,
            ) {
                Ok(sink) => {
                    if repeat {
                        sink.append(audio_source.decoder().repeat_infinite());
                    } else {
                        sink.append(audio_source.decoder());
                    }
                    Some(sink)
                }
                Err(err) => {
                    warn!("Error playing spatial sound: {err:?}");
                    None
                }
            }
        })
    }

    fn try_play_queued(
        &self,
        audio_sources: &Assets<Source>,
        audio: &mut Audio<Source>,
        sinks: &mut Assets<AudioSink>,
        spatial_sinks: &mut Assets<SpatialAudioSink>,
    ) {
        let mut queue = audio.queue.write();
        let len = queue.len();
        let mut i = 0;
        while i < len {
            let config = queue.pop_front().unwrap();
            if let Some(audio_source) = audio_sources.get(&config.source_handle) {
                if let Some(spatial) = config.spatial {
                    if let Some(sink) =
                        self.play_spatial_source(audio_source, config.settings.repeat, spatial)
                    {
                        sink.set_speed(config.settings.speed);
                        sink.set_volume(config.settings.volume);

                        // don't keep the strong handle. there is no way to return it to the user here as it is async
                        let _ = spatial_sinks
                            .set(config.sink_handle, SpatialAudioSink { sink: Some(sink) });
                    }
                } else if let Some(sink) = self.play_source(audio_source, config.settings.repeat) {
                    sink.set_speed(config.settings.speed);
                    sink.set_volume(config.settings.volume);

                    // don't keep the strong handle. there is no way to return it to the user here as it is async
                    let _ = sinks.set(config.sink_handle, AudioSink { sink: Some(sink) });
                }
            } else {
                // audio source hasn't loaded yet. add it back to the queue
                queue.push_back(config);
            }
            i += 1;
        }
    }
}

/// Plays audio currently queued in the [`Audio`] resource through the [`AudioOutput`] resource
pub fn play_queued_audio_system<Source: Asset + Decodable>(
    audio_output: Res<AudioOutput<Source>>,
    audio_sources: Option<Res<Assets<Source>>>,
    mut audio: ResMut<Audio<Source>>,
    mut sinks: ResMut<Assets<AudioSink>>,
    mut spatial_sinks: ResMut<Assets<SpatialAudioSink>>,
) where
    f32: rodio::cpal::FromSample<Source::DecoderItem>,
{
    if let Some(audio_sources) = audio_sources {
        audio_output.try_play_queued(&*audio_sources, &mut *audio, &mut sinks, &mut spatial_sinks);
    };
}
