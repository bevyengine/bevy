use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_reflect::TypeUuid;
use bevy_utils::{BoxedFuture, Duration};
use rodio::Source;
use std::{io::Cursor, sync::Arc};

/// A source of audio data
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7a14806a-672b-443b-8d16-4f18afefa463"]
pub struct AudioSource {
    pub bytes: Arc<[u8]>,
    pub decodable_kind: DecodableKind,
}

impl AsRef<[u8]> for AudioSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Loads mp3 files as [AudioSource] [Assets](bevy_asset::Assets)
#[derive(Default)]
pub struct Mp3Loader;

impl AssetLoader for Mp3Loader {
    fn load(&self, bytes: &[u8], load_context: &mut LoadContext) -> BoxedFuture<Result<()>> {
        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: bytes.into(),
            decodable_kind: DecodableKind::Decoder,
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &[
            #[cfg(feature = "mp3")]
            "mp3",
            #[cfg(feature = "flac")]
            "flac",
            #[cfg(feature = "wav")]
            "wav",
            #[cfg(feature = "vorbis")]
            "ogg",
        ]
    }
}

#[derive(Debug, Clone)]
pub enum DecodableKind {
    Decoder,
    Repeat,
}

pub trait Decodable: Send + Sync + 'static {
    fn decoder(&self) -> AudioSourceDecoder;
}

impl Decodable for AudioSource {
    fn decoder(&self) -> AudioSourceDecoder {
        let decodable = rodio::Decoder::new(Cursor::new(self.clone())).unwrap();
        match self.decodable_kind {
            DecodableKind::Decoder => AudioSourceDecoder::Decoder(decodable),
            DecodableKind::Repeat => AudioSourceDecoder::Repeat(decodable.repeat_infinite()),
        }
    }
}

impl AudioSource {
    pub fn set_repeat_infinite(&mut self, repeat_infinite: bool) {
        match repeat_infinite {
            true => self.decodable_kind = DecodableKind::Repeat,
            false => self.decodable_kind = DecodableKind::Decoder,
        }
    }
}

pub enum AudioSourceDecoder {
    Decoder(rodio::Decoder<Cursor<AudioSource>>),
    Repeat(rodio::source::Repeat<rodio::Decoder<Cursor<AudioSource>>>),
}

impl rodio::Source for AudioSourceDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        match self {
            AudioSourceDecoder::Decoder(d) => d.current_frame_len(),
            AudioSourceDecoder::Repeat(r) => r.current_frame_len(),
        }
    }

    fn channels(&self) -> u16 {
        match self {
            AudioSourceDecoder::Decoder(d) => d.channels(),
            AudioSourceDecoder::Repeat(r) => r.channels(),
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            AudioSourceDecoder::Decoder(d) => d.sample_rate(),
            AudioSourceDecoder::Repeat(r) => r.sample_rate(),
        }
    }

    fn total_duration(&self) -> Option<Duration> {
        match self {
            AudioSourceDecoder::Decoder(d) => d.total_duration(),
            AudioSourceDecoder::Repeat(r) => r.total_duration(),
        }
    }
}

impl Iterator for AudioSourceDecoder {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            AudioSourceDecoder::Decoder(d) => d.next(),
            AudioSourceDecoder::Repeat(r) => r.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            AudioSourceDecoder::Decoder(d) => d.size_hint(),
            AudioSourceDecoder::Repeat(r) => r.size_hint(),
        }
    }
}
