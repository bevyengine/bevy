use anyhow::Result;
use bevy_asset::{Asset, AssetLoader, LoadContext, LoadedAsset};
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

/// A type implementing this trait can be converted to a [`rodio::Source`] type. It must be [`Send`] and [`Sync`], and usually implements [`Asset`] so needs to be [`TypeUuid`], in order to be registered. Types that implement this trait usually contain raw sound data that can be converted into an iterator of samples. This trait is implemented for [`AudioSource`].
///
///    # Examples
///    Basic implementation:
///    ```
///    use bevy_app::App;
///    use bevy_reflect::TypeUuid;
///    use bevy_asset::AssetPlugin;
///    use bevy_audio::{AddAudioSource, Decodable, AudioPlugin};
///
///
///    // This struct contains the raw data for the audio being played. This is where data read from an audio file would be stored, for example.
///    // `TypeUuid` is derived for it so that `Asset` can be implemented for it, which allows it to be registered in the App.
///    #[derive(TypeUuid)]
///    #[uuid = "c2090c23-78fd-44f1-8508-c89b1f3cec29"]
///    struct CustomAudio {}
///    // This decoder is responsible for playing the audio, and so stores data about the audio being played.
///    struct CustomDecoder {
///        number_frames: u64,
///        channels: u16,
///        sample_rate: u32,
///        iter: std::vec::IntoIter<f32>,
///        frames_left: usize,
///    }
///
///    // The decoder must implement iterator so that it can implement `Decodable`. In this implementation it simply returns the next frame and decrements the frame count.
///    impl Iterator for CustomDecoder {
///        type Item = f32;
///
///        fn next(&mut self) -> Option<Self::Item> {
///            if let Some(frame) = self.iter.next() {
///                self.frames_left -= 1;
///                Some(frame)
///            } else {
///                None
///            }
///        }
///    }
///    // `rodio::Source` is what allows the audio source to be played by bevy. This trait provides information on the audio.
///    impl rodio::Source for CustomDecoder {
///        fn current_frame_len(&self) -> Option<usize> {
///            Some(self.frames_left)
///        }
///
///        fn channels(&self) -> u16 {
///            self.channels
///        }
///
///        fn sample_rate(&self) -> u32 {
///            self.sample_rate
///        }
///
///        fn total_duration(&self) -> Option<bevy_utils::Duration> {
///            Some(bevy_utils::Duration::from_secs(
///                self.number_frames / (self.sample_rate as u64 * self.channels as u64),
///            ))
///        }
///    }
///
///    // Finally `Decodable` can be implemented for our `CustomAudio`.
///    impl Decodable for CustomAudio {
///        type Decoder = CustomDecoder;
///
///        type DecoderItem = <CustomDecoder as Iterator>::Item;
///
///        fn decoder(&self) -> Self::Decoder {
///            // in reality the data would be read from a file then stored in `CustomAudio`, but for simplicity it is created here.
///            let frames = vec![0., 1., 2.];
///            CustomDecoder {
///                number_frames: frames.len() as u64,
///                channels: 1,
///                sample_rate: 1000,
///                iter: frames.clone().into_iter(),
///                frames_left: frames.len(),
///            }
///        }
///    }
///    
///
///    let mut app = App::new();
///    // register the audio source so that it can be used
///    app
///        .add_plugin(AssetPlugin::default())
///        .add_plugin(AudioPlugin)
///        .add_audio_source::<CustomAudio>();
///    ```
pub trait Decodable: Send + Sync + 'static {
    /// The type of the audio samples. Usually a [`u16`], [`i16`] or [`f32`], as those implement [`rodio::Sample`], but other types can implement [`rodio::Sample`] as well.
    type DecoderItem: rodio::Sample + Send + Sync;

    /// The type of the iterator of the audio samples, which iterators over samples of type [`Self::DecoderItem`]. Must be a [`rodio::Source`] so that it can provide information on the audio it is iterating over.
    type Decoder: rodio::Source + Send + Iterator<Item = Self::DecoderItem>;

    /// Build and return an iterator [`Self::Decoder`] for the implementing type
    fn decoder(&self) -> Self::Decoder;
}

impl Decodable for AudioSource {
    type Decoder = rodio::Decoder<Cursor<AudioSource>>;
    type DecoderItem = <rodio::Decoder<Cursor<AudioSource>> as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        rodio::Decoder::new(Cursor::new(self.clone())).unwrap()
    }
}

/// A trait that allows adding a custom audio source to the object. This is implemented for [`App`][bevy_app::App] to allow registering custom [`Decodable`] types.
pub trait AddAudioSource {
    /// Registers an audio source. The type must implement [`Decodable`], so that it can be converted to [`rodio::Source`], and [`Asset`], so that it can be registered. To use this method on [`App`][bevy_app::App] the [`Audio`][super::AudioPlugin] [`Asset`][bevy_asset::AssetPlugin] plugins must be added to the app.    
    fn add_audio_source<T>(&mut self) -> &mut Self
    where
        T: Decodable + Asset;
}
