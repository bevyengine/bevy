use bevy_ecs::event::Events;
use rodio::Sink;
use std::fmt;
use std::fmt::Debug;

use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::world::{World, WorldBorrowMut};
use bevy_utils::tracing::info;

use crate::{AudioOutput, AudioSource, Decodable};

/// Used internally to play audio lists with simple control event on the current "audio device"
/// This could be used for background loop music
#[derive(Default)]
pub struct AssetPlayer {
    audios: Vec<Handle<AudioSource>>,
    sink: Option<Sink>,
    is_loop: bool,
    output: AudioOutput<AudioSource>,
}

impl AssetPlayer {
    fn play_once<T: Asset + Decodable>(&mut self, audio_source: &T) {
        if let Some(h) = &self.output.stream_handle {
            let sink = Sink::try_new(h).unwrap();
            sink.append(audio_source.decoder());
            sink.detach()
        }
    }

    fn append_source<T: Asset + Decodable>(&mut self, audio_source: &T) {
        if let Some(stream_handle) = &self.output.stream_handle {
            if self.sink.is_none() {
                let sink = Sink::try_new(stream_handle).unwrap();
                sink.append(audio_source.decoder());
                self.sink = Some(sink)
            } else if let Some(sink) = self.sink.as_mut() {
                sink.append(audio_source.decoder());
            }
        }
    }

    /// stop audio play
    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.stop();
        }
    }

    /// pause audio play
    pub fn pause(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.pause();
        }
    }

    /// resume audio play
    pub fn resume(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.play();
        }
    }

    fn try_handle_events<Source: Asset + Decodable>(
        &mut self,
        audio_sources: &Assets<Source>,
        mut events: WorldBorrowMut<Events<PlayEvent>>,
    ) {
        let mut pending = vec![];
        for ev in events.get_reader().iter(&events) {
            info!("recv play event {:?}", ev);
            match *ev {
                PlayEvent::Resume => {
                    self.resume();
                }
                PlayEvent::Pause => {
                    self.pause();
                }
                PlayEvent::Clear => {
                    self.audios.clear();
                    self.sink.take();
                }
                PlayEvent::Loop(v) => {
                    self.is_loop = v;
                }
                PlayEvent::Once(ref v) => {
                    if let Some(audio_source) = audio_sources.get(v.id) {
                        info!("failed to load asset, will reload");
                        self.play_once(audio_source);
                    } else {
                        // audio source hasn't loaded yet. add it back to the queue
                        pending.push(PlayEvent::Once(v.clone()));
                    }
                }
                PlayEvent::Append(ref v) => {
                    if let Some(audio_source) = audio_sources.get(v.id) {
                        info!("failed to load asset, will reload");
                        self.append_source(audio_source);
                        self.audios.push(v.clone());
                    } else {
                        // audio source hasn't loaded yet. add it back to the queue
                        pending.push(PlayEvent::Append(v.clone()));
                    }
                }
            }
        }
        events.update();
        let len_pend = pending.len();
        pending.into_iter().for_each(|t| {
            events.send(t);
        });
        if len_pend == 0 {
            self.check_loop(audio_sources);
        }
    }

    fn is_loop_end(&self) -> bool {
        self.sink.as_ref().map(|v| v.empty()).unwrap_or(false)
    }

    fn check_loop<T: Asset + Decodable>(&mut self, audio_sources: &Assets<T>) {
        if self.is_loop && self.sink.is_some() && self.is_loop_end() {
            let sources = self
                .audios
                .iter()
                .filter_map(|audio| audio_sources.get(audio.clone()))
                .collect::<Vec<_>>();
            for source in sources {
                self.append_source(source);
            }
        }
    }
}

/// Receiving play events and check loop
pub fn play_assets_system<Source: Asset>(world: &mut World)
where
    Source: Decodable,
{
    let world = world.cell();
    let mut asset_player = world.get_non_send_mut::<AssetPlayer>().unwrap();
    let events = world
        .get_resource_mut::<Events<PlayEvent<AudioSource>>>()
        .unwrap();

    if let Some(audio_sources) = world.get_resource::<Assets<Source>>() {
        asset_player.try_handle_events(&*audio_sources, events);
    };
}

///  play events for audio asset player
pub enum PlayEvent<Source = AudioSource>
where
    Source: Asset + Decodable,
{
    /// clear play list, stop audio play
    Clear,
    /// set loop, stop when all audios played if loop value is false
    Loop(bool),
    /// pause audio play
    Pause,
    /// resume audio play
    Resume,
    /// just play once, no loop, no pause
    Once(Handle<Source>),
    /// add audio to playlist
    Append(Handle<Source>),
}

impl<Source> Debug for PlayEvent<Source>
where
    Source: Asset + Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &PlayEvent::Clear => f.debug_struct("Clear").finish(),
            &PlayEvent::Loop(v) => f.debug_struct("Loop").field("Loop", &v).finish(),
            &PlayEvent::Pause => f.debug_struct("Pause").finish(),
            PlayEvent::Append(v) => f.debug_struct("Append").field("handle", v).finish(),
            PlayEvent::Once(v) => f.debug_struct("Once").field("handle", v).finish(),
            &PlayEvent::Resume => f.debug_struct("Resume").finish(),
        }
    }
}
