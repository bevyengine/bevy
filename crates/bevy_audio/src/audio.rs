use crate::{AudioSource, Decodable};
use bevy_asset::{Asset, Handle};
use parking_lot::RwLock;
use std::{collections::VecDeque, fmt};

/// The external struct used to play audio
pub struct Audio<P = AudioSource>
where
    P: Asset + Decodable,
{
    pub queue: RwLock<VecDeque<Handle<P>>>,
}

impl<P: Asset> fmt::Debug for Audio<P>
where
    P: Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Audio").field("queue", &self.queue).finish()
    }
}

impl<P> Default for Audio<P>
where
    P: Asset + Decodable,
{
    fn default() -> Self {
        Self {
            queue: Default::default(),
        }
    }
}

impl<P> Audio<P>
where
    P: Asset + Decodable,
    <P as Decodable>::Decoder: rodio::Source + Send + Sync,
    <<P as Decodable>::Decoder as Iterator>::Item: rodio::Sample + Send + Sync,
{
    pub fn play(&self, audio_source: Handle<P>) {
        self.queue.write().push_front(audio_source);
    }
}
