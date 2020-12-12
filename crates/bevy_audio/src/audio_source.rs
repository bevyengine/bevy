use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_reflect::TypeUuid;
use bevy_utils::BoxedFuture;
use std::{io::Cursor, sync::Arc};

/// A source of audio data
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7a14806a-672b-443b-8d16-4f18afefa463"]
pub struct AudioSource {
    pub bytes: Arc<[u8]>,
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
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &["mp3", "flac", "wav", "ogg"]
    }
}

pub trait Decodable: Send + Sync + 'static {
    type Decoder;

    fn decoder(&self) -> Self::Decoder;
}

impl Decodable for AudioSource {
    type Decoder = rodio::Decoder<Cursor<AudioSource>>;

    fn decoder(&self) -> Self::Decoder {
        rodio::Decoder::new(Cursor::new(self.clone())).unwrap()
    }
}
