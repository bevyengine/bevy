use anyhow::Result;
use bevy_asset::{Asset, AssetLoader, LoadContext, LoadedAsset};
use bevy_reflect::TypeUuid;
use bevy_utils::BoxedFuture;
use std::{io::Cursor, sync::Arc};

/// A source of audio data
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7a14806a-672b-443b-8d16-4f18afefa463"]
pub struct AudioSource {
    /// Raw data of the audio source.
    ///
    /// The data must be one of the file formats supported by Bevy (`wav`, `ogg`, `flac`, or `mp3`).
    /// It is decoded using [`rodio::decoder::Decoder`](https://docs.rs/rodio/latest/rodio/decoder/struct.Decoder.html).
    ///
    /// The decoder has conditionally compiled methods
    /// depending on the features enabled.
    /// If the format used is not enabled,
    /// then this will panic with an `UnrecognizedFormat` error.
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

/// A type implementing this trait can be converted to a [`rodio::Source`] type.
/// It must be [`Send`] and [`Sync`], and usually implements [`Asset`] so needs to be [`TypeUuid`],
/// in order to be registered.
/// Types that implement this trait usually contain raw sound data that can be converted into an iterator of samples.
/// This trait is implemented for [`AudioSource`].
/// Check the example `audio/decodable` for how to implement this trait on a custom type.
pub trait Decodable: Send + Sync + 'static {
    /// The type of the audio samples.
    /// Usually a [`u16`], [`i16`] or [`f32`], as those implement [`rodio::Sample`].
    /// Other types can implement the [`rodio::Sample`] trait as well.
    type DecoderItem: rodio::Sample + Send + Sync;

    /// The type of the iterator of the audio samples,
    /// which iterates over samples of type [`Self::DecoderItem`].
    /// Must be a [`rodio::Source`] so that it can provide information on the audio it is iterating over.
    type Decoder: rodio::Source + Send + Iterator<Item = Self::DecoderItem>;

    /// Build and return a [`Self::Decoder`] of the implementing type
    fn decoder(&self) -> Self::Decoder;
}

impl Decodable for AudioSource {
    type Decoder = rodio::Decoder<Cursor<AudioSource>>;
    type DecoderItem = <rodio::Decoder<Cursor<AudioSource>> as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        rodio::Decoder::new(Cursor::new(self.clone())).unwrap()
    }
}

/// A trait that allows adding a custom audio source to the object.
/// This is implemented for [`App`][bevy_app::App] to allow registering custom [`Decodable`] types.
pub trait AddAudioSource {
    /// Registers an audio source.
    /// The type must implement [`Decodable`],
    /// so that it can be converted to a [`rodio::Source`] type,
    /// and [`Asset`], so that it can be registered as an asset.
    /// To use this method on [`App`][bevy_app::App],
    /// the [audio][super::AudioPlugin] and [asset][bevy_asset::AssetPlugin] plugins must be added first.    
    fn add_audio_source<T>(&mut self) -> &mut Self
    where
        T: Decodable + Asset;
}
