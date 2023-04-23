use crate::{
    io::{AssetReaderError, AssetWriterError, Writer},
    meta::{
        AssetAction, AssetMeta, AssetMetaDyn, ProcessDependencyInfo, ProcessedInfo, Settings,
        META_FORMAT_VERSION,
    },
    processor::AssetProcessor,
    saver::AssetSaver,
    AssetLoadError, AssetLoader, AssetPath, DeserializeMetaError, ErasedLoadedAsset,
    MissingAssetLoaderForExtensionError, MissingAssetLoaderForTypeNameError,
};
use bevy_utils::BoxedFuture;
use futures_lite::FutureExt;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

pub trait Process: Send + Sync + Sized + 'static {
    type Asset: crate::Asset;
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    type OutputLoader: AssetLoader;
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        meta: AssetMeta<(), Self>,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, ProcessError>>;
}

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

#[derive(Serialize, Deserialize, Default)]
pub struct LoadAndSaveSettings<LoaderSettings, SaverSettings> {
    pub loader_settings: LoaderSettings,
    pub saver_settings: SaverSettings,
}

#[derive(Error, Debug)]
pub enum ProcessError {
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
}

impl<Loader: AssetLoader, Saver: AssetSaver<Asset = Loader::Asset>> Process
    for LoadAndSave<Loader, Saver>
{
    type Asset = Loader::Asset;
    type Settings = LoadAndSaveSettings<Loader::Settings, Saver::Settings>;
    type OutputLoader = Saver::OutputLoader;

    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        meta: AssetMeta<(), Self>,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, ProcessError>> {
        async move {
            let AssetAction::Process { settings, .. } = meta.asset else {
                return Err(ProcessError::WrongMetaType);
            };
            let loader_meta = AssetMeta::<Loader, ()> {
                meta_format_version: META_FORMAT_VERSION.to_string(),
                asset: AssetAction::Load {
                    loader: std::any::type_name::<Loader>().to_string(),
                    settings: settings.loader_settings,
                },
                processed_info: None,
            };
            let loaded_asset = context.load_source_asset(loader_meta).await?;
            let output_settings = self
                .saver
                .save(
                    writer,
                    loaded_asset.get::<Loader::Asset>().unwrap(),
                    &settings.saver_settings,
                )
                .await
                .map_err(ProcessError::AssetSaveError)?;
            Ok(output_settings)
        }
        .boxed()
    }
}

pub trait ErasedProcessor: Send + Sync {
    fn process<'a>(
        &'a self,
        context: ProcessContext<'a>,
        meta: Box<dyn AssetMetaDyn>,
        writer: &'a mut Writer,
        asset_hash: u64,
    ) -> BoxedFuture<'a, Result<Box<dyn AssetMetaDyn>, ProcessError>>;
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    fn default_meta(&self) -> Box<dyn AssetMetaDyn>;
}

impl<P: Process> ErasedProcessor for P {
    fn process<'a>(
        &'a self,
        mut context: ProcessContext<'a>,
        meta: Box<dyn AssetMetaDyn>,
        writer: &'a mut Writer,
        asset_hash: u64,
    ) -> BoxedFuture<'a, Result<Box<dyn AssetMetaDyn>, ProcessError>> {
        async move {
            let meta = meta
                .downcast::<AssetMeta<(), P>>()
                .map_err(|_e| ProcessError::WrongMetaType)?;
            let loader_settings =
                <P as Process>::process(self, &mut context, *meta, writer).await?;
            let full_hash = AssetProcessor::get_full_hash(
                asset_hash,
                context
                    .new_processed_info
                    .process_dependencies
                    .iter()
                    .map(|i| i.full_hash),
            );
            context.new_processed_info.full_hash = full_hash;
            let output_meta: Box<dyn AssetMetaDyn> = Box::new(AssetMeta::<P::OutputLoader, ()> {
                meta_format_version: META_FORMAT_VERSION.to_string(),
                asset: AssetAction::Load {
                    loader: std::any::type_name::<P::OutputLoader>().to_string(),
                    settings: loader_settings,
                },
                processed_info: Some(context.new_processed_info.clone()),
            });
            Ok(output_meta)
        }
        .boxed()
    }

    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError> {
        let meta: AssetMeta<(), P> = ron::de::from_bytes(meta)?;
        Ok(Box::new(meta))
    }

    fn default_meta(&self) -> Box<dyn AssetMetaDyn> {
        Box::new(AssetMeta::<(), P> {
            meta_format_version: META_FORMAT_VERSION.to_string(),
            processed_info: None,
            asset: AssetAction::Process {
                processor: std::any::type_name::<P>().to_string(),
                settings: P::Settings::default(),
            },
        })
    }
}

/// Provides scoped data access to the [`AssetProcessor`].
/// This must only expose processor data that is represented in the asset's hash.
pub struct ProcessContext<'a> {
    processor: &'a AssetProcessor,
    path: &'a AssetPath<'static>,
    asset_bytes: &'a [u8],
    new_processed_info: &'a mut ProcessedInfo,
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

    /// Load the source asset
    pub async fn load_source_asset<L: AssetLoader>(
        &mut self,
        meta: AssetMeta<L, ()>,
    ) -> Result<ErasedLoadedAsset, AssetLoadError> {
        let server = &self.processor.server;
        let loader_name = std::any::type_name::<L>();
        let loader = server.get_asset_loader_with_type_name(loader_name)?;
        let loaded_asset = server
            .load_with_meta_loader_and_reader(
                &self.path,
                Box::new(meta),
                &*loader,
                &mut self.asset_bytes,
                false,
            )
            .await?;
        for (path, full_hash) in loaded_asset.loader_dependencies.iter() {
            self.new_processed_info
                .process_dependencies
                .push(ProcessDependencyInfo {
                    full_hash: *full_hash,

                    path: path.to_owned(),
                })
        }
        Ok(loaded_asset)
    }
}
