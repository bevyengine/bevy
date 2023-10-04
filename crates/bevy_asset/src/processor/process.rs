use crate::{
    io::{AssetReaderError, AssetWriterError, Writer},
    meta::{AssetAction, AssetMeta, AssetMetaDyn, ProcessDependencyInfo, ProcessedInfo, Settings},
    processor::AssetProcessor,
    saver::{AssetSaver, SavedAsset},
    AssetLoadError, AssetLoader, AssetPath, DeserializeMetaError, ErasedLoadedAsset,
    MissingAssetLoaderForExtensionError, MissingAssetLoaderForTypeNameError,
};
use bevy_utils::BoxedFuture;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, path::PathBuf};
use thiserror::Error;

/// Asset "processor" logic that reads input asset bytes (stored on [`ProcessContext`]), processes the value in some way,
/// and then writes the final processed bytes with [`Writer`]. The resulting bytes must be loadable with the given [`Process::OutputLoader`].
///
/// This is a "low level", maximally flexible interface. Most use cases are better served by the [`LoadAndSave`] implementation
/// of [`Process`].
pub trait Process: Send + Sync + Sized + 'static {
    /// The configuration / settings used to process the asset. This will be stored in the [`AssetMeta`] and is user-configurable per-asset.
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The [`AssetLoader`] that will be used to load the final processed asset.
    type OutputLoader: AssetLoader;
    /// Processes the asset stored on `context` in some way using the settings stored on `meta`. The results are written to `writer`. The
    /// final written processed asset is loadable using [`Process::OutputLoader`]. This load will use the returned [`AssetLoader::Settings`].
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        meta: AssetMeta<(), Self>,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, ProcessError>>;
}

/// A flexible [`Process`] implementation that loads the source [`Asset`] using the `L` [`AssetLoader`], then
/// saves that `L` asset using the `S` [`AssetSaver`].
///
/// When creating custom processors, it is generally recommended to use the [`LoadAndSave`] [`Process`] implementation,
/// as it encourages you to write both an [`AssetLoader`] capable of loading assets without processing enabled _and_
/// an [`AssetSaver`] that allows you to efficiently process that asset type when that is desirable by users. However you can
/// also implement [`Process`] directly if [`LoadAndSave`] feels limiting or unnecessary.
///
/// This uses [`LoadAndSaveSettings`] to configure the processor.
///
/// [`Asset`]: crate::Asset
pub struct LoadAndSave<L: AssetLoader, S: AssetSaver<Asset = L::Asset>> {
    saver: S,
    marker: PhantomData<fn() -> L>,
}

impl<L: AssetLoader, S: AssetSaver<Asset = L::Asset>> From<S> for LoadAndSave<L, S> {
    fn from(value: S) -> Self {
        LoadAndSave {
            saver: value,
            marker: PhantomData,
        }
    }
}

/// Settings for the [`LoadAndSave`] [`Process::Settings`] implementation.
///
/// `LoaderSettings` corresponds to [`AssetLoader::Settings`] and `SaverSettings` corresponds to [`AssetSaver::Settings`].
#[derive(Serialize, Deserialize, Default)]
pub struct LoadAndSaveSettings<LoaderSettings, SaverSettings> {
    /// The [`AssetLoader::Settings`] for [`LoadAndSave`].
    pub loader_settings: LoaderSettings,
    /// The [`AssetSaver::Settings`] for [`LoadAndSave`].
    pub saver_settings: SaverSettings,
}

/// An error that is encountered during [`Process::process`].
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("The asset source file for '{0}' does not exist")]
    MissingAssetSource(PathBuf),
    #[error(transparent)]
    AssetSourceIoError(std::io::Error),
    #[error(transparent)]
    MissingAssetLoaderForExtension(#[from] MissingAssetLoaderForExtensionError),
    #[error(transparent)]
    MissingAssetLoaderForTypeName(#[from] MissingAssetLoaderForTypeNameError),
    #[error("The processor '{0}' does not exist")]
    MissingProcessor(String),
    #[error(transparent)]
    AssetWriterError(#[from] AssetWriterError),
    #[error("Failed to read asset metadata {0:?}")]
    ReadAssetMetaError(AssetReaderError),
    #[error(transparent)]
    DeserializeMetaError(#[from] DeserializeMetaError),
    #[error(transparent)]
    AssetLoadError(#[from] AssetLoadError),
    #[error("The wrong meta type was passed into a processor. This is probably an internal implementation error.")]
    WrongMetaType,
    #[error("Encountered an error while saving the asset: {0}")]
    AssetSaveError(anyhow::Error),
    #[error("Assets without extensions are not supported.")]
    ExtensionRequired,
}

impl<Loader: AssetLoader, Saver: AssetSaver<Asset = Loader::Asset>> Process
    for LoadAndSave<Loader, Saver>
{
    type Settings = LoadAndSaveSettings<Loader::Settings, Saver::Settings>;
    type OutputLoader = Saver::OutputLoader;

    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        meta: AssetMeta<(), Self>,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, ProcessError>> {
        Box::pin(async move {
            let AssetAction::Process { settings, .. } = meta.asset else {
                return Err(ProcessError::WrongMetaType);
            };
            let loader_meta = AssetMeta::<Loader, ()>::new(AssetAction::Load {
                loader: std::any::type_name::<Loader>().to_string(),
                settings: settings.loader_settings,
            });
            let loaded_asset = context.load_source_asset(loader_meta).await?;
            let saved_asset = SavedAsset::<Loader::Asset>::from_loaded(&loaded_asset).unwrap();
            let output_settings = self
                .saver
                .save(writer, saved_asset, &settings.saver_settings)
                .await
                .map_err(ProcessError::AssetSaveError)?;
            Ok(output_settings)
        })
    }
}

/// A type-erased variant of [`Process`] that enables interacting with processor implementations without knowing
/// their type.
pub trait ErasedProcessor: Send + Sync {
    /// Type-erased variant of [`Process::process`].
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        meta: Box<dyn AssetMetaDyn>,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<Box<dyn AssetMetaDyn>, ProcessError>>;
    /// Deserialized `meta` as type-erased [`AssetMeta`], operating under the assumption that it matches the meta
    /// for the underlying [`Process`] impl.
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    /// Returns the default type-erased [`AssetMeta`] for the underlying [`Process`] impl.
    fn default_meta(&self) -> Box<dyn AssetMetaDyn>;
}

impl<P: Process> ErasedProcessor for P {
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        meta: Box<dyn AssetMetaDyn>,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<Box<dyn AssetMetaDyn>, ProcessError>> {
        Box::pin(async move {
            let meta = meta
                .downcast::<AssetMeta<(), P>>()
                .map_err(|_e| ProcessError::WrongMetaType)?;
            let loader_settings = <P as Process>::process(self, context, *meta, writer).await?;
            let output_meta: Box<dyn AssetMetaDyn> =
                Box::new(AssetMeta::<P::OutputLoader, ()>::new(AssetAction::Load {
                    loader: std::any::type_name::<P::OutputLoader>().to_string(),
                    settings: loader_settings,
                }));
            Ok(output_meta)
        })
    }

    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError> {
        let meta: AssetMeta<(), P> = ron::de::from_bytes(meta)?;
        Ok(Box::new(meta))
    }

    fn default_meta(&self) -> Box<dyn AssetMetaDyn> {
        Box::new(AssetMeta::<(), P>::new(AssetAction::Process {
            processor: std::any::type_name::<P>().to_string(),
            settings: P::Settings::default(),
        }))
    }
}

/// Provides scoped data access to the [`AssetProcessor`].
/// This must only expose processor data that is represented in the asset's hash.
pub struct ProcessContext<'a> {
    /// The "new" processed info for the final processed asset. It is [`ProcessContext`]'s
    /// job to populate `process_dependencies` with any asset dependencies used to process
    /// this asset (ex: loading an asset value from the [`AssetServer`] of the [`AssetProcessor`])
    ///
    /// DO NOT CHANGE ANY VALUES HERE OTHER THAN APPENDING TO `process_dependencies`
    ///
    /// Do not expose this publicly as it would be too easily to invalidate state.
    ///
    /// [`AssetServer`]: crate::server::AssetServer
    pub(crate) new_processed_info: &'a mut ProcessedInfo,
    /// This exists to expose access to asset values (via the [`AssetServer`]).
    ///
    /// ANY ASSET VALUE THAT IS ACCESSED SHOULD BE ADDED TO `new_processed_info.process_dependencies`
    ///
    /// Do not expose this publicly as it would be too easily to invalidate state by forgetting to update
    /// `process_dependencies`.
    ///
    /// [`AssetServer`]: crate::server::AssetServer
    processor: &'a AssetProcessor,
    path: &'a AssetPath<'static>,
    asset_bytes: &'a [u8],
}

impl<'a> ProcessContext<'a> {
    pub(crate) fn new(
        processor: &'a AssetProcessor,
        path: &'a AssetPath<'static>,
        asset_bytes: &'a [u8],
        new_processed_info: &'a mut ProcessedInfo,
    ) -> Self {
        Self {
            processor,
            path,
            asset_bytes,
            new_processed_info,
        }
    }

    /// Load the source asset using the `L` [`AssetLoader`] and the passed in `meta` config.
    /// This will take the "load dependencies" (asset values used when loading with `L`]) and
    /// register them as "process dependencies" because they are asset values required to process the
    /// current asset.
    pub async fn load_source_asset<L: AssetLoader>(
        &mut self,
        meta: AssetMeta<L, ()>,
    ) -> Result<ErasedLoadedAsset, AssetLoadError> {
        let server = &self.processor.server;
        let loader_name = std::any::type_name::<L>();
        let loader = server.get_asset_loader_with_type_name(loader_name).await?;
        let loaded_asset = server
            .load_with_meta_loader_and_reader(
                self.path,
                Box::new(meta),
                &*loader,
                &mut self.asset_bytes,
                false,
                true,
            )
            .await?;
        for (path, full_hash) in &loaded_asset.loader_dependencies {
            self.new_processed_info
                .process_dependencies
                .push(ProcessDependencyInfo {
                    full_hash: *full_hash,
                    path: path.to_owned(),
                });
        }
        Ok(loaded_asset)
    }

    /// The source bytes of the asset being processed.
    #[inline]
    pub fn asset_bytes(&self) -> &[u8] {
        self.asset_bytes
    }
}
