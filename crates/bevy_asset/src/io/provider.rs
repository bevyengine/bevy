use bevy_ecs::system::Resource;
use bevy_utils::HashMap;

use crate::{
    io::{AssetReader, AssetWriter},
    AssetPlugin,
};

#[derive(Default, Clone, Debug)]
pub enum AssetProvider {
    #[default]
    Default,
    Custom(String),
}

#[derive(Resource, Default)]
pub struct AssetProviders {
    readers: HashMap<String, Box<dyn FnMut() -> Box<dyn AssetReader> + Send + Sync>>,
    writers: HashMap<String, Box<dyn FnMut() -> Box<dyn AssetWriter> + Send + Sync>>,
    default_file_source: Option<String>,
    default_file_destination: Option<String>,
}

impl AssetProviders {
    pub fn insert_reader(
        &mut self,
        provider: &str,
        get_reader: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) {
        self.readers
            .insert(provider.to_string(), Box::new(get_reader));
    }
    pub fn with_reader(
        mut self,
        provider: &str,
        get_reader: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.insert_reader(provider, get_reader);
        self
    }

    pub fn insert_writer(
        &mut self,
        provider: &str,
        get_writer: impl FnMut() -> Box<dyn AssetWriter> + Send + Sync + 'static,
    ) {
        self.writers
            .insert(provider.to_string(), Box::new(get_writer));
    }
    pub fn with_writer(
        mut self,
        provider: &str,
        get_writer: impl FnMut() -> Box<dyn AssetWriter> + Send + Sync + 'static,
    ) -> Self {
        self.insert_writer(provider, get_writer);
        self
    }

    pub fn default_file_source(&self) -> &str {
        self.default_file_source
            .as_deref()
            .unwrap_or(AssetPlugin::DEFAULT_FILE_SOURCE)
    }

    pub fn with_default_file_source(mut self, path: String) -> Self {
        self.default_file_source = Some(path);
        self
    }

    pub fn with_default_file_destination(mut self, path: String) -> Self {
        self.default_file_destination = Some(path);
        self
    }

    pub fn default_file_destination(&self) -> &str {
        self.default_file_destination
            .as_deref()
            .unwrap_or(AssetPlugin::DEFAULT_FILE_DESTINATION)
    }

    pub fn get_source_reader(&mut self, provider: &AssetProvider) -> Box<dyn AssetReader> {
        match provider {
            AssetProvider::Default => {
                #[cfg(not(target_arch = "wasm32"))]
                let reader = super::file::FileAssetReader::new(self.default_file_source());
                #[cfg(target_arch = "wasm32")]
                let reader = super::wasm::HttpWasmAssetReader::new(self.default_file_source());
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
    pub fn get_destination_reader(&mut self, provider: &AssetProvider) -> Box<dyn AssetReader> {
        match provider {
            AssetProvider::Default => {
                #[cfg(not(target_arch = "wasm32"))]
                let reader = super::file::FileAssetReader::new(self.default_file_destination());
                #[cfg(target_arch = "wasm32")]
                let reader = super::wasm::HttpWasmAssetReader::new(self.default_file_destination());
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

    pub fn get_source_writer(&mut self, provider: &AssetProvider) -> Box<dyn AssetWriter> {
        match provider {
            AssetProvider::Default => {
                #[cfg(not(target_arch = "wasm32"))]
                return Box::new(super::file::FileAssetWriter::new(
                    self.default_file_source(),
                ));
                #[cfg(target_arch = "wasm32")]
                panic!("Writing assets isn't supported on WASM yet");
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
    pub fn get_destination_writer(&mut self, provider: &AssetProvider) -> Box<dyn AssetWriter> {
        match provider {
            AssetProvider::Default => {
                #[cfg(not(target_arch = "wasm32"))]
                return Box::new(super::file::FileAssetWriter::new(
                    self.default_file_destination(),
                ));
                #[cfg(target_arch = "wasm32")]
                panic!("Writing assets isn't supported on WASM yet");
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
