use anyhow::Result;
use bevy_asset::AssetLoader;
use std::{path::Path, sync::Arc};

/// A source of audio data
#[derive(Clone)]
pub struct AudioSource {
    pub bytes: Arc<Vec<u8>>,
}

impl AsRef<[u8]> for AudioSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Loads mp3 files as [AudioSource] [Assets](bevy_asset::Assets)
#[derive(Default)]
pub struct Mp3Loader;

impl AssetLoader<AudioSource> for Mp3Loader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<AudioSource> {
        Ok(AudioSource {
            bytes: Arc::new(bytes),
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg"];
        EXTENSIONS
    }
}
