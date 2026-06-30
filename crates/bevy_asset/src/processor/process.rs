use crate::{
    io::{
        AssetReaderError, AssetWriterError, MissingAssetWriterError,
        MissingProcessedAssetReaderError, MissingProcessedAssetWriterError, Reader, Writer,
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
use bevy_ecs::error::BevyError;
use bevy_reflect::TypePath;
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::marker::PhantomData;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Asset "processor" logic that reads input asset bytes (stored on [`ProcessContext`]), processes the value in some way,
/// and then writes the final processed bytes with [`Writer`]. The resulting bytes must be loadable with the given [`Process::OutputLoader`].
///
/// This is a "low level", maximally flexible interface. Most use cases are better served by the
/// [`make_load_transform_and_save_processor`] macro that generates an implementation of
/// [`Process`].
///
/// [`make_load_transform_and_save_processor`]: crate::make_load_transform_and_save_processor
pub trait Process: TypePath + Send + Sync + Sized + 'static {
    /// The configuration / settings used to process the asset. This will be stored in the [`AssetMeta`] and is user-configurable per-asset.
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The [`AssetLoader`] that will be used to load the final processed asset.
    type OutputLoader: AssetLoader;
    /// Processes the asset stored on `context` in some way using the settings stored on `meta`. The results are written to `writer`. The
    /// final written processed asset is loadable using [`Process::OutputLoader`]. This load will use the returned [`AssetLoader::Settings`].
    fn process(
        &self,
        context: &mut ProcessContext,
        settings: &Self::Settings,
        writer: &mut Writer,
    ) -> impl ConditionalSendFuture<
        Output = Result<<Self::OutputLoader as AssetLoader>::Settings, ProcessError>,
    >;
}

/// Creates a [`Process`] implementation (aka an asset processor) that loads an asset with the given
/// [`AssetLoader`], transforms the asset using the given [`AssetTransformer`],  and then saves the
/// final asset with the given [`AssetSaver`].
///
/// This macro requires creating two structs: the processor and the processor's settings. It also
/// requires that the processor struct includes 2 or 3 fields:
///
/// 1. `loader` whose type must implement [`AssetLoader`].
/// 2. (optional) `transformer` whose type must implement [`AssetTransformer`]. If omitted, the
///    loaded asset is passed to the saver without being transformed.
/// 3. `saver` whose type must implement [`AssetSaver`].
///
/// This macro **does not** support generics since each concrete instance needs to be registered
/// anyway.
///
/// Here are examples of defining such processors:
///
/// ```rust
/// # use bevy_asset::{*, saver::*, transformer::*, processor::*, io::*};
/// # use bevy_reflect::TypePath;
/// # #[derive(TypePath)]
/// # pub struct FakeSaver;
/// # impl AssetSaver for FakeSaver {
/// #     type Asset = ();
/// #     type Settings = ();
/// #     type Error = ProcessError;
/// #     type OutputLoader = ();
/// #
/// #     async fn save(
/// #         &self,
/// #         _writer: &mut Writer,
/// #         _asset: SavedAsset<'_, '_, Self::Asset>,
/// #         _settings: &Self::Settings,
/// #         _asset_path: AssetPath<'_>,
/// #     ) -> Result<(), Self::Error> {
/// #         todo!()
/// #     }
/// # }
/// # type ImageLoader = ();
/// # type CompressedImageSaver = FakeSaver;
/// // This processor only needs to load and save.
/// make_load_transform_and_save_processor!{
///     /// This is a doc comment!
///     pub struct ImageProcessor {
///         loader: ImageLoader,
///         saver: CompressedImageSaver,
///     }
///
///     // This is another doc comment!
///     pub struct ImageProcessorSettings { .. }
/// }
///
/// # type CoolTextLoader = ();
/// # type ReplaceBadWordsWithStars = IdentityAssetTransformer<()>;
/// # type CoolTextSaver = FakeSaver;
/// make_load_transform_and_save_processor!{
///     pub struct CoolTextProcessor {
///         loader: CoolTextLoader,
///         transformer: ReplaceBadWordsWithStars,
///         saver: CoolTextSaver,
///     }
///
///     pub struct CoolTextProcessorSettings { .. }
/// }
/// ```
#[macro_export]
macro_rules! make_load_transform_and_save_processor {
    ($(#[$meta:meta])* $v:vis struct $ty:ident {
        loader: $loader:ty,
        $(transformer: $transformer:ty,)?
        saver: $saver:ty $(,)?
    }

    $(#[$meta2:meta])*
    $v2:vis struct $settings_ty:ident { .. }) => {
        $(#[$meta])*
        #[derive(bevy_reflect::TypePath)]
        $v struct $ty {
            $(transformer: $transformer,)?
            saver: $saver,
        }

        impl $ty {
            /// Creates a new instance of this processor, with the given `saver`.
            pub fn new($(transformer: $transformer,)? saver: $saver) -> Self {
                Self {
                    $(transformer: {
                        let t: $transformer = transformer;
                        t
                    },)?
                    saver,
                }
            }
        }

        $(#[$meta])*
        #[derive(serde::Serialize, serde::Deserialize, Default)]
        $v2 struct $settings_ty {
            loader_settings: <$loader as $crate::AssetLoader>::Settings,
            $(transformer_settings: <$transformer as $crate::transformer::AssetTransformer>::Settings,)?
            saver_settings: <$saver as $crate::saver::AssetSaver>::Settings,
        }

        impl $crate::processor::Process for $ty {
            type Settings = $settings_ty;
            type OutputLoader = <$saver as $crate::saver::AssetSaver>::OutputLoader;

            async fn process(
                &self,
                context: &mut $crate::processor::ProcessContext<'_>,
                settings: &$settings_ty,
                writer: &mut $crate::io::Writer,
            ) -> Result<<<Self as $crate::processor::Process>::OutputLoader as $crate::AssetLoader>::Settings, $crate::processor::ProcessError> {
                let transformer = &$crate::transformer::IdentityAssetTransformer::<
                    <$loader as $crate::AssetLoader>::Asset
                >::new();
                let transformer_settings = &();
                $(
                    let _ = (transformer, transformer_settings);
                    let transformer: &$transformer = &self.transformer;
                    let transformer_settings = &settings.transformer_settings;
                )?
                $crate::processor::load_transform_and_save::<$loader, _, _>(
                    (transformer, &self.saver),
                    (&settings.loader_settings, transformer_settings, &settings.saver_settings),
                    context,
                    writer,
                ).await
            }
        }
    };
}

/// A flexible [`Process`] implementation that loads the source [`Asset`] using the `L` [`AssetLoader`], then transforms
/// the `L` asset into an `S` [`AssetSaver`] asset using the `T` [`AssetTransformer`], and lastly saves the asset using the `S` [`AssetSaver`].
///
/// [`Asset`]: crate::Asset
#[derive(TypePath)]
#[deprecated = "Use `make_load_transform_and_save_processor` instead."]
pub struct LoadTransformAndSave<
    L: AssetLoader,
    T: AssetTransformer<AssetInput = L::Asset>,
    S: AssetSaver<Asset = T::AssetOutput>,
> {
    transformer: T,
    saver: S,
    marker: PhantomData<fn() -> L>,
}

#[expect(
    deprecated,
    reason = "We need to maintain the trait impls until we delete `LoadTransformAndSave`"
)]
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
#[deprecated = "Use `make_load_transform_and_save_processor` instead."]
pub struct LoadTransformAndSaveSettings<LoaderSettings, TransformerSettings, SaverSettings> {
    /// The [`AssetLoader::Settings`] for [`LoadTransformAndSave`].
    pub loader_settings: LoaderSettings,
    /// The [`AssetTransformer::Settings`] for [`LoadTransformAndSave`].
    pub transformer_settings: TransformerSettings,
    /// The [`AssetSaver::Settings`] for [`LoadTransformAndSave`].
    pub saver_settings: SaverSettings,
}

#[expect(
    deprecated,
    reason = "We need to maintain the trait impls until we delete `LoadTransformAndSave`"
)]
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
    /// No processor with that name is registered..
    #[error("The processor '{0}' does not exist")]
    #[from(ignore)]
    MissingProcessor(String),
    /// The given short name is ambiguous between several processors.
    #[error("The processor '{processor_short_name}' is ambiguous between several processors: {ambiguous_processor_names:?}")]
    AmbiguousProcessor {
        /// The given string for the processor name.
        processor_short_name: String,
        /// The list of processors that might match it.
        ambiguous_processor_names: Vec<&'static str>,
    },
    /// Encountered an [`AssetReaderError`] for this path.
    #[error("Encountered an AssetReader error for '{path}': {err}")]
    #[from(ignore)]
    AssetReaderError {
        path: AssetPath<'static>,
        err: AssetReaderError,
    },
    /// Encountered an [`AssetWriterError`] for this path.
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
    /// Encountered an [`AssetReaderError`] when reading the asset metadata for this path.
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
    /// The wrong meta type was passed into a processor.
    /// This is probably an internal implementation error.
    #[error("The wrong meta type was passed into a processor. This is probably an internal implementation error.")]
    WrongMetaType,
    /// Encountered an error while saving the asset.
    #[error("Encountered an error while saving the asset: {0}")]
    #[from(ignore)]
    AssetSaveError(BevyError),
    /// Encountered an error while transforming the asset.
    #[error("Encountered an error while transforming the asset: {0}")]
    #[from(ignore)]
    AssetTransformError(Box<dyn core::error::Error + Send + Sync + 'static>),
    /// Assets without extensions are not supported.
    #[error("Assets without extensions are not supported.")]
    ExtensionRequired,
}

#[expect(
    deprecated,
    reason = "We need to maintain the trait impls until we delete `LoadTransformAndSave`"
)]
impl<Loader, Transformer, Saver> Process for LoadTransformAndSave<Loader, Transformer, Saver>
where
    Loader: AssetLoader,
    Transformer: AssetTransformer<AssetInput = Loader::Asset>,
    Saver: AssetSaver<Asset = Transformer::AssetOutput>,
{
    type Settings =
        LoadTransformAndSaveSettings<Loader::Settings, Transformer::Settings, Saver::Settings>;
    type OutputLoader = Saver::OutputLoader;

    async fn process(
        &self,
        context: &mut ProcessContext<'_>,
        settings: &Self::Settings,
        writer: &mut Writer,
    ) -> Result<<Self::OutputLoader as AssetLoader>::Settings, ProcessError> {
        load_transform_and_save::<Loader, Transformer, Saver>(
            (&self.transformer, &self.saver),
            (
                &settings.loader_settings,
                &settings.transformer_settings,
                &settings.saver_settings,
            ),
            context,
            writer,
        )
        .await
    }
}

/// Loads the reader in `context` with the `L` loader, transforms it with the `T` transformer, and
/// saves it to `writer` with the `S` saver.
pub async fn load_transform_and_save<L, T, S>(
    (transformer, saver): (&T, &S),
    (loader_settings, transformer_settings, saver_settings): (
        &L::Settings,
        &T::Settings,
        &S::Settings,
    ),
    context: &mut ProcessContext<'_>,
    writer: &mut Writer,
) -> Result<<<S as AssetSaver>::OutputLoader as AssetLoader>::Settings, ProcessError>
where
    L: AssetLoader,
    T: AssetTransformer<AssetInput = L::Asset>,
    S: AssetSaver<Asset = T::AssetOutput>,
{
    let pre_transformed_asset = TransformedAsset::<L::Asset>::from_loaded(
        context.load_source_asset::<L>(loader_settings).await?,
    )
    .unwrap();

    let post_transformed_asset = transformer
        .transform(pre_transformed_asset, transformer_settings)
        .await
        .map_err(|err| ProcessError::AssetTransformError(err.into()))?;

    let saved_asset = SavedAsset::<T::AssetOutput>::from_transformed(&post_transformed_asset);

    let output_settings = saver
        .save(writer, saved_asset, saver_settings, context.path.clone())
        .await
        .map_err(|error| ProcessError::AssetSaveError(error.into()))?;
    Ok(output_settings)
}

/// A type-erased variant of [`Process`] that enables interacting with processor implementations without knowing
/// their type.
pub trait ErasedProcessor: Send + Sync {
    /// Type-erased variant of [`Process::process`].
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        settings: &'a dyn Settings,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<Box<dyn AssetMetaDyn>, ProcessError>>;
    /// Deserialized `meta` as type-erased [`AssetMeta`], operating under the assumption that it matches the meta
    /// for the underlying [`Process`] impl.
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    /// Returns the type-path of the original [`Process`].
    fn type_path(&self) -> &'static str;
    /// Returns the short type path of this processor.
    fn short_type_path(&self) -> &'static str;
    /// Returns the default type-erased [`AssetMeta`] for the underlying [`Process`] impl.
    fn default_meta(&self, processor_path_kind: MetaTypePathKind) -> Box<dyn AssetMetaDyn>;
}

/// Specifies which kind of path to use to specify a type.
pub enum MetaTypePathKind {
    /// Use the short type path.
    Short,
    /// Use the fully-qualified type path.
    Long,
}

impl<P: Process> ErasedProcessor for P {
    fn process<'a>(
        &'a self,
        context: &'a mut ProcessContext,
        settings: &'a dyn Settings,
        writer: &'a mut Writer,
    ) -> BoxedFuture<'a, Result<Box<dyn AssetMetaDyn>, ProcessError>> {
        Box::pin(async move {
            let settings = settings.downcast_ref().ok_or(ProcessError::WrongMetaType)?;
            let loader_settings = <P as Process>::process(self, context, settings, writer).await?;
            let output_meta: Box<dyn AssetMetaDyn> =
                Box::new(AssetMeta::<P::OutputLoader, ()>::new(AssetAction::Load {
                    loader: P::OutputLoader::type_path().to_string(),
                    settings: loader_settings,
                }));
            Ok(output_meta)
        })
    }

    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError> {
        let meta: AssetMeta<(), P> = ron::de::from_bytes(meta)?;
        Ok(Box::new(meta))
    }

    fn type_path(&self) -> &'static str {
        P::type_path()
    }

    fn short_type_path(&self) -> &'static str {
        P::short_type_path()
    }

    fn default_meta(&self, processor_path_kind: MetaTypePathKind) -> Box<dyn AssetMetaDyn> {
        let type_path = match processor_path_kind {
            MetaTypePathKind::Short => P::short_type_path(),
            MetaTypePathKind::Long => P::type_path(),
        };
        Box::new(AssetMeta::<(), P>::new(AssetAction::Process {
            processor: type_path.to_string(),
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
