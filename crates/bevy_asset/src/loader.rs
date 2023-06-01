use crate::{
    io::{AssetReaderError, Reader},
    meta::{AssetMeta, AssetMetaDyn, AssetMetaProcessedInfoMinimal, Settings},
    path::AssetPath,
    Asset, AssetLoadError, AssetServer, Assets, Handle, UntypedAssetId, UntypedHandle,
};
use bevy_ecs::world::World;
use bevy_utils::{BoxedFuture, HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use futures_lite::AsyncReadExt;
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    path::Path,
};
use thiserror::Error;

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: crate::Asset;
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// Processes the asset in an asynchronous closure.
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>>;

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
    fn extensions(&self) -> &[&str];
}

pub trait ErasedAssetLoader: Send + Sync + 'static {
    /// Processes the asset in an asynchronous closure.
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        meta: Box<dyn AssetMetaDyn>,
        load_context: LoadContext<'a>,
    ) -> BoxedFuture<'a, Result<ErasedLoadedAsset, AssetLoaderError>>;

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
    fn extensions(&self) -> &[&str];
    fn deserialize_meta(&self, meta: &[u8]) -> Result<Box<dyn AssetMetaDyn>, DeserializeMetaError>;
    fn default_meta(&self) -> Box<dyn AssetMetaDyn>;
    fn type_name(&self) -> &'static str;
    fn type_id(&self) -> TypeId;
    fn asset_type_name(&self) -> &'static str;
    fn asset_type_id(&self) -> TypeId;
}

#[derive(Error, Debug)]
pub enum AssetLoaderError {
    #[error(transparent)]
    Load(#[from] anyhow::Error),
    #[error(transparent)]
    DeserializeMeta(#[from] DeserializeMetaError),
}

#[derive(Error, Debug)]
#[error("Failed to load dependency {dependency:?} {error}")]
pub struct LoadDirectError {
    pub dependency: AssetPath<'static>,
    pub error: AssetLoadError,
}

#[derive(Error, Debug)]
pub enum DeserializeMetaError {
    #[error("Failed to deserialize asset meta: {0:?}")]
    DeserializeSettings(#[from] SpannedError),
    #[error("Failed to deserialize minimal asset meta: {0:?}")]
    DeserializeMinimal(SpannedError),
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

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
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

pub struct LoadedAsset<A: Asset> {
    pub(crate) value: A,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) dependencies: HashSet<UntypedHandle>,
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, u64>,
    pub(crate) labeled_assets: HashMap<String, LabeledAsset>,
    pub(crate) meta: Option<Box<dyn AssetMetaDyn>>,
}

impl<A: Asset> LoadedAsset<A> {
    /// Create a new loaded asset. This will use [`AssetDependencyVisitor`](crate::AssetDependencyVisitor) to populate `dependencies`.
    pub fn new_with_dependencies(value: A, meta: Option<Box<dyn AssetMetaDyn>>) -> Self {
        let mut dependencies = HashSet::new();
        value.visit_dependencies(&mut |handle| {
            dependencies.insert(handle);
        });
        LoadedAsset {
            value,
            path: None,
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

pub struct ErasedLoadedAsset {
    pub(crate) value: Box<dyn AssetContainer>,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) dependencies: HashSet<UntypedHandle>,
    pub(crate) loader_dependencies: HashMap<AssetPath<'static>, u64>,
    pub(crate) labeled_assets: HashMap<String, LabeledAsset>,
    pub(crate) meta: Option<Box<dyn AssetMetaDyn>>,
}

impl<A: Asset> From<LoadedAsset<A>> for ErasedLoadedAsset {
    fn from(asset: LoadedAsset<A>) -> Self {
        ErasedLoadedAsset {
            value: Box::new(asset.value),
            path: asset.path,
            dependencies: asset.dependencies,
            loader_dependencies: asset.loader_dependencies,
            labeled_assets: asset.labeled_assets,
            meta: asset.meta,
        }
    }
}

impl ErasedLoadedAsset {
    pub fn take<A: Asset>(self) -> Option<A> {
        self.value.downcast::<A>().map(|a| *a).ok()
    }

    pub fn get<A: Asset>(&self) -> Option<&A> {
        self.value.downcast_ref::<A>()
    }

    pub fn asset_type_id(&self) -> TypeId {
        (*self.value).type_id()
    }

    pub fn path(&self) -> Option<&AssetPath<'static>> {
        self.path.as_ref()
    }
}

pub trait AssetContainer: Downcast + Any + Send + Sync + 'static {
    fn insert(self: Box<Self>, id: UntypedAssetId, world: &mut World);
}

impl_downcast!(AssetContainer);

impl<A: Asset> AssetContainer for A {
    fn insert(self: Box<Self>, id: UntypedAssetId, world: &mut World) {
        world.resource_mut::<Assets<A>>().insert(id.typed(), *self);
    }
}

pub struct LoadContext<'a> {
    asset_server: &'a AssetServer,
    should_load_dependencies: bool,
    populate_hashes: bool,
    asset_path: AssetPath<'static>,
    dependencies: HashSet<UntypedHandle>,
    /// Direct dependencies used by this loader.
    loader_dependencies: HashMap<AssetPath<'static>, u64>,
    labeled_assets: HashMap<String, LabeledAsset>,
}

impl<'a> LoadContext<'a> {
    pub(crate) fn new(
        asset_server: &'a AssetServer,
        asset_path: AssetPath<'static>,
        load_dependencies: bool,
        populate_hashes: bool,
    ) -> Self {
        Self {
            asset_server,
            asset_path,
            populate_hashes,
            should_load_dependencies: load_dependencies,
            dependencies: HashSet::default(),
            loader_dependencies: HashMap::default(),
            labeled_assets: HashMap::default(),
        }
    }

    /// Begins a new labeled asset load. Use the returned [`LoadContext`] to load
    /// dependencies for the new asset and call [`LoadContext::finish`] to finalize the asset load.
    /// When finished, make sure you call [`Self::add_labled_asset`] to add the results back to the parent
    /// context.
    /// Prefer [`Self::labeled_asset_scope`] when possible, which will automatically add
    /// the labeled [`LoadContext`] back to the parent context.
    /// [`Self::begin_labeled_asset`] exists largely to enable parallel asset loading.
    pub fn begin_labeled_asset(&self, label: String) -> LoadContext {
        LoadContext::new(
            self.asset_server,
            self.asset_path.with_label(label),
            self.should_load_dependencies,
            self.populate_hashes,
        )
    }

    pub fn labeled_asset_scope<A: Asset>(
        &mut self,
        label: String,
        load: impl FnOnce(&mut LoadContext) -> A,
    ) -> Handle<A> {
        let mut context = self.begin_labeled_asset(label);
        let asset = load(&mut context);
        let loaded_asset = context.finish(asset, None);
        self.add_loaded_labeled_asset(loaded_asset)
    }

    pub fn add_labeled_asset<A: Asset>(&mut self, label: String, asset: A) -> Handle<A> {
        self.labeled_asset_scope(label, |_| asset)
    }

    /// Add a [`LoadedAsset`] that is a "labeled sub asset" of the root path of this load context.
    /// This can be used in combination with [`LoadContext::begin_labeled_asset`] to parallelize
    /// sub asset loading.
    pub fn add_loaded_labeled_asset<A: Asset>(
        &mut self,
        loaded_asset: LoadedAsset<A>,
    ) -> Handle<A> {
        let path = loaded_asset.path.as_ref().unwrap().to_owned();
        let loaded_asset: ErasedLoadedAsset = loaded_asset.into();
        debug_assert_eq!(path.without_label(), self.asset_path.without_label());
        let label = path.label().unwrap().to_string();
        let handle = self
            .asset_server
            .get_or_create_path_handle(path, TypeId::of::<A>());
        let returned_handle = handle.clone().typed_debug_checked();
        self.labeled_assets.insert(
            label,
            LabeledAsset {
                asset: loaded_asset,
                handle,
            },
        );
        returned_handle
    }

    pub fn has_labeled_asset(&self, label: &str) -> bool {
        let path = self.asset_path.with_label(label);
        self.asset_server.get_handle_untyped(path).is_some()
    }

    pub fn finish<A: Asset>(self, value: A, meta: Option<Box<dyn AssetMetaDyn>>) -> LoadedAsset<A> {
        LoadedAsset {
            value,
            path: Some(self.asset_path),
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
    pub fn asset_path(&self) -> &AssetPath {
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
            // See `ProcessorGatdReader` for more info
            let meta_bytes = self.asset_server.reader().read_meta_bytes(path).await?;
            let minimal: AssetMetaProcessedInfoMinimal = ron::de::from_bytes(&meta_bytes)
                .map_err(DeserializeMetaError::DeserializeMinimal)?;
            let processed_info = minimal
                .processed_info
                .ok_or(ReadAssetBytesError::MissingAssetHash)?;
            processed_info.full_hash
        } else {
            0
        };
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        self.loader_dependencies
            .insert(AssetPath::new(path.to_owned(), None), hash);
        Ok(bytes)
    }

    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset.
    /// If the current context is a normal [`AssetServer::load`], an actual asset load will be kicked off immediately, which ensures the load happens
    /// as soon as possible.
    /// If the current context is an [`AssetServer::load_direct_async`] (such as in the [`AssetProcessor`](crate::processor::AssetProcessor)),
    /// a load will not be kicked off automatically. It is then the calling context's responsibility to begin a load if necessary.
    pub fn load<'b, A: Asset>(&mut self, path: impl Into<AssetPath<'b>>) -> Handle<A> {
        let path = path.into().to_owned();
        let handle = if self.should_load_dependencies {
            self.asset_server.load(path.clone())
        } else {
            self.asset_server
                .get_or_create_path_handle(path.clone(), TypeId::of::<A>())
                .typed_debug_checked()
        };
        self.dependencies.insert(handle.clone().untyped());
        handle
    }

    pub fn get_label_handle<A: Asset>(&mut self, label: &str) -> Handle<A> {
        let path = self.asset_path.with_label(label);
        let handle = self
            .asset_server
            .get_or_create_path_handle(path.to_owned(), TypeId::of::<A>())
            .typed_debug_checked();
        self.dependencies.insert(handle.clone().untyped());
        handle
    }

    pub async fn load_direct<'b>(
        &mut self,
        path: impl Into<AssetPath<'b>>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let path = path.into();
        let to_error = |e: AssetLoadError| -> LoadDirectError {
            LoadDirectError {
                dependency: path.to_owned(),
                error: e,
            }
        };
        let (meta, loader, mut reader) = self
            .asset_server
            .get_meta_loader_and_reader(&path)
            .await
            .map_err(to_error)?;
        let loaded_asset = self
            .asset_server
            .load_with_meta_loader_and_reader(
                &path,
                meta,
                &*loader,
                &mut *reader,
                false,
                self.populate_hashes,
            )
            .await
            .map_err(to_error)?;
        let info = loaded_asset
            .meta
            .as_ref()
            .and_then(|m| m.processed_info().as_ref());
        let hash = info.map(|i| i.full_hash).unwrap_or(0);
        self.loader_dependencies.insert(path.to_owned(), hash);
        Ok(loaded_asset)
    }
}

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
