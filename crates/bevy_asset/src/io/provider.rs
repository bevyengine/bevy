use bevy_ecs::system::Resource;
use bevy_utils::HashMap;

use crate::{
    io::{AssetReader, AssetWriter},
    AssetPlugin,
};

/// A reference to an "asset provider", which maps to an [`AssetReader`] and/or [`AssetWriter`].
#[derive(Default, Clone, Debug)]
pub enum AssetProvider {
    /// The default asset provider
    #[default]
    Default,
    /// A custom / named asset provider
    Custom(String),
}

/// A [`Resource`] that hold (repeatable) functions capable of producing new [`AssetReader`] and [`AssetWriter`] instances
/// for a given [`AssetProvider`].
#[derive(Resource, Default)]
pub struct AssetProviders {
    readers: HashMap<String, Box<dyn FnMut() -> Box<dyn AssetReader> + Send + Sync>>,
    writers: HashMap<String, Box<dyn FnMut() -> Box<dyn AssetWriter> + Send + Sync>>,
    default_file_source: Option<String>,
    default_file_destination: Option<String>,
}

impl AssetProviders {
    /// Inserts a new `get_reader` function with the given `provider` name. This function will be used to create new [`AssetReader`]s
    /// when they are requested for the given `provider`.
    pub fn insert_reader(
        &mut self,
        provider: &str,
        get_reader: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) {
        self.readers
            .insert(provider.to_string(), Box::new(get_reader));
    }
    /// Inserts a new `get_reader` function with the given `provider` name. This function will be used to create new [`AssetReader`]s
    /// when they are requested for the given `provider`.
    pub fn with_reader(
        mut self,
        provider: &str,
        get_reader: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.insert_reader(provider, get_reader);
        self
    }
    /// Inserts a new `get_writer` function with the given `provider` name. This function will be used to create new [`AssetWriter`]s
    /// when they are requested for the given `provider`.
    pub fn insert_writer(
        &mut self,
        provider: &str,
        get_writer: impl FnMut() -> Box<dyn AssetWriter> + Send + Sync + 'static,
    ) {
        self.writers
            .insert(provider.to_string(), Box::new(get_writer));
    }
    /// Inserts a new `get_writer` function with the given `provider` name. This function will be used to create new [`AssetWriter`]s
    /// when they are requested for the given `provider`.
    pub fn with_writer(
        mut self,
        provider: &str,
        get_writer: impl FnMut() -> Box<dyn AssetWriter> + Send + Sync + 'static,
    ) -> Self {
        self.insert_writer(provider, get_writer);
        self
    }
    /// Returns the default "asset source" path for the [`FileAssetReader`] and [`FileAssetWriter`].
    ///
    /// [`FileAssetReader`]: crate::io::file::FileAssetReader
    /// [`FileAssetWriter`]: crate::io::file::FileAssetWriter
    pub fn default_file_source(&self) -> &str {
        self.default_file_source
            .as_deref()
            .unwrap_or(AssetPlugin::DEFAULT_FILE_SOURCE)
    }

    /// Sets the default "asset source" path for the [`FileAssetReader`] and [`FileAssetWriter`].
    ///
    /// [`FileAssetReader`]: crate::io::file::FileAssetReader
    /// [`FileAssetWriter`]: crate::io::file::FileAssetWriter
    pub fn with_default_file_source(mut self, path: String) -> Self {
        self.default_file_source = Some(path);
        self
    }

    /// Sets the default "asset destination" path for the [`FileAssetReader`] and [`FileAssetWriter`].
    ///
    /// [`FileAssetReader`]: crate::io::file::FileAssetReader
    /// [`FileAssetWriter`]: crate::io::file::FileAssetWriter
    pub fn with_default_file_destination(mut self, path: String) -> Self {
        self.default_file_destination = Some(path);
        self
    }

    /// Returns the default "asset destination" path for the [`FileAssetReader`] and [`FileAssetWriter`].
    ///
    /// [`FileAssetReader`]: crate::io::file::FileAssetReader
    /// [`FileAssetWriter`]: crate::io::file::FileAssetWriter
    pub fn default_file_destination(&self) -> &str {
        self.default_file_destination
            .as_deref()
            .unwrap_or(AssetPlugin::DEFAULT_FILE_DESTINATION)
    }

    /// Returns a new "source" [`AssetReader`] for the given [`AssetProvider`].
    pub fn get_source_reader(&mut self, provider: &AssetProvider) -> Box<dyn AssetReader> {
        match provider {
            AssetProvider::Default => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                let reader = super::file::FileAssetReader::new(self.default_file_source());
                #[cfg(target_arch = "wasm32")]
                let reader = super::wasm::HttpWasmAssetReader::new(self.default_file_source());
                #[cfg(target_os = "android")]
                let reader = super::android::AndroidAssetReader;
                Box::new(reader)
            }
            AssetProvider::Custom(provider) => {
                let get_reader = self
                    .readers
                    .get_mut(provider)
                    .unwrap_or_else(|| panic!("Asset Provider {} does not exist", provider));
                (get_reader)()
            }
        }
    }
    /// Returns a new "destination" [`AssetReader`] for the given [`AssetProvider`].
    pub fn get_destination_reader(&mut self, provider: &AssetProvider) -> Box<dyn AssetReader> {
        match provider {
            AssetProvider::Default => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                let reader = super::file::FileAssetReader::new(self.default_file_destination());
                #[cfg(target_arch = "wasm32")]
                let reader = super::wasm::HttpWasmAssetReader::new(self.default_file_destination());
                #[cfg(target_os = "android")]
                let reader = super::android::AndroidAssetReader;
                Box::new(reader)
            }
            AssetProvider::Custom(provider) => {
                let get_reader = self
                    .readers
                    .get_mut(provider)
                    .unwrap_or_else(|| panic!("Asset Provider {} does not exist", provider));
                (get_reader)()
            }
        }
    }
    /// Returns a new "source" [`AssetWriter`] for the given [`AssetProvider`].
    pub fn get_source_writer(&mut self, provider: &AssetProvider) -> Box<dyn AssetWriter> {
        match provider {
            AssetProvider::Default => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                return Box::new(super::file::FileAssetWriter::new(
                    self.default_file_source(),
                ));
                #[cfg(any(target_arch = "wasm32", target_os = "android"))]
                panic!("Writing assets isn't supported on this platform yet");
            }
            AssetProvider::Custom(provider) => {
                let get_writer = self
                    .writers
                    .get_mut(provider)
                    .unwrap_or_else(|| panic!("Asset Provider {} does not exist", provider));
                (get_writer)()
            }
        }
    }
    /// Returns a new "destination" [`AssetWriter`] for the given [`AssetProvider`].
    pub fn get_destination_writer(&mut self, provider: &AssetProvider) -> Box<dyn AssetWriter> {
        match provider {
            AssetProvider::Default => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                return Box::new(super::file::FileAssetWriter::new(
                    self.default_file_destination(),
                ));
                #[cfg(any(target_arch = "wasm32", target_os = "android"))]
                panic!("Writing assets isn't supported on this platform yet");
            }
            AssetProvider::Custom(provider) => {
                let get_writer = self
                    .writers
                    .get_mut(provider)
                    .unwrap_or_else(|| panic!("Asset Provider {} does not exist", provider));
                (get_writer)()
            }
        }
    }
}
