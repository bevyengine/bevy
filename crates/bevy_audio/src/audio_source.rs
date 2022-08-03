use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_reflect::TypeUuid;
use bevy_utils::BoxedFuture;
use std::{io::Cursor, sync::Arc};

/// A source of audio data
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7a14806a-672b-443b-8d16-4f18afefa463"]
pub struct AudioSource {
    /// Raw data of the audio source
    pub bytes: Arc<[u8]>,
}

impl AsRef<[u8]> for AudioSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Loads files as [`AudioSource`] [`Assets`](bevy_asset::Assets)
///
/// This asset loader supports different audio formats based on the enable Bevy features.
/// The feature `bevy/vorbis` enables loading from `.ogg` files and is enabled by default.
/// Other file endings can be loaded from with additional features:
/// `.mp3` with `bevy/mp3`
/// `.flac` with `bevy/flac`
/// `.wav` with `bevy/wav`
#[derive(Default)]
pub struct AudioLoader;

impl AssetLoader for AudioLoader {
    fn load(&self, bytes: &[u8], load_context: &mut LoadContext) -> BoxedFuture<Result<()>> {
        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: bytes.into(),
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
            "oga",
            #[cfg(feature = "vorbis")]
            "ogg",
            #[cfg(feature = "vorbis")]
            "spx",
        ]
    }
}

/// A type implementing this trait can be decoded as a rodio source
pub trait Decodable: Send + Sync + 'static {
    /// The decoder that can decode the implementing type
    type Decoder: rodio::Source + Send + Sync + Iterator<Item = Self::DecoderItem>;
    /// A single value given by the decoder
    type DecoderItem: rodio::Sample + Send + Sync;

    /// Build and return a [`Self::Decoder`] for the implementing type
    fn decoder(&self) -> Self::Decoder;
}

impl Decodable for AudioSource {
    type Decoder = rodio::Decoder<Cursor<AudioSource>>;
    type DecoderItem = <rodio::Decoder<Cursor<AudioSource>> as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        rodio::Decoder::new(Cursor::new(self.clone())).unwrap()
    }
}
