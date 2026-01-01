use crate::{
    io::{
        AssetReaderError, AssetWriterError, ErasedAssetWriter, MissingAssetWriterError,
        MissingProcessedAssetReaderError, MissingProcessedAssetWriterError, Reader,
        ReaderRequiredFeatures, Writer,
    },
    meta::{AssetAction, AssetMeta, AssetMetaDyn, ProcessDependencyInfo, ProcessedInfo, Settings},
    processor::AssetProcessor,
    saver::{AssetSaver, SavedAsset},
    transformer::{AssetTransformer, IdentityAssetTransformer, TransformedAsset},
    AssetLoadError, AssetLoader, AssetPath, DeserializeMetaError, ErasedLoadedAsset,
    MissingAssetLoaderForExtensionError, MissingAssetLoaderForTypeNameError,
};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use bevy_platform::collections::HashSet;
use bevy_reflect::TypePath;
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};
use futures_lite::AsyncWriteExt;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
};
use thiserror::Error;

/// Asset "processor" logic that reads input asset bytes (stored on [`ProcessContext`]), processes
/// the value in some way, and then writes the processed assets with [`WriterContext`].
///
/// This is a "low level", maximally flexible interface. Most use cases are better served by the
/// [`LoadTransformAndSave`] implementation of [`Process`].
pub trait Process: TypePath + Send + Sync + Sized + 'static {
    /// The configuration / settings used to process the asset. This will be stored in the [`AssetMeta`] and is user-configurable per-asset.
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// Processes the asset stored on `context` in some way using the settings stored on `meta`. The
    /// results are written to `writer_context`.
    fn process(
        &self,
        context: &mut ProcessContext,
        settings: &Self::Settings,
        writer_context: WriterContext<'_>,
    ) -> impl ConditionalSendFuture<Output = Result<(), ProcessError>>;

    /// Gets the features of the reader required to process the asset.
    fn reader_required_features(_settings: &Self::Settings) -> ReaderRequiredFeatures {
        ReaderRequiredFeatures::default()
    }
}

/// A flexible [`Process`] implementation that loads the source [`Asset`] using the `L` [`AssetLoader`], then transforms
/// the `L` asset into an `S` [`AssetSaver`] asset using the `T` [`AssetTransformer`], and lastly saves the asset using the `S` [`AssetSaver`].
///
/// When creating custom processors, it is generally recommended to use the [`LoadTransformAndSave`] [`Process`] implementation,
/// as it encourages you to separate your code into an [`AssetLoader`] capable of loading assets without processing enabled,
/// an [`AssetTransformer`] capable of converting from an `L` asset to an `S` asset, and
/// an [`AssetSaver`] that allows you save any `S` asset. However you can
/// also implement [`Process`] directly if [`LoadTransformAndSave`] feels limiting or unnecessary.
///
/// If your [`Process`] does not need to transform the [`Asset`], you can use [`IdentityAssetTransformer`] as `T`.
/// This will directly return the input [`Asset`], allowing your [`Process`] to directly load and then save an [`Asset`].
/// However, this pattern should only be used for cases such as file format conversion.
/// Otherwise, consider refactoring your [`AssetLoader`] and [`AssetSaver`] to isolate the transformation step into an explicit [`AssetTransformer`].
///
/// This uses [`LoadTransformAndSaveSettings`] to configure the processor.
///
/// [`Asset`]: crate::Asset
#[derive(TypePath)]
pub struct LoadTransformAndSave<
    L: AssetLoader,
    T: AssetTransformer<AssetInput = L::Asset>,
    S: AssetSaver<Asset = T::AssetOutput>,
> {
    transformer: T,
    saver: S,
    marker: PhantomData<fn() -> L>,
}

impl<L: AssetLoader, S: AssetSaver<Asset = L::Asset>> From<S>
    for LoadTransformAndSave<L, IdentityAssetTransformer<L::Asset>, S>
{
    fn from(value: S) -> Self {
        LoadTransformAndSave {
            transformer: IdentityAssetTransformer::new(),
            saver: value,
            marker: PhantomData,
        }
    }
}

/// Settings for the [`LoadTransformAndSave`] [`Process::Settings`] implementation.
///
/// `LoaderSettings` corresponds to [`AssetLoader::Settings`], `TransformerSettings` corresponds to [`AssetTransformer::Settings`],
/// and `SaverSettings` corresponds to [`AssetSaver::Settings`].
#[derive(Serialize, Deserialize, Default)]
pub struct LoadTransformAndSaveSettings<LoaderSettings, TransformerSettings, SaverSettings> {
    /// The [`AssetLoader::Settings`] for [`LoadTransformAndSave`].
    pub loader_settings: LoaderSettings,
    /// The [`AssetTransformer::Settings`] for [`LoadTransformAndSave`].
    pub transformer_settings: TransformerSettings,
    /// The [`AssetSaver::Settings`] for [`LoadTransformAndSave`].
    pub saver_settings: SaverSettings,
}

impl<
        L: AssetLoader,
        T: AssetTransformer<AssetInput = L::Asset>,
        S: AssetSaver<Asset = T::AssetOutput>,
    > LoadTransformAndSave<L, T, S>
{
    pub fn new(transformer: T, saver: S) -> Self {
        LoadTransformAndSave {
            transformer,
            saver,
            marker: PhantomData,
        }
    }
}

/// An error that is encountered during [`Process::process`].
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error(transparent)]
    MissingAssetLoaderForExtension(#[from] MissingAssetLoaderForExtensionError),
    #[error(transparent)]
    MissingAssetLoaderForTypeName(#[from] MissingAssetLoaderForTypeNameError),
    #[error("The processor '{0}' does not exist")]
    #[from(ignore)]
    MissingProcessor(String),
    #[error("The processor '{processor_short_name}' is ambiguous between several processors: {ambiguous_processor_names:?}")]
    AmbiguousProcessor {
        processor_short_name: String,
        ambiguous_processor_names: Vec<&'static str>,
    },
    #[error("Encountered an AssetReader error for '{path}': {err}")]
    #[from(ignore)]
    AssetReaderError {
        path: AssetPath<'static>,
        err: AssetReaderError,
    },
    #[error("Encountered an AssetWriter error for '{path}': {err}")]
    #[from(ignore)]
    AssetWriterError {
        path: AssetPath<'static>,
        err: AssetWriterError,
    },
    #[error(transparent)]
    MissingAssetWriterError(#[from] MissingAssetWriterError),
    #[error(transparent)]
    MissingProcessedAssetReaderError(#[from] MissingProcessedAssetReaderError),
    #[error(transparent)]
    MissingProcessedAssetWriterError(#[from] MissingProcessedAssetWriterError),
    #[error("Failed to read asset metadata for {path}: {err}")]
    #[from(ignore)]
    ReadAssetMetaError {
        path: AssetPath<'static>,
        err: AssetReaderError,
    },
    #[error(transparent)]
    DeserializeMetaError(#[from] DeserializeMetaError),
    #[error(transparent)]
    AssetLoadError(#[from] AssetLoadError),
    #[error("The wrong meta type was passed into a processor. This is probably an internal implementation error.")]
    WrongMetaType,
    #[error("Encountered an error while saving the asset: {0}")]
    #[from(ignore)]
    AssetSaveError(Box<dyn core::error::Error + Send + Sync + 'static>),
    #[error("Encountered an error while transforming the asset: {0}")]
    #[from(ignore)]
    AssetTransformError(Box<dyn core::error::Error + Send + Sync + 'static>),
    #[error("Assets without extensions are not supported.")]
    ExtensionRequired,
    #[error(transparent)]
    InvalidProcessOutput(#[from] InvalidProcessOutput),
}

impl<Loader, Transformer, Saver> Process for LoadTransformAndSave<Loader, Transformer, Saver>
where
    Loader: AssetLoader,
    Transformer: AssetTransformer<AssetInput = Loader::Asset>,
    Saver: AssetSaver<Asset = Transformer::AssetOutput>,
{
    type Settings =
        LoadTransformAndSaveSettings<Loader::Settings, Transformer::Settings, Saver::Settings>;

    async fn process(
        &self,
        context: &mut ProcessContext<'_>,
        settings: &Self::Settings,
        writer_context: WriterContext<'_>,
    ) -> Result<(), ProcessError> {
        let pre_transformed_asset = TransformedAsset::<Loader::Asset>::from_loaded(
            context
                .load_source_asset::<Loader>(&settings.loader_settings)
                .await?,
        )
        .unwrap();

        let post_transformed_asset = self
            .transformer
            .transform(pre_transformed_asset, &settings.transformer_settings)
            .await
            .map_err(|err| ProcessError::AssetTransformError(err.into()))?;

        let saved_asset =
            SavedAsset::<Transformer::AssetOutput>::from_transformed(&post_transformed_asset);

        let saver = &self.saver;
        let saver_settings = &settings.saver_settings;
        let mut writer = writer_context.write_full().await?;

        let output_settings = saver
            .save(&mut *writer, saved_asset, saver_settings)
            .await
            .map_err(|error| ProcessError::AssetSaveError(error.into()))?;

        writer.finish::<Saver::OutputLoader>(output_settings).await
    }

    fn reader_required_features(settings: &Self::Settings) -> ReaderRequiredFeatures {
        Loader::reader_required_features(&settings.loader_settings)
    }
}

/// A type-erased variant of [`Process`] that enables interacting with processor implementations without knowing
/// their type.
pub trait ErasedProcessor: Send + Sync {
    /// Type-erased variant of [`Process::process`].
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        settings: &'a dyn Settings,
        writer_context: WriterContext<'a>,
    ) -> BoxedFuture<'a, Result<(), ProcessError>>;
    /// Type-erased variant of [`Process::reader_required_features`].
    // Note: This takes &self just to be dyn compatible.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::result_large_err,
            reason = "this is only an error here because this isn't a future"
        )
    )]
    fn reader_required_features(
        &self,
        settings: &dyn Settings,
    ) -> Result<ReaderRequiredFeatures, ProcessError>;
    /// Deserialized `meta` as type-erased [`AssetMeta`], operating under the assumption that it matches the meta
    /// for the underlying [`Process`] impl.
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    /// Returns the type-path of the original [`Process`].
    fn type_path(&self) -> &'static str;
    /// Returns the default type-erased [`AssetMeta`] for the underlying [`Process`] impl.
    fn default_meta(&self) -> Box<dyn AssetMetaDyn>;
}

impl<P: Process> ErasedProcessor for P {
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        settings: &'a dyn Settings,
        writer_context: WriterContext<'a>,
    ) -> BoxedFuture<'a, Result<(), ProcessError>> {
        Box::pin(async move {
            let settings = settings.downcast_ref().ok_or(ProcessError::WrongMetaType)?;
            <P as Process>::process(self, context, settings, writer_context).await
        })
    }

    fn reader_required_features(
        &self,
        settings: &dyn Settings,
    ) -> Result<ReaderRequiredFeatures, ProcessError> {
        let settings = settings.downcast_ref().ok_or(ProcessError::WrongMetaType)?;
        Ok(P::reader_required_features(settings))
    }

    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError> {
        let meta: AssetMeta<(), P> = ron::de::from_bytes(meta)?;
        Ok(Box::new(meta))
    }

    fn type_path(&self) -> &'static str {
        P::type_path()
    }

    fn default_meta(&self) -> Box<dyn AssetMetaDyn> {
        Box::new(AssetMeta::<(), P>::new(AssetAction::Process {
            processor: P::type_path().to_string(),
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
    reader: Box<dyn Reader + 'a>,
}

impl<'a> ProcessContext<'a> {
    pub(crate) fn new(
        processor: &'a AssetProcessor,
        path: &'a AssetPath<'static>,
        reader: Box<dyn Reader + 'a>,
        new_processed_info: &'a mut ProcessedInfo,
    ) -> Self {
        Self {
            processor,
            path,
            reader,
            new_processed_info,
        }
    }

    /// Load the source asset using the `L` [`AssetLoader`] and the passed in `meta` config.
    /// This will take the "load dependencies" (asset values used when loading with `L`]) and
    /// register them as "process dependencies" because they are asset values required to process the
    /// current asset.
    pub async fn load_source_asset<L: AssetLoader>(
        &mut self,
        settings: &L::Settings,
    ) -> Result<ErasedLoadedAsset, AssetLoadError> {
        let server = &self.processor.server;
        let loader_name = L::type_path();
        let loader = server.get_asset_loader_with_type_name(loader_name).await?;
        let loaded_asset = server
            .load_with_settings_loader_and_reader(
                self.path,
                settings,
                &*loader,
                &mut self.reader,
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

    /// The path of the asset being processed.
    #[inline]
    pub fn path(&self) -> &AssetPath<'static> {
        self.path
    }

    /// The reader for the asset being processed.
    #[inline]
    pub fn asset_reader(&mut self) -> &mut dyn Reader {
        &mut self.reader
    }
}

/// The context for any writers that a [`Process`] may use.
pub struct WriterContext<'a> {
    /// The underlying writer of all writes for the [`Process`].
    writer: &'a dyn ErasedAssetWriter,
    /// The context for initializing a write.
    // We use a Mutex to avoid requiring a mutable borrow for `write_partial`. See `write_partial`
    // for more details.
    init_context: Mutex<WriteInitContext<'a>>,
    /// The number of writes that have been fully finished.
    ///
    /// Note we use an `AtomicU32` instead of a u32 so that writes (and therefore finish's) don't
    /// need to be synchronous. We use a mutable borrow so that full-writes can just update the
    /// value without atomics.
    finished_writes: &'a mut AtomicU32,
    /// The meta object to write when writing a single file. Must be set to [`Some`] when writing a
    /// "full" file.
    full_meta: &'a mut Option<Box<dyn AssetMetaDyn>>,
    /// The path of the asset being processed.
    path: &'a AssetPath<'static>,
}

/// The context for the initialization when writing a processed file.
struct WriteInitContext<'a> {
    /// The number of writes that have been started.
    started_writes: &'a mut u32,
    /// The set of currently started [`WriterContext::write_partial`] instances.
    ///
    /// This protects us from starting writes for the same path multiple times.
    started_paths: HashSet<PathBuf>,
}

impl<'a> WriterContext<'a> {
    pub(crate) fn new(
        writer: &'a dyn ErasedAssetWriter,
        started_writes: &'a mut u32,
        finished_writes: &'a mut AtomicU32,
        full_meta: &'a mut Option<Box<dyn AssetMetaDyn>>,
        path: &'a AssetPath<'static>,
    ) -> Self {
        Self {
            writer,
            init_context: Mutex::new(WriteInitContext {
                started_writes,
                started_paths: HashSet::new(),
            }),
            finished_writes,
            full_meta,
            path,
        }
    }

    /// Start writing a single output file, which can be loaded with the `load_settings`.
    ///
    /// Returns an error if you have previously called [`Self::write_partial`].
    pub async fn write_full(self) -> Result<FullWriter<'a>, ProcessError> {
        let started_writes = self
            .init_context
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .started_writes;
        if *started_writes != 0 {
            return Err(ProcessError::InvalidProcessOutput(
                InvalidProcessOutput::FullFileAfterPartialFile,
            ));
        }
        *started_writes = 1;

        let writer = self.writer.write(self.path.path()).await.map_err(|err| {
            ProcessError::AssetWriterError {
                path: self.path.clone_owned(),
                err,
            }
        })?;
        Ok(FullWriter {
            writer,
            finished_writes: self.finished_writes.get_mut(),
            path: self.path,
            meta: self.full_meta,
        })
    }

    /// Start writing one of multiple output files, which can be loaded with the `load_settings`.
    // Note: It would be nice to take this by a mutable reference instead. However, doing so would
    // mean that the returned value would be tied to a "mutable reference lifetime", meaning we
    // could not use more than one `PartialWriter` instance concurrently.
    pub async fn write_partial(&self, file: &Path) -> Result<PartialWriter<'_>, ProcessError> {
        // Do all the validation in a scope so we don't hold the init_context for too long.
        {
            let mut init_context = self
                .init_context
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            // Check whether this path is valid first so that we don't mark the write as started
            // when it hasn't.
            if !init_context.started_paths.insert(file.to_path_buf()) {
                return Err(InvalidProcessOutput::RepeatedPartialWriteToSamePath(
                    file.to_path_buf(),
                )
                .into());
            }
            *init_context.started_writes += 1;
        }

        let path = self.path.path().join(file);
        let path = AssetPath::from_path_buf(path).with_source(self.path.source().clone_owned());

        let writer = self
            .writer
            .write(path.path())
            .await
            // Note: It's possible that a user receives the error and then tries to recover, but
            // this would leave the process in an invalid state (since you would never be able to
            // call `finish` enough times). We could decrement the `started_writes` counter, but
            // it's unclear what a reasonable recovery a user could do in this case - just
            // propagating the error is safer and makes more sense.
            .map_err(|err| ProcessError::AssetWriterError {
                path: path.clone_owned(),
                err,
            })?;
        Ok(PartialWriter {
            meta_writer: self.writer,
            writer,
            finished_writes: &*self.finished_writes,
            path,
        })
    }
}

/// An error regarding the output state of a [`Process`].
#[derive(Error, Debug)]
pub enum InvalidProcessOutput {
    /// The processor didn't start a write at all.
    #[error(
        "The processor never started writing a file (never called `write_full` or `write_partial`)"
    )]
    NoWriter,
    /// The processor started a write but never finished it.
    #[error("The processor started writing a file, but never called `finish`")]
    UnfinishedWriter,
    /// The processor started at least one partial write, then continued with a full write.
    #[error("The processor called `write_full` after already calling `write_partial`")]
    FullFileAfterPartialFile,
    /// The processor started a partial write with the same path multiple times.
    #[error("The processor called `write_partial` more than once with the same path")]
    RepeatedPartialWriteToSamePath(PathBuf),
}

/// The writer for a [`Process`] writing a single file (at the same path as the unprocessed asset).
pub struct FullWriter<'a> {
    /// The writer to write to.
    writer: Box<Writer>,
    /// The counter for finished writes that will be incremented when the write completes.
    finished_writes: &'a mut u32,
    /// The meta object that will be assigned on [`Self::finish`].
    meta: &'a mut Option<Box<dyn AssetMetaDyn>>,
    /// The path of the asset being written.
    path: &'a AssetPath<'static>,
}

impl FullWriter<'_> {
    /// Finishes a write and indicates that the written asset should be loaded with the provided
    /// loader and the provided settings for that loader.
    ///
    /// This must be called before the [`Process`] ends.
    pub async fn finish<L: AssetLoader>(
        mut self,
        load_settings: L::Settings,
    ) -> Result<(), ProcessError> {
        self.writer
            .flush()
            .await
            .map_err(|err| ProcessError::AssetWriterError {
                path: self.path.clone_owned(),
                err: AssetWriterError::Io(err),
            })?;

        let output_meta = AssetMeta::<L, ()>::new(AssetAction::Load {
            loader: L::type_path().to_string(),
            settings: load_settings,
        });

        // This should always be none, since we consumed the WriterContext, and we consume the
        // only borrow here.
        assert!(self.meta.is_none());
        *self.meta = Some(Box::new(output_meta));

        // Make sure to increment finished writes at the very end, so that we only count it, once
        // the future is finished anyway.
        *self.finished_writes += 1;
        Ok(())
    }
}

/// A writer for a [`Process`] writing multiple partial files (as children of the unprocessed asset
/// path).
pub struct PartialWriter<'a> {
    /// The writer to use when writing the meta file for this file.
    meta_writer: &'a dyn ErasedAssetWriter,
    /// The writer to write to.
    writer: Box<Writer>,
    /// The counter for finished writes that will be incremented when the write completes.
    finished_writes: &'a AtomicU32,
    /// The path of the file being written.
    ///
    /// This includes the path relative to the unprocessed asset.
    path: AssetPath<'static>,
}

impl PartialWriter<'_> {
    /// Finishes a write and indicates that the written asset should be loaded with the provided
    /// loader and the provided settings for that loader.
    ///
    /// This must be called before the [`Process`] ends.
    pub async fn finish<L: AssetLoader>(
        mut self,
        load_settings: L::Settings,
    ) -> Result<(), ProcessError> {
        self.writer
            .flush()
            .await
            .map_err(|err| ProcessError::AssetWriterError {
                path: self.path.clone_owned(),
                err: AssetWriterError::Io(err),
            })?;

        let output_meta = AssetMeta::<L, ()>::new(AssetAction::Load {
            loader: L::type_path().to_string(),
            settings: load_settings,
        });

        let output_meta_bytes = AssetMetaDyn::serialize(&output_meta);

        let result = self
            .meta_writer
            .write_meta_bytes(self.path.path(), &output_meta_bytes)
            .await
            .map_err(|err| ProcessError::AssetWriterError {
                path: self.path.clone_owned(),
                err,
            });

        if result.is_ok() {
            // The ordering here doesn't really matter, since this is just a cheaper Mutex<u32>.
            // Just in case, we'll be overly safe and use SeqCst.
            self.finished_writes.fetch_add(1, Ordering::SeqCst);
        }

        result
    }
}

impl Deref for FullWriter<'_> {
    type Target = Writer;

    fn deref(&self) -> &Self::Target {
        self.writer.as_ref()
    }
}

impl DerefMut for FullWriter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.writer.as_mut()
    }
}

impl Deref for PartialWriter<'_> {
    type Target = Writer;

    fn deref(&self) -> &Self::Target {
        self.writer.as_ref()
    }
}

impl DerefMut for PartialWriter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.writer.as_mut()
    }
}
