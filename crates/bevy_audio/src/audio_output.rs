use crate::{
    AudioSourceBundle, Decodable, GlobalVolume, PlaybackMode, PlaybackSettings, SpatialAudioSink,
    SpatialAudioSourceBundle, SpatialSettings, Volume,
};
use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_utils::tracing::warn;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source, SpatialSink};

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
pub(crate) struct AudioOutput {
    stream_handle: Option<OutputStreamHandle>,
}

impl Default for AudioOutput {
    fn default() -> Self {
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            // We leak `OutputStream` to prevent the audio from stopping.
            std::mem::forget(stream);
            Self {
                stream_handle: Some(stream_handle),
            }
        } else {
            warn!("No audio device found.");
            Self {
                stream_handle: None,
            }
        }
    }
}

/// Marker for internal use, to despawn entities when playback finishes.
#[derive(Component)]
pub struct PlaybackDespawnMarker;

/// Marker for internal use, to remove audio components when playback finishes.
#[derive(Component)]
pub struct PlaybackRemoveMarker;

/// Plays "queued" audio through the [`AudioOutput`] resource.
///
/// "Queued" audio is any audio entity (with the components from
/// [`AudioBundle`][crate::AudioBundle] or [`SpatialAudioBundle`][crate::SpatialAudioBundle])
/// that does not have an [`AudioSink`]/[`SpatialAudioSink`] component.
///
/// This system detects such entities, checks if their source asset
/// data is available, and creates/inserts the sink.
pub(crate) fn play_queued_audio_system<Source: Asset + Decodable>(
    audio_output: Res<AudioOutput>,
    audio_sources: Res<Assets<Source>>,
    global_volume: Res<GlobalVolume>,
    query_nonplaying: Query<
        (
            Entity,
            &Handle<Source>,
            &PlaybackSettings,
            Option<&SpatialSettings>,
        ),
        (Without<AudioSink>, Without<SpatialAudioSink>),
    >,
    mut commands: Commands,
) where
    f32: rodio::cpal::FromSample<Source::DecoderItem>,
{
    let Some(stream_handle) = audio_output.stream_handle.as_ref() else {
        // audio output unavailable; cannot play sound
        return;
    };

    for (entity, source_handle, settings, spatial) in &query_nonplaying {
        if let Some(audio_source) = audio_sources.get(source_handle) {
            // audio data is available (has loaded), begin playback and insert sink component
            if let Some(spatial) = spatial {
                match SpatialSink::try_new(
                    stream_handle,
                    spatial.emitter,
                    spatial.left_ear,
                    spatial.right_ear,
                ) {
                    Ok(sink) => {
                        sink.set_speed(settings.speed);
                        match settings.volume {
                            Volume::Relative(vol) => {
                                sink.set_volume(vol.0 * global_volume.volume.0);
                            }
                            Volume::Absolute(vol) => sink.set_volume(vol.0),
                        }
                        if settings.paused {
                            sink.pause();
                        }
                        match settings.mode {
                            PlaybackMode::Loop => {
                                sink.append(audio_source.decoder().repeat_infinite());
                                commands.entity(entity).insert(SpatialAudioSink { sink });
                            }
                            PlaybackMode::Once => {
                                sink.append(audio_source.decoder());
                                commands.entity(entity).insert(SpatialAudioSink { sink });
                            }
                            PlaybackMode::Despawn => {
                                sink.append(audio_source.decoder());
                                commands
                                    .entity(entity)
                                    // PERF: insert as bundle to reduce archetype moves
                                    .insert((SpatialAudioSink { sink }, PlaybackDespawnMarker));
                            }
                            PlaybackMode::Remove => {
                                sink.append(audio_source.decoder());
                                commands
                                    .entity(entity)
                                    // PERF: insert as bundle to reduce archetype moves
                                    .insert((SpatialAudioSink { sink }, PlaybackRemoveMarker));
                            }
                        };
                    }
                    Err(err) => {
                        warn!("Error playing spatial sound: {err:?}");
                    }
                }
            } else {
                match Sink::try_new(stream_handle) {
                    Ok(sink) => {
                        sink.set_speed(settings.speed);
                        match settings.volume {
                            Volume::Relative(vol) => {
                                sink.set_volume(vol.0 * global_volume.volume.0);
                            }
                            Volume::Absolute(vol) => sink.set_volume(vol.0),
                        }
                        if settings.paused {
                            sink.pause();
                        }
                        match settings.mode {
                            PlaybackMode::Loop => {
                                sink.append(audio_source.decoder().repeat_infinite());
                                commands.entity(entity).insert(AudioSink { sink });
                            }
                            PlaybackMode::Once => {
                                sink.append(audio_source.decoder());
                                commands.entity(entity).insert(AudioSink { sink });
                            }
                            PlaybackMode::Despawn => {
                                sink.append(audio_source.decoder());
                                commands
                                    .entity(entity)
                                    // PERF: insert as bundle to reduce archetype moves
                                    .insert((AudioSink { sink }, PlaybackDespawnMarker));
                            }
                            PlaybackMode::Remove => {
                                sink.append(audio_source.decoder());
                                commands
                                    .entity(entity)
                                    // PERF: insert as bundle to reduce archetype moves
                                    .insert((AudioSink { sink }, PlaybackRemoveMarker));
                            }
                        };
                    }
                    Err(err) => {
                        warn!("Error playing sound: {err:?}");
                    }
                }
            }
        }
    }
}

pub(crate) fn cleanup_finished_audio<T: Decodable + Asset>(
    mut commands: Commands,
    query_nonspatial_despawn: Query<
        (Entity, &AudioSink),
        (With<PlaybackDespawnMarker>, With<Handle<T>>),
    >,
    query_spatial_despawn: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackDespawnMarker>, With<Handle<T>>),
    >,
    query_nonspatial_remove: Query<
        (Entity, &AudioSink),
        (With<PlaybackRemoveMarker>, With<Handle<T>>),
    >,
    query_spatial_remove: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackRemoveMarker>, With<Handle<T>>),
    >,
) {
    for (entity, sink) in &query_nonspatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_spatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_nonspatial_remove {
        if sink.sink.empty() {
            commands
                .entity(entity)
                .remove::<(AudioSourceBundle<T>, AudioSink, PlaybackRemoveMarker)>();
        }
    }
    for (entity, sink) in &query_spatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                SpatialAudioSourceBundle<T>,
                SpatialAudioSink,
                PlaybackRemoveMarker,
            )>();
        }
    }
}

/// Run Condition to only play audio if the audio output is available
pub(crate) fn audio_output_available(audio_output: Res<AudioOutput>) -> bool {
    audio_output.stream_handle.is_some()
}
