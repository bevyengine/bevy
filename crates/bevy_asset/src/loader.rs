use crate::{
    io::{AssetReaderError, MissingAssetSourceError, MissingProcessedAssetReaderError, Reader},
    loader_builders::{Deferred, NestedLoader, StaticTyped},
    meta::{AssetHash, AssetMeta, AssetMetaDyn, ProcessedInfo, ProcessedInfoMinimal, Settings},
    path::AssetPath,
    Asset, AssetIndex, AssetLoadError, AssetServer, AssetServerMode, Assets, ErasedAssetIndex,
    Handle, UntypedAssetId, UntypedHandle,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use atomicow::CowArc;
use bevy_ecs::{error::BevyError, world::World};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::TypePath;
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::any::{Any, TypeId};
use downcast_rs::{impl_downcast, Downcast};
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Loads an [`Asset`] from a given byte [`Reader`]. This can accept [`AssetLoader::Settings`], which configure how the [`Asset`]
/// should be loaded.
///
/// This trait is generally used in concert with [`AssetReader`](crate::io::AssetReader) to load assets from a byte source.
///
/// For a complementary version of this trait that can save assets, see [`AssetSaver`](crate::saver::AssetSaver).
pub trait AssetLoader: TypePath + Send + Sync + 'static {
    /// The top level [`Asset`] loaded by this [`AssetLoader`].
    type Asset: Asset;
    /// The settings type used by this [`AssetLoader`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [error](`std::error::Error`) which could be encountered by this loader.
    type Error: Into<BevyError>;
    /// Asynchronously loads [`AssetLoader::Asset`] (and any other subassets) from the bytes provided by [`Reader`].
    fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>>;

    /// Returns a list of extensions supported by this [`AssetLoader`], without the preceding dot.
    /// Note that users of this [`AssetLoader`] may choose to load files with a non-matching extension.
    fn extensions(&self) -> &[&str] {
        &[]
    }
}

/// Provides type-erased access to an [`AssetLoader`].
pub trait ErasedAssetLoader: Send + Sync + 'static {
    /// Asynchronously loads the asset(s) from the bytes provided by [`Reader`].
    fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        settings: &'a dyn Settings,
        load_context: LoadContext<'a>,
    ) -> BoxedFuture<'a, Result<ErasedLoadedAsset, BevyError>>;

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
    fn extensions(&self) -> &[&str];
    /// Deserializes metadata from the input `meta` bytes into the appropriate type (erased as [`Box<dyn AssetMetaDyn>`]).
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    /// Returns the default meta value for the [`AssetLoader`] (erased as [`Box<dyn AssetMetaDyn>`]).
    fn default_meta(&self) -> Box<dyn AssetMetaDyn>;
    /// Returns the type path of the [`AssetLoader`].
    fn type_path(&self) -> &'static str;
    /// Returns the [`TypeId`] of the [`AssetLoader`].
    fn type_id(&self) -> TypeId;
    /// Returns the type name of the top-level [`Asset`] loaded by the [`AssetLoader`].
    fn asset_type_name(&self) -> &'static str;
    /// Returns the [`TypeId`] of the top-level [`Asset`] loaded by the [`AssetLoader`].
    fn asset_type_id(&self) -> TypeId;
}

impl<L> ErasedAssetLoader for L
where
    L: AssetLoader + Send + Sync,
{
    /// Processes the asset in an asynchronous closure.
    fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        settings: &'a dyn Settings,
        mut load_context: LoadContext<'a>,
    ) -> BoxedFuture<'a, Result<ErasedLoadedAsset, BevyError>> {
        Box::pin(async move {
            let settings = settings
                .downcast_ref::<L::Settings>()
                .expect("AssetLoader settings should match the loader type");
            let asset = <L as AssetLoader>::load(self, reader, settings, &mut load_context)
                .await
                .map_err(Into::into)?;
            Ok(load_context.finish(asset).into())
        })
    }

    fn extensions(&self) -> &[&str] {
        <L as AssetLoader>::extensions(self)
    }

    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError> {
        let meta = AssetMeta::<L, ()>::deserialize(meta)?;
        Ok(Box::new(meta))
    }

    fn default_meta(&self) -> Box<dyn AssetMetaDyn> {
        Box::new(AssetMeta::<L, ()>::new(crate::meta::AssetAction::Load {
            loader: self.type_path().to_string(),
            settings: L::Settings::default(),
        }))
    }

    fn type_path(&self) -> &'static str {
        L::type_path()
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<L>()
    }

    fn asset_type_name(&self) -> &'static str {
        core::any::type_name::<L::Asset>()
    }

    fn asset_type_id(&self) -> TypeId {
        TypeId::of::<L::Asset>()
    }
}

pub(crate) struct LoadedSubAsset {
    pub(crate) asset: ErasedLoadedAsset,
    pub(crate) handle: UntypedHandle,
}

/// The successful result of an [`AssetLoader::load`] call. This contains the loaded "root" asset and any other "sub-assets" produced
/// by the loader. It also holds the input [`AssetMeta`] (if it exists) and tracks dependencies:
/// * normal dependencies: dependencies that must be loaded as part of this asset load (ex: assets a given asset has handles to).
/// * Loader dependencies: dependencies whose actual asset values are used during the load process
pub struct LoadedAsset<A: Asset> {
    pub(crate) value: A,
    pub(crate) dependencies: HashSet<ErasedAssetIndex>,
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    pub(crate) subassets: HashMap<CowArc<'static, str>, LoadedSubAsset>,
}

impl<A: Asset> LoadedAsset<A> {
    /// Create a new loaded asset. This will use [`VisitAssetDependencies`](crate::VisitAssetDependencies) to populate `dependencies`.
    pub fn new_with_dependencies(value: A) -> Self {
        let mut dependencies = <HashSet<_>>::default();
        value.visit_dependencies(&mut |id| {
            let Ok(asset_index) = id.try_into() else {
                return;
            };
            dependencies.insert(asset_index);
        });
        LoadedAsset {
            value,
            dependencies,
            loader_dependencies: HashMap::default(),
            subassets: HashMap::default(),
        }
    }

    /// Cast (and take ownership) of the [`Asset`] value of the given type.
    pub fn take(self) -> A {
        self.value
    }

    /// Retrieves a reference to the internal [`Asset`] type.
    pub fn get(&self) -> &A {
        &self.value
    }

    /// Returns the [`ErasedLoadedAsset`] for the given subasset name, if it exists.
    pub fn get_subasset(
        &self,
        subasset_name: impl Into<CowArc<'static, str>>,
    ) -> Option<&ErasedLoadedAsset> {
        self.subassets.get(&subasset_name.into()).map(|a| &a.asset)
    }

    /// Iterate over all subasset names for subassets in this loaded asset.
    pub fn iter_subasset_names(&self) -> impl Iterator<Item = &str> {
        self.subassets.keys().map(|s| &**s)
    }
}

impl<A: Asset> From<A> for LoadedAsset<A> {
    fn from(asset: A) -> Self {
        LoadedAsset::new_with_dependencies(asset)
    }
}

/// A "type erased / boxed" counterpart to [`LoadedAsset`]. This is used in places where the loaded type is not statically known.
pub struct ErasedLoadedAsset {
    pub(crate) value: Box<dyn AssetContainer>,
    pub(crate) dependencies: HashSet<ErasedAssetIndex>,
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    pub(crate) subassets: HashMap<CowArc<'static, str>, LoadedSubAsset>,
}

impl<A: Asset> From<LoadedAsset<A>> for ErasedLoadedAsset {
    fn from(asset: LoadedAsset<A>) -> Self {
        ErasedLoadedAsset {
            value: Box::new(asset.value),
            dependencies: asset.dependencies,
            loader_dependencies: asset.loader_dependencies,
            subassets: asset.subassets,
        }
    }
}

impl ErasedLoadedAsset {
    /// Cast (and take ownership) of the [`Asset`] value of the given type. This will return [`Some`] if
    /// the stored type matches `A` and [`None`] if it does not.
    pub fn take<A: Asset>(self) -> Option<A> {
        self.value.downcast::<A>().map(|a| *a).ok()
    }

    /// Retrieves a reference to the internal [`Asset`] type, if it matches the type `A`. Otherwise returns [`None`].
    pub fn get<A: Asset>(&self) -> Option<&A> {
        self.value.downcast_ref::<A>()
    }

    /// Retrieves the [`TypeId`] of the stored [`Asset`] type.
    pub fn asset_type_id(&self) -> TypeId {
        (*self.value).type_id()
    }

    /// Retrieves the `type_name` of the stored [`Asset`] type.
    pub fn asset_type_name(&self) -> &'static str {
        self.value.asset_type_name()
    }

    /// Returns the [`ErasedLoadedAsset`] for the given subasset name, if it exists.
    pub fn get_subasset(
        &self,
        subasset_name: impl Into<CowArc<'static, str>>,
    ) -> Option<&ErasedLoadedAsset> {
        self.subassets.get(&subasset_name.into()).map(|a| &a.asset)
    }

    /// Iterate over all subasset names for subassets in this loaded asset.
    pub fn iter_subasset_names(&self) -> impl Iterator<Item = &str> {
        self.subassets.keys().map(|s| &**s)
    }

    /// Cast this loaded asset as the given type. If the type does not match,
    /// the original type-erased asset is returned.
    pub fn downcast<A: Asset>(mut self) -> Result<LoadedAsset<A>, ErasedLoadedAsset> {
        match self.value.downcast::<A>() {
            Ok(value) => Ok(LoadedAsset {
                value: *value,
                dependencies: self.dependencies,
                loader_dependencies: self.loader_dependencies,
                subassets: self.subassets,
            }),
            Err(value) => {
                self.value = value;
                Err(self)
            }
        }
    }
}

/// A type erased container for an [`Asset`] value that is capable of inserting the [`Asset`] into a [`World`]'s [`Assets`] collection.
pub(crate) trait AssetContainer: Downcast + Any + Send + Sync + 'static {
    fn insert(self: Box<Self>, id: AssetIndex, world: &mut World);
    fn asset_type_name(&self) -> &'static str;
}

impl_downcast!(AssetContainer);

impl<A: Asset> AssetContainer for A {
    fn insert(self: Box<Self>, index: AssetIndex, world: &mut World) {
        // We only ever call this if we know the asset is still alive, so it is fine to unwrap here.
        world
            .resource_mut::<Assets<A>>()
            .insert(index, *self)
            .expect("the AssetIndex is still valid");
    }

    fn asset_type_name(&self) -> &'static str {
        core::any::type_name::<A>()
    }
}

/// An error that occurs when attempting to call [`NestedLoader::load`] which
/// is configured to work [immediately].
///
/// [`NestedLoader::load`]: crate::NestedLoader::load
/// [immediately]: crate::Immediate
#[derive(Error, Debug)]
pub enum LoadDirectError {
    #[error("Requested to load an asset path ({0:?}) with a subasset, but this is unsupported. See issue #18291")]
    RequestedSubasset(AssetPath<'static>),
    #[error("Failed to load dependency {dependency:?} {error}")]
    LoadError {
        dependency: AssetPath<'static>,
        error: AssetLoadError,
    },
}

/// An error that occurs while deserializing [`AssetMeta`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DeserializeMetaError {
    #[error("Failed to deserialize asset meta: {0:?}")]
    DeserializeSettings(#[from] SpannedError),
    #[error("Failed to deserialize minimal asset meta: {0:?}")]
    DeserializeMinimal(SpannedError),
}

/// A context that provides access to assets in [`AssetLoader`]s, tracks dependencies, and collects asset load state.
///
/// Any asset state accessed by [`LoadContext`] will be tracked and stored for use in dependency events and asset preprocessing.
pub struct LoadContext<'a> {
    pub(crate) asset_server: &'a AssetServer,
    /// Specifies whether dependencies that are loaded deferred should be loaded.
    ///
    /// This allows us to skip loads for cases where we're never going to use the asset and we just
    /// need the dependency information, for example during asset processing.
    pub(crate) should_load_dependencies: bool,
    populate_hashes: bool,
    asset_path: AssetPath<'static>,
    pub(crate) dependencies: HashSet<ErasedAssetIndex>,
    /// Direct dependencies used by this loader.
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    pub(crate) subassets: HashMap<CowArc<'static, str>, LoadedSubAsset>,
}

impl<'a> LoadContext<'a> {
    /// Creates a new [`LoadContext`] instance.
    pub(crate) fn new(
        asset_server: &'a AssetServer,
        asset_path: AssetPath<'static>,
        should_load_dependencies: bool,
        populate_hashes: bool,
    ) -> Self {
        Self {
            asset_server,
            asset_path,
            populate_hashes,
            should_load_dependencies,
            dependencies: HashSet::default(),
            loader_dependencies: HashMap::default(),
            subassets: HashMap::default(),
        }
    }

    /// Begins a new subasset load. Use the returned [`LoadContext`] to load dependencies for the
    /// new asset and call [`LoadContext::finish`] to finalize the subasset load. When finished,
    /// make sure you call [`LoadContext::add_loaded_subasset`] to add the results back to the parent
    /// context.
    /// Prefer [`LoadContext::subasset_scope`] when possible, which will automatically add
    /// the subasset [`LoadContext`] back to the parent context.
    /// [`LoadContext::begin_subasset`] exists largely to enable parallel asset loading.
    ///
    /// See [`AssetPath`] for more on subassets.
    ///
    /// ```no_run
    /// # use bevy_asset::{Asset, LoadContext};
    /// # use bevy_reflect::TypePath;
    /// # #[derive(Asset, TypePath, Default)]
    /// # struct Image;
    /// # let load_context: LoadContext = panic!();
    /// let mut handles = Vec::new();
    /// for i in 0..2 {
    ///     let subasset = load_context.begin_subasset();
    ///     handles.push(std::thread::spawn(move || {
    ///         (i.to_string(), subasset.finish(Image::default()))
    ///     }));
    /// }
    ///
    /// for handle in handles {
    ///     let (subasset_name, loaded_asset) = handle.join().unwrap();
    ///     load_context.add_loaded_subasset(subasset_name, loaded_asset);
    /// }
    /// ```
    pub fn begin_subasset(&self) -> LoadContext<'_> {
        LoadContext::new(
            self.asset_server,
            self.asset_path.clone(),
            self.should_load_dependencies,
            self.populate_hashes,
        )
    }

    /// Creates a new [`LoadContext`] for the given `subasset_name`. The `load` function is responsible for loading an [`Asset`] of
    /// type `A`. `load` will be called immediately and the result will be used to finalize the [`LoadContext`], resulting in a new
    /// [`LoadedAsset`], which is registered under the `subasset_name`.
    ///
    /// This exists to remove the need to manually call [`LoadContext::begin_subasset`] and then manually register the
    /// result with [`LoadContext::add_loaded_subasset`].
    ///
    /// See [`AssetPath`] for more on subassets.
    pub fn subasset_scope<A: Asset, E>(
        &mut self,
        subasset_name: String,
        load: impl FnOnce(&mut LoadContext) -> Result<A, E>,
    ) -> Result<Handle<A>, E> {
        let mut context = self.begin_subasset();
        let asset = load(&mut context)?;
        let loaded_asset = context.finish(asset);
        Ok(self.add_loaded_subasset(subasset_name, loaded_asset))
    }

    /// This will add the given `asset` as a sub-[`Asset`] with the `subasset_name`.
    ///
    /// # Warning
    ///
    /// This will not assign dependencies to the given `asset`. If adding an asset
    /// with dependencies generated from calls such as [`LoadContext::load`], use
    /// [`LoadContext::subasset_scope`] or [`LoadContext::begin_subasset`] to generate a
    /// new [`LoadContext`] to track the dependencies for the subasset.
    ///
    /// See [`AssetPath`] for more on subassets.
    pub fn add_subasset<A: Asset>(&mut self, subasset_name: String, asset: A) -> Handle<A> {
        self.subasset_scope(subasset_name, |_| Ok::<_, ()>(asset))
            .expect("the closure returns Ok")
    }

    /// Add a [`LoadedAsset`] that is a "sub asset" of the root path of this load context.
    /// This can be used in combination with [`LoadContext::begin_subasset`] to parallelize
    /// sub asset loading.
    ///
    /// See [`AssetPath`] for more on subassets.
    pub fn add_loaded_subasset<A: Asset>(
        &mut self,
        subasset_name: impl Into<CowArc<'static, str>>,
        loaded_asset: LoadedAsset<A>,
    ) -> Handle<A> {
        let subasset_name = subasset_name.into();
        let loaded_asset: ErasedLoadedAsset = loaded_asset.into();
        let subasset_path = self
            .asset_path
            .clone()
            .with_subasset_name(subasset_name.clone());
        let handle = self
            .asset_server
            .get_or_create_path_handle(subasset_path, None);
        self.subassets.insert(
            subasset_name,
            LoadedSubAsset {
                asset: loaded_asset,
                handle: handle.clone().untyped(),
            },
        );
        handle
    }

    /// Returns `true` if an asset with the `subasset_name` exists in this context.
    ///
    /// See [`AssetPath`] for more on subassets.
    pub fn has_subasset<'b>(&self, subasset_name: impl Into<CowArc<'b, str>>) -> bool {
        let path = self
            .asset_path
            .clone()
            .with_subasset_name(subasset_name.into());
        !self.asset_server.get_handles_untyped(&path).is_empty()
    }

    /// "Finishes" this context by populating the final [`Asset`] value.
    pub fn finish<A: Asset>(mut self, value: A) -> LoadedAsset<A> {
        // At this point, we assume the asset/subasset is "locked in" and won't be changed, so we
        // can ensure all the dependencies are included (in case a handle was used without loading
        // it through this `LoadContext`). If in the future we provide an API for mutating assets in
        // `LoadedAsset`, `ErasedLoadedAsset`, or `LoadContext` (for mutating existing subassets),
        // we should move this to some point after those mutations are not possible. This spot is
        // convenient because we still have access to the static type of `A`.
        value.visit_dependencies(&mut |asset_id| {
            let (type_id, index) = match asset_id {
                UntypedAssetId::Index { type_id, index } => (type_id, index),
                // UUID assets can't be loaded anyway, so just ignore this ID.
                UntypedAssetId::Uuid { .. } => return,
            };
            self.dependencies
                .insert(ErasedAssetIndex { index, type_id });
        });
        LoadedAsset {
            value,
            dependencies: self.dependencies,
            loader_dependencies: self.loader_dependencies,
            subassets: self.subassets,
        }
    }

    /// Gets the source asset path for this load context.
    pub fn path(&self) -> &AssetPath<'static> {
        &self.asset_path
    }

    /// Reads the asset at the given path and returns its bytes
    pub async fn read_asset_bytes<'b, 'c>(
        &'b mut self,
        path: impl Into<AssetPath<'c>>,
    ) -> Result<Vec<u8>, ReadAssetBytesError> {
        let path = path.into();
        let source = self.asset_server.get_source(path.source())?;
        let asset_reader = match self.asset_server.mode() {
            AssetServerMode::Unprocessed => source.reader(),
            AssetServerMode::Processed => source.processed_reader()?,
        };
        let mut reader = asset_reader.read(path.path()).await?;
        let hash = if self.populate_hashes {
            // NOTE: ensure meta is read while the asset bytes reader is still active to ensure transactionality
            // See `ProcessorGatedReader` for more info
            let meta_bytes = asset_reader.read_meta_bytes(path.path()).await?;
            let minimal: ProcessedInfoMinimal = ron::de::from_bytes(&meta_bytes)
                .map_err(DeserializeMetaError::DeserializeMinimal)?;
            let processed_info = minimal
                .processed_info
                .ok_or(ReadAssetBytesError::MissingAssetHash)?;
            processed_info.full_hash
        } else {
            Default::default()
        };
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|source| ReadAssetBytesError::Io {
                path: path.path().to_path_buf(),
                source,
            })?;
        self.loader_dependencies.insert(path.clone_owned(), hash);
        Ok(bytes)
    }

    /// Returns a handle to an asset of type `A` with the `subasset_name`. This [`LoadContext`] must produce an asset of the
    /// given type and the given subasset name or the dependencies of this asset will never be considered "fully loaded". However you
    /// can call this method before _or_ after adding the subasset.
    pub fn get_subasset_handle<'b, A: Asset>(
        &mut self,
        subasset_name: impl Into<CowArc<'b, str>>,
    ) -> Handle<A> {
        let path = self.asset_path.clone().with_subasset_name(subasset_name);
        let handle = self.asset_server.get_or_create_path_handle::<A>(path, None);
        // `get_or_create_path_handle` always returns a Strong variant, so we are safe to unwrap.
        let index = (&handle).try_into().unwrap();
        self.dependencies.insert(index);
        handle
    }

    pub(crate) async fn load_direct_internal(
        &mut self,
        path: AssetPath<'static>,
        settings: &dyn Settings,
        loader: &dyn ErasedAssetLoader,
        reader: &mut dyn Reader,
        processed_info: Option<&ProcessedInfo>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let loaded_asset = self
            .asset_server
            .load_with_settings_loader_and_reader(
                &path,
                settings,
                loader,
                reader,
                self.should_load_dependencies,
                self.populate_hashes,
            )
            .await
            .map_err(|error| LoadDirectError::LoadError {
                dependency: path.clone(),
                error,
            })?;
        let hash = processed_info.map(|i| i.full_hash).unwrap_or_default();
        self.loader_dependencies.insert(path, hash);
        Ok(loaded_asset)
    }

    /// Create a builder for loading nested assets in this context.
    #[must_use]
    pub fn loader(&mut self) -> NestedLoader<'a, '_, StaticTyped, Deferred> {
        NestedLoader::new(self)
    }

    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset.
    /// If the current context is a normal [`AssetServer::load`], an actual asset load will be kicked off immediately, which ensures the load happens
    /// as soon as possible.
    /// "Normal loads" kicked from within a normal Bevy App will generally configure the context to kick off loads immediately.
    /// If the current context is configured to not load dependencies automatically (ex: [`AssetProcessor`](crate::processor::AssetProcessor)),
    /// a load will not be kicked off automatically. It is then the calling context's responsibility to begin a load if necessary.
    ///
    /// If you need to override asset settings, asset type, or load directly, please see [`LoadContext::loader`].
    pub fn load<'b, A: Asset>(&mut self, path: impl Into<AssetPath<'b>>) -> Handle<A> {
        self.loader().load(path)
    }
}

/// An error produced when calling [`LoadContext::read_asset_bytes`]
#[derive(Error, Debug)]
pub enum ReadAssetBytesError {
    #[error(transparent)]
    DeserializeMetaError(#[from] DeserializeMetaError),
    #[error(transparent)]
    AssetReaderError(#[from] AssetReaderError),
    #[error(transparent)]
    MissingAssetSourceError(#[from] MissingAssetSourceError),
    #[error(transparent)]
    MissingProcessedAssetReaderError(#[from] MissingProcessedAssetReaderError),
    /// Encountered an I/O error while loading an asset.
    #[error("Encountered an io error while loading asset at `{}`: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("The LoadContext for this read_asset_bytes call requires hash metadata, but it was not provided. This is likely an internal implementation error.")]
    MissingAssetHash,
}
