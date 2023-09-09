use crate::{
    io::{AssetReaderError, Reader},
    meta::{
        loader_settings_meta_transform, AssetHash, AssetMeta, AssetMetaDyn, ProcessedInfoMinimal,
        Settings,
    },
    path::AssetPath,
    Asset, AssetLoadError, AssetServer, Assets, Handle, UntypedAssetId, UntypedHandle,
};
use bevy_ecs::world::World;
use bevy_utils::{BoxedFuture, CowArc, HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use futures_lite::AsyncReadExt;
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    path::Path,
};
use thiserror::Error;

/// Loads an [`Asset`] from a given byte [`Reader`]. This can accept [`AssetLoader::Settings`], which configure how the [`Asset`]
/// should be loaded.
pub trait AssetLoader: Send + Sync + 'static {
    /// The top level [`Asset`] loaded by this [`AssetLoader`].
    type Asset: crate::Asset;
    /// The settings type used by this [`AssetLoader`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// Asynchronously loads [`AssetLoader::Asset`] (and any other labeled assets) from the bytes provided by [`Reader`].
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>>;

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
    fn extensions(&self) -> &[&str];
}

/// Provides type-erased access to an [`AssetLoader`].
pub trait ErasedAssetLoader: Send + Sync + 'static {
    /// Asynchronously loads the asset(s) from the bytes provided by [`Reader`].
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        meta: Box<dyn AssetMetaDyn>,
        load_context: LoadContext<'a>,
    ) -> BoxedFuture<'a, Result<ErasedLoadedAsset, AssetLoaderError>>;

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
    fn extensions(&self) -> &[&str];
    /// Deserializes metadata from the input `meta` bytes into the appropriate type (erased as [`Box<dyn AssetMetaDyn>`]).
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    /// Returns the default meta value for the [`AssetLoader`] (erased as [`Box<dyn AssetMetaDyn>`]).
    fn default_meta(&self) -> Box<dyn AssetMetaDyn>;
    /// Returns the type name of the [`AssetLoader`].
    fn type_name(&self) -> &'static str;
    /// Returns the [`TypeId`] of the [`AssetLoader`].
    fn type_id(&self) -> TypeId;
    /// Returns the type name of the top-level [`Asset`] loaded by the [`AssetLoader`].
    fn asset_type_name(&self) -> &'static str;
    /// Returns the [`TypeId`] of the top-level [`Asset`] loaded by the [`AssetLoader`].
    fn asset_type_id(&self) -> TypeId;
}

/// An error encountered during [`AssetLoader::load`].
#[derive(Error, Debug)]
pub enum AssetLoaderError {
    /// Any error that occurs during load.
    #[error(transparent)]
    Load(#[from] anyhow::Error),
    /// A failure to deserialize metadata during load.
    #[error(transparent)]
    DeserializeMeta(#[from] DeserializeMetaError),
}

impl<L> ErasedAssetLoader for L
where
    L: AssetLoader + Send + Sync,
{
    /// Processes the asset in an asynchronous closure.
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        meta: Box<dyn AssetMetaDyn>,
        mut load_context: LoadContext<'a>,
    ) -> BoxedFuture<'a, Result<ErasedLoadedAsset, AssetLoaderError>> {
        Box::pin(async move {
            let settings = meta
                .loader_settings()
                .expect("Loader settings should exist")
                .downcast_ref::<L::Settings>()
                .expect("AssetLoader settings should match the loader type");
            let asset = <L as AssetLoader>::load(self, reader, settings, &mut load_context).await?;
            Ok(load_context.finish(asset, Some(meta)).into())
        })
    }

    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError> {
        let meta = AssetMeta::<L, ()>::deserialize(meta)?;
        Ok(Box::new(meta))
    }

    fn default_meta(&self) -> Box<dyn AssetMetaDyn> {
        Box::new(AssetMeta::<L, ()>::new(crate::meta::AssetAction::Load {
            loader: self.type_name().to_string(),
            settings: L::Settings::default(),
        }))
    }

    fn extensions(&self) -> &[&str] {
        <L as AssetLoader>::extensions(self)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<L>()
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<L>()
    }

    fn asset_type_id(&self) -> TypeId {
        TypeId::of::<L::Asset>()
    }

    fn asset_type_name(&self) -> &'static str {
        std::any::type_name::<L::Asset>()
    }
}

pub(crate) struct LabeledAsset {
    pub(crate) asset: ErasedLoadedAsset,
    pub(crate) handle: UntypedHandle,
}

/// The successful result of an [`AssetLoader::load`] call. This contains the loaded "root" asset and any other "labeled" assets produced
/// by the loader. It also holds the input [`AssetMeta`] (if it exists) and tracks dependencies:
/// * normal dependencies: dependencies that must be loaded as part of this asset load (ex: assets a given asset has handles to).
/// * Loader dependencies: dependencies whose actual asset values are used during the load process
pub struct LoadedAsset<A: Asset> {
    pub(crate) value: A,
    pub(crate) dependencies: HashSet<UntypedAssetId>,
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    pub(crate) labeled_assets: HashMap<CowArc<'static, str>, LabeledAsset>,
    pub(crate) meta: Option<Box<dyn AssetMetaDyn>>,
}

impl<A: Asset> LoadedAsset<A> {
    /// Create a new loaded asset. This will use [`VisitAssetDependencies`](crate::VisitAssetDependencies) to populate `dependencies`.
    pub fn new_with_dependencies(value: A, meta: Option<Box<dyn AssetMetaDyn>>) -> Self {
        let mut dependencies = HashSet::new();
        value.visit_dependencies(&mut |id| {
            dependencies.insert(id);
        });
        LoadedAsset {
            value,
            dependencies,
            loader_dependencies: HashMap::default(),
            labeled_assets: HashMap::default(),
            meta,
        }
    }
}

impl<A: Asset> From<A> for LoadedAsset<A> {
    fn from(asset: A) -> Self {
        LoadedAsset::new_with_dependencies(asset, None)
    }
}

/// A "type erased / boxed" counterpart to [`LoadedAsset`]. This is used in places where the loaded type is not statically known.
pub struct ErasedLoadedAsset {
    pub(crate) value: Box<dyn AssetContainer>,
    pub(crate) dependencies: HashSet<UntypedAssetId>,
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    pub(crate) labeled_assets: HashMap<CowArc<'static, str>, LabeledAsset>,
    pub(crate) meta: Option<Box<dyn AssetMetaDyn>>,
}

impl<A: Asset> From<LoadedAsset<A>> for ErasedLoadedAsset {
    fn from(asset: LoadedAsset<A>) -> Self {
        ErasedLoadedAsset {
            value: Box::new(asset.value),
            dependencies: asset.dependencies,
            loader_dependencies: asset.loader_dependencies,
            labeled_assets: asset.labeled_assets,
            meta: asset.meta,
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

    /// Returns the [`ErasedLoadedAsset`] for the given label, if it exists.
    pub fn get_labeled(
        &self,
        label: impl Into<CowArc<'static, str>>,
    ) -> Option<&ErasedLoadedAsset> {
        self.labeled_assets.get(&label.into()).map(|a| &a.asset)
    }

    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.labeled_assets.keys().map(|s| &**s)
    }
}

/// A type erased container for an [`Asset`] value that is capable of inserting the [`Asset`] into a [`World`]'s [`Assets`] collection.
pub trait AssetContainer: Downcast + Any + Send + Sync + 'static {
    fn insert(self: Box<Self>, id: UntypedAssetId, world: &mut World);
    fn asset_type_name(&self) -> &'static str;
}

impl_downcast!(AssetContainer);

impl<A: Asset> AssetContainer for A {
    fn insert(self: Box<Self>, id: UntypedAssetId, world: &mut World) {
        world.resource_mut::<Assets<A>>().insert(id.typed(), *self);
    }

    fn asset_type_name(&self) -> &'static str {
        std::any::type_name::<A>()
    }
}

/// An error that occurs when attempting to call [`LoadContext::load_direct`]
#[derive(Error, Debug)]
#[error("Failed to load dependency {dependency:?} {error}")]
pub struct LoadDirectError {
    pub dependency: AssetPath<'static>,
    pub error: AssetLoadError,
}

/// An error that occurs while deserializing [`AssetMeta`].
#[derive(Error, Debug)]
pub enum DeserializeMetaError {
    #[error("Failed to deserialize asset meta: {0:?}")]
    DeserializeSettings(#[from] SpannedError),
    #[error("Failed to deserialize minimal asset meta: {0:?}")]
    DeserializeMinimal(SpannedError),
}

/// A context that provides access to assets in [`AssetLoader`]s, tracks dependencies, and collects asset load state.
/// Any asset state accessed by [`LoadContext`] will be tracked and stored for use in dependency events and asset preprocessing.
pub struct LoadContext<'a> {
    asset_server: &'a AssetServer,
    should_load_dependencies: bool,
    populate_hashes: bool,
    asset_path: AssetPath<'static>,
    dependencies: HashSet<UntypedAssetId>,
    /// Direct dependencies used by this loader.
    loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    labeled_assets: HashMap<CowArc<'static, str>, LabeledAsset>,
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
            labeled_assets: HashMap::default(),
        }
    }

    /// Begins a new labeled asset load. Use the returned [`LoadContext`] to load
    /// dependencies for the new asset and call [`LoadContext::finish`] to finalize the asset load.
    /// When finished, make sure you call [`LoadContext::add_labeled_asset`] to add the results back to the parent
    /// context.
    /// Prefer [`LoadContext::labeled_asset_scope`] when possible, which will automatically add
    /// the labeled [`LoadContext`] back to the parent context.
    /// [`LoadContext::begin_labeled_asset`] exists largely to enable parallel asset loading.
    ///
    /// See [`AssetPath`] for more on labeled assets.
    ///
    /// ```no_run
    /// # use bevy_asset::{Asset, LoadContext};
    /// # use bevy_reflect::TypePath;
    /// # #[derive(Asset, TypePath, Default)]
    /// # struct Image;
    /// # let load_context: LoadContext = panic!();
    /// let mut handles = Vec::new();
    /// for i in 0..2 {
    ///     let mut labeled = load_context.begin_labeled_asset();
    ///     handles.push(std::thread::spawn(move || {
    ///         (i.to_string(), labeled.finish(Image::default(), None))
    ///     }));
    /// }

    /// for handle in handles {
    ///     let (label, loaded_asset) = handle.join().unwrap();
    ///     load_context.add_loaded_labeled_asset(label, loaded_asset);
    /// }
    /// ```
    pub fn begin_labeled_asset(&self) -> LoadContext {
        LoadContext::new(
            self.asset_server,
            self.asset_path.clone(),
            self.should_load_dependencies,
            self.populate_hashes,
        )
    }

    /// Creates a new [`LoadContext`] for the given `label`. The `load` function is responsible for loading an [`Asset`] of
    /// type `A`. `load` will be called immediately and the result will be used to finalize the [`LoadContext`], resulting in a new
    /// [`LoadedAsset`], which is registered under the `label` label.
    ///
    /// This exists to remove the need to manually call [`LoadContext::begin_labeled_asset`] and then manually register the
    /// result with [`LoadContext::add_labeled_asset`].
    ///
    /// See [`AssetPath`] for more on labeled assets.
    pub fn labeled_asset_scope<A: Asset>(
        &mut self,
        label: String,
        load: impl FnOnce(&mut LoadContext) -> A,
    ) -> Handle<A> {
        let mut context = self.begin_labeled_asset();
        let asset = load(&mut context);
        let loaded_asset = context.finish(asset, None);
        self.add_loaded_labeled_asset(label, loaded_asset)
    }

    /// This will add the given `asset` as a "labeled [`Asset`]" with the `label` label.
    ///
    /// See [`AssetPath`] for more on labeled assets.
    pub fn add_labeled_asset<A: Asset>(&mut self, label: String, asset: A) -> Handle<A> {
        self.labeled_asset_scope(label, |_| asset)
    }

    /// Add a [`LoadedAsset`] that is a "labeled sub asset" of the root path of this load context.
    /// This can be used in combination with [`LoadContext::begin_labeled_asset`] to parallelize
    /// sub asset loading.
    ///
    /// See [`AssetPath`] for more on labeled assets.
    pub fn add_loaded_labeled_asset<A: Asset>(
        &mut self,
        label: impl Into<CowArc<'static, str>>,
        loaded_asset: LoadedAsset<A>,
    ) -> Handle<A> {
        let label = label.into();
        let loaded_asset: ErasedLoadedAsset = loaded_asset.into();
        let labeled_path = self.asset_path.with_label(label.clone());
        let handle = self
            .asset_server
            .get_or_create_path_handle(labeled_path, None);
        self.labeled_assets.insert(
            label,
            LabeledAsset {
                asset: loaded_asset,
                handle: handle.clone().untyped(),
            },
        );
        handle
    }

    /// Returns `true` if an asset with the label `label` exists in this context.
    ///
    /// See [`AssetPath`] for more on labeled assets.
    pub fn has_labeled_asset<'b>(&self, label: impl Into<CowArc<'b, str>>) -> bool {
        let path = self.asset_path.with_label(label.into());
        self.asset_server.get_handle_untyped(&path).is_some()
    }

    /// "Finishes" this context by populating the final [`Asset`] value (and the erased [`AssetMeta`] value, if it exists).
    /// The relevant asset metadata collected in this context will be stored in the returned [`LoadedAsset`].
    pub fn finish<A: Asset>(self, value: A, meta: Option<Box<dyn AssetMetaDyn>>) -> LoadedAsset<A> {
        LoadedAsset {
            value,
            dependencies: self.dependencies,
            loader_dependencies: self.loader_dependencies,
            labeled_assets: self.labeled_assets,
            meta,
        }
    }

    /// Gets the source path for this load context.
    pub fn path(&self) -> &Path {
        self.asset_path.path()
    }

    /// Gets the source asset path for this load context.
    pub fn asset_path(&self) -> &AssetPath<'static> {
        &self.asset_path
    }

    /// Gets the source asset path for this load context.
    pub async fn read_asset_bytes<'b>(
        &mut self,
        path: &'b Path,
    ) -> Result<Vec<u8>, ReadAssetBytesError> {
        let mut reader = self.asset_server.reader().read(path).await?;
        let hash = if self.populate_hashes {
            // NOTE: ensure meta is read while the asset bytes reader is still active to ensure transactionality
            // See `ProcessorGatedReader` for more info
            let meta_bytes = self.asset_server.reader().read_meta_bytes(path).await?;
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
        reader.read_to_end(&mut bytes).await?;
        self.loader_dependencies
            .insert(AssetPath::from_path(path.to_owned()), hash);
        Ok(bytes)
    }

    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset.
    /// If the current context is a normal [`AssetServer::load`], an actual asset load will be kicked off immediately, which ensures the load happens
    /// as soon as possible.
    /// "Normal loads" kicked from within a normal Bevy App will generally configure the context to kick off loads immediately.  
    /// If the current context is configured to not load dependencies automatically (ex: [`AssetProcessor`](crate::processor::AssetProcessor)),
    /// a load will not be kicked off automatically. It is then the calling context's responsibility to begin a load if necessary.
    pub fn load<'b, A: Asset>(&mut self, path: impl Into<AssetPath<'b>>) -> Handle<A> {
        let path = path.into().to_owned();
        let handle = if self.should_load_dependencies {
            self.asset_server.load(path)
        } else {
            self.asset_server.get_or_create_path_handle(path, None)
        };
        self.dependencies.insert(handle.id().untyped());
        handle
    }

    /// Loads the [`Asset`] of type `A` at the given `path` with the given [`AssetLoader::Settings`] settings `S`. This is a "deferred"
    /// load. If the settings type `S` does not match the settings expected by `A`'s asset loader, an error will be printed to the log
    /// and the asset load will fail.  
    pub fn load_with_settings<'b, A: Asset, S: Settings + Default>(
        &mut self,
        path: impl Into<AssetPath<'b>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A> {
        let path = path.into();
        let handle = if self.should_load_dependencies {
            self.asset_server.load_with_settings(path.clone(), settings)
        } else {
            self.asset_server
                .get_or_create_path_handle(path, Some(loader_settings_meta_transform(settings)))
        };
        self.dependencies.insert(handle.id().untyped());
        handle
    }

    /// Returns a handle to an asset of type `A` with the label `label`. This [`LoadContext`] must produce an asset of the
    /// given type and the given label or the dependencies of this asset will never be considered "fully loaded". However you
    /// can call this method before _or_ after adding the labeled asset.
    pub fn get_label_handle<'b, A: Asset>(
        &mut self,
        label: impl Into<CowArc<'b, str>>,
    ) -> Handle<A> {
        let path = self.asset_path.with_label(label);
        let handle = self.asset_server.get_or_create_path_handle::<A>(path, None);
        self.dependencies.insert(handle.id().untyped());
        handle
    }

    /// Loads the asset at the given `path` directly. This is an async function that will wait until the asset is fully loaded before
    /// returning. Use this if you need the _value_ of another asset in order to load the current asset. For example, if you are
    /// deriving a new asset from the referenced asset, or you are building a collection of assets. This will add the `path` as a
    /// "load dependency".
    ///
    /// If the current loader is used in a [`Process`] "asset preprocessor", such as a [`LoadAndSave`] preprocessor,
    /// changing a "load dependency" will result in re-processing of the asset.
    ///
    /// [`Process`]: crate::processor::Process
    /// [`LoadAndSave`]: crate::processor::LoadAndSave
    pub async fn load_direct<'b>(
        &mut self,
        path: impl Into<AssetPath<'b>>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let path = path.into().into_owned();
        let to_error = |e: AssetLoadError| -> LoadDirectError {
            LoadDirectError {
                dependency: path.clone(),
                error: e,
            }
        };
        let loaded_asset = {
            let (meta, loader, mut reader) = self
                .asset_server
                .get_meta_loader_and_reader(&path)
                .await
                .map_err(to_error)?;
            self.asset_server
                .load_with_meta_loader_and_reader(
                    &path,
                    meta,
                    &*loader,
                    &mut *reader,
                    false,
                    self.populate_hashes,
                )
                .await
                .map_err(to_error)?
        };
        let info = loaded_asset
            .meta
            .as_ref()
            .and_then(|m| m.processed_info().as_ref());
        let hash = info.map(|i| i.full_hash).unwrap_or(Default::default());
        self.loader_dependencies.insert(path, hash);
        Ok(loaded_asset)
    }
}

/// An error produced when calling [`LoadContext::read_asset_bytes`]
#[derive(Error, Debug)]
pub enum ReadAssetBytesError {
    #[error(transparent)]
    DeserializeMetaError(#[from] DeserializeMetaError),
    #[error(transparent)]
    AssetReaderError(#[from] AssetReaderError),
    /// Encountered an I/O error while loading an asset.
    #[error("Encountered an io error while loading asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("The LoadContext for this read_asset_bytes call requires hash metadata, but it was not provided. This is likely an internal implementation error.")]
    MissingAssetHash,
}
