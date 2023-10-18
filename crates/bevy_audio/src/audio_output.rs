use crate::{
    AudioSourceBundle, Decodable, GlobalVolume, PlaybackMode, PlaybackSettings, SpatialAudioSink,
    SpatialListener, SpatialScale, Volume,
};
use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::Vec3;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::tracing::warn;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source, SpatialSink};

use crate::AudioSink;

/// Used internally to play audio on the current "audio device"
///
/// ## Note
///
/// Initializing this resource will leak [`rodio::OutputStream`]
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

#[derive(SystemParam)]
pub(crate) struct EarPositions<'w, 's> {
    pub(crate) query: Query<'w, 's, (Entity, &'static GlobalTransform, &'static SpatialListener)>,
    pub(crate) scale: Res<'w, SpatialScale>,
}
impl<'w, 's> EarPositions<'w, 's> {
    /// Gets a set of transformed and scaled ear positions.
    ///
    /// If there are no listeners, use the default values. If a user has added multiple
    /// listeners for whatever reason, we will return the first value.
    pub(crate) fn get(&self) -> (Vec3, Vec3) {
        let (left_ear, right_ear) = self
            .query
            .iter()
            .next()
            .map(|(_, transform, settings)| {
                (
                    transform.transform_point(settings.left_ear_offset) * self.scale.0,
                    transform.transform_point(settings.right_ear_offset) * self.scale.0,
                )
            })
            .unwrap_or_else(|| {
                let settings = SpatialListener::default();
                (
                    (settings.left_ear_offset * self.scale.0),
                    (settings.right_ear_offset * self.scale.0),
                )
            });

        (left_ear, right_ear)
    }

    pub(crate) fn multiple_listeners(&self) -> bool {
        self.query.iter().len() > 1
    }
}

/// Plays "queued" audio through the [`AudioOutput`] resource.
///
/// "Queued" audio is any audio entity (with the components from
/// [`AudioBundle`][crate::AudioBundle] that does not have an
/// [`AudioSink`]/[`SpatialAudioSink`] component.
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
            Option<&GlobalTransform>,
        ),
        (Without<AudioSink>, Without<SpatialAudioSink>),
    >,
    ear_positions: EarPositions,
    mut commands: Commands,
) where
    f32: rodio::cpal::FromSample<Source::DecoderItem>,
{
    let Some(stream_handle) = audio_output.stream_handle.as_ref() else {
        // audio output unavailable; cannot play sound
        return;
    };

    for (entity, source_handle, settings, maybe_emitter_transform) in &query_nonplaying {
        if let Some(audio_source) = audio_sources.get(source_handle) {
            // audio data is available (has loaded), begin playback and insert sink component
            if settings.spatial {
                let (left_ear, right_ear) = ear_positions.get();

                // We can only use one `SpatialListener`. If there are more than that, then
                // the user may have made a mistake.
                if ear_positions.multiple_listeners() {
                    warn!(
                        "Multiple SpatialListeners found. Using {:?}.",
                        ear_positions.query.iter().next().unwrap().0
                    );
                }

                let emitter_translation = maybe_emitter_transform
                    .map(|t| (t.translation() * ear_positions.scale.0).into())
                    .unwrap_or_else(|| {
                        warn!("Spatial AudioBundle with no GlobalTransform component. Using zero.");
                        Vec3::ZERO.into()
                    });

                match SpatialSink::try_new(
                    stream_handle,
                    emitter_translation,
                    left_ear.into(),
                    right_ear.into(),
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
            commands
                .entity(entity)
                .remove::<(AudioSourceBundle<T>, SpatialAudioSink, PlaybackRemoveMarker)>();
        }
    }
}

/// Run Condition to only play audio if the audio output is available
pub(crate) fn audio_output_available(audio_output: Res<AudioOutput>) -> bool {
    audio_output.stream_handle.is_some()
}

/// Updates spatial audio sinks when emitter positions change.
pub(crate) fn update_emitter_positions(
    mut emitters: Query<(&mut GlobalTransform, &SpatialAudioSink), Changed<GlobalTransform>>,
    spatial_scale: Res<SpatialScale>,
) {
    for (transform, sink) in emitters.iter_mut() {
        let translation = transform.translation() * spatial_scale.0;
        sink.set_emitter_position(translation);
    }
}

/// Updates spatial audio sink ear positions when spatial listeners change.
pub(crate) fn update_listener_positions(
    mut emitters: Query<&SpatialAudioSink>,
    changed_listener: Query<
        (),
        (
            Or<(Changed<SpatialListener>, Changed<GlobalTransform>)>,
            With<SpatialListener>,
        ),
    >,
    ear_positions: EarPositions,
) {
    if !ear_positions.scale.is_changed() && changed_listener.is_empty() {
        return;
    }

    let (left_ear, right_ear) = ear_positions.get();

    for sink in emitters.iter_mut() {
        sink.set_ears_position(left_ear, right_ear);
    }
}
