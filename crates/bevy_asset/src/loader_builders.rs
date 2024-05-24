//! Implementations of the builder-pattern used for loading dependent assets via
//! [`LoadContext::loader`].

use crate::{
    io::Reader,
    meta::{meta_transform_settings, AssetMetaDyn, MetaTransform, Settings},
    Asset, AssetLoadError, AssetPath, ErasedAssetLoader, ErasedLoadedAsset, Handle, LoadContext,
    LoadDirectError, LoadedAsset, LoadedUntypedAsset,
};
use std::any::TypeId;
use std::sync::Arc;

// Utility type for handling the sources of reader references
enum ReaderRef<'a, 'b> {
    Borrowed(&'a mut Reader<'b>),
    Boxed(Box<Reader<'b>>),
}

impl<'a, 'b> ReaderRef<'a, 'b> {
    pub fn as_mut(&mut self) -> &mut Reader {
        match self {
            ReaderRef::Borrowed(r) => r,
            ReaderRef::Boxed(b) => &mut *b,
        }
    }
}

/// A builder for loading nested assets inside a `LoadContext`.
///
/// # Lifetimes
/// - `ctx`: the lifetime of the associated [`AssetServer`] reference
/// - `builder`: the lifetime of the temporary builder structs
pub struct NestedLoader<'ctx, 'builder> {
    load_context: &'builder mut LoadContext<'ctx>,
    meta_transform: Option<MetaTransform>,
    asset_type_id: Option<TypeId>,
}

impl<'ctx, 'builder> NestedLoader<'ctx, 'builder> {
    pub(crate) fn new(
        load_context: &'builder mut LoadContext<'ctx>,
    ) -> NestedLoader<'ctx, 'builder> {
        NestedLoader {
            load_context,
            meta_transform: None,
            asset_type_id: None,
        }
    }

    fn with_transform(
        mut self,
        transform: impl Fn(&mut dyn AssetMetaDyn) + Send + Sync + 'static,
    ) -> Self {
        if let Some(prev_transform) = self.meta_transform {
            self.meta_transform = Some(Box::new(move |meta| {
                prev_transform(meta);
                transform(meta);
            }));
        } else {
            self.meta_transform = Some(Box::new(transform));
        }
        self
    }

    /// Configure the settings used to load the asset.
    ///
    /// If the settings type `S` does not match the settings expected by `A`'s asset loader, an error will be printed to the log
    /// and the asset load will fail.
    #[must_use]
    pub fn with_settings<S: Settings>(
        self,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Self {
        self.with_transform(move |meta| meta_transform_settings(meta, &settings))
    }

    /// Specify the output asset type.
    #[must_use]
    pub fn with_asset_type<A: Asset>(mut self) -> Self {
        self.asset_type_id = Some(TypeId::of::<A>());
        self
    }

    /// Specify the output asset type.
    #[must_use]
    pub fn with_asset_type_id(mut self, asset_type_id: TypeId) -> Self {
        self.asset_type_id = Some(asset_type_id);
        self
    }

    /// Load assets directly, rather than creating handles.
    #[must_use]
    pub fn direct<'c>(self) -> DirectNestedLoader<'ctx, 'builder, 'c> {
        DirectNestedLoader {
            base: self,
            reader: None,
        }
    }

    /// Load assets without static type information.
    ///
    /// If you need to specify the type of asset, but cannot do it statically,
    /// use `.with_asset_type_id()`.
    #[must_use]
    pub fn untyped(self) -> UntypedNestedLoader<'ctx, 'builder> {
        UntypedNestedLoader { base: self }
    }

    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset.
    /// If the current context is a normal [`AssetServer::load`], an actual asset load will be kicked off immediately, which ensures the load happens
    /// as soon as possible.
    /// "Normal loads" kicked from within a normal Bevy App will generally configure the context to kick off loads immediately.
    /// If the current context is configured to not load dependencies automatically (ex: [`AssetProcessor`](crate::processor::AssetProcessor)),
    /// a load will not be kicked off automatically. It is then the calling context's responsibility to begin a load if necessary.
    pub fn load<'c, A: Asset>(self, path: impl Into<AssetPath<'c>>) -> Handle<A> {
        let path = path.into().to_owned();
        let handle = if self.load_context.should_load_dependencies {
            self.load_context
                .asset_server
                .load_with_meta_transform(path, self.meta_transform, ())
        } else {
            self.load_context
                .asset_server
                .get_or_create_path_handle(path, None)
        };
        self.load_context.dependencies.insert(handle.id().untyped());
        handle
    }
}

/// A builder for loading untyped nested assets inside a [`LoadContext`].
///
/// # Lifetimes
/// - `ctx`: the lifetime of the associated [`AssetServer`] reference
/// - `builder`: the lifetime of the temporary builder structs
pub struct UntypedNestedLoader<'ctx, 'builder> {
    base: NestedLoader<'ctx, 'builder>,
}

impl<'ctx, 'builder> UntypedNestedLoader<'ctx, 'builder> {
    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset without knowing its type.
    pub fn load<'p>(self, path: impl Into<AssetPath<'p>>) -> Handle<LoadedUntypedAsset> {
        let path = path.into().to_owned();
        let handle = if self.base.load_context.should_load_dependencies {
            self.base
                .load_context
                .asset_server
                .load_untyped_with_meta_transform(path, self.base.meta_transform)
        } else {
            self.base
                .load_context
                .asset_server
                .get_or_create_path_handle(path, self.base.meta_transform)
        };
        self.base
            .load_context
            .dependencies
            .insert(handle.id().untyped());
        handle
    }
}

/// A builder for directly loading nested assets inside a `LoadContext`.
///
/// # Lifetimes
/// - `ctx`: the lifetime of the associated [`AssetServer`] reference
/// - `builder`: the lifetime of the temporary builder structs
/// - `reader`: the lifetime of the [`Reader`] reference used to read the asset data
pub struct DirectNestedLoader<'ctx, 'builder, 'reader> {
    base: NestedLoader<'ctx, 'builder>,
    reader: Option<&'builder mut Reader<'reader>>,
}

impl<'ctx: 'reader, 'builder, 'reader> DirectNestedLoader<'ctx, 'builder, 'reader> {
    /// Specify the reader to use to read the asset data.
    #[must_use]
    pub fn with_reader(mut self, reader: &'builder mut Reader<'reader>) -> Self {
        self.reader = Some(reader);
        self
    }

    /// Load the asset without providing static type information.
    ///
    /// If you need to specify the type of asset, but cannot do it statically,
    /// use `.with_asset_type_id()`.
    #[must_use]
    pub fn untyped(self) -> UntypedDirectNestedLoader<'ctx, 'builder, 'reader> {
        UntypedDirectNestedLoader { base: self }
    }

    async fn load_internal(
        self,
        path: &AssetPath<'static>,
    ) -> Result<(Arc<dyn ErasedAssetLoader>, ErasedLoadedAsset), LoadDirectError> {
        let (mut meta, loader, mut reader) = if let Some(reader) = self.reader {
            let loader = if let Some(asset_type_id) = self.base.asset_type_id {
                self.base
                    .load_context
                    .asset_server
                    .get_asset_loader_with_asset_type_id(asset_type_id)
                    .await
                    .map_err(|error| LoadDirectError {
                        dependency: path.clone(),
                        error: error.into(),
                    })?
            } else {
                self.base
                    .load_context
                    .asset_server
                    .get_path_asset_loader(path)
                    .await
                    .map_err(|error| LoadDirectError {
                        dependency: path.clone(),
                        error: error.into(),
                    })?
            };
            let meta = loader.default_meta();
            (meta, loader, ReaderRef::Borrowed(reader))
        } else {
            let (meta, loader, reader) = self
                .base
                .load_context
                .asset_server
                .get_meta_loader_and_reader(path, self.base.asset_type_id)
                .await
                .map_err(|error| LoadDirectError {
                    dependency: path.clone(),
                    error,
                })?;
            (meta, loader, ReaderRef::Boxed(reader))
        };

        if let Some(meta_transform) = self.base.meta_transform {
            meta_transform(&mut *meta);
        }

        let asset = self
            .base
            .load_context
            .load_direct_internal(path.clone(), meta, &*loader, reader.as_mut())
            .await?;
        Ok((loader, asset))
    }

    /// Loads the asset at the given `path` directly. This is an async function that will wait until the asset is fully loaded before
    /// returning. Use this if you need the _value_ of another asset in order to load the current asset. For example, if you are
    /// deriving a new asset from the referenced asset, or you are building a collection of assets. This will add the `path` as a
    /// "load dependency".
    ///
    /// If the current loader is used in a [`Process`] "asset preprocessor", such as a [`LoadTransformAndSave`] preprocessor,
    /// changing a "load dependency" will result in re-processing of the asset.
    ///
    /// [`Process`]: crate::processor::Process
    /// [`LoadTransformAndSave`]: crate::processor::LoadTransformAndSave
    pub async fn load<'p, A: Asset>(
        mut self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<LoadedAsset<A>, LoadDirectError> {
        self.base.asset_type_id = Some(TypeId::of::<A>());
        let path = path.into().into_owned();
        self.load_internal(&path)
            .await
            .and_then(move |(loader, untyped_asset)| {
                untyped_asset.downcast::<A>().map_err(|_| LoadDirectError {
                    dependency: path.clone(),
                    error: AssetLoadError::RequestedHandleTypeMismatch {
                        path,
                        requested: TypeId::of::<A>(),
                        actual_asset_name: loader.asset_type_name(),
                        loader_name: loader.type_name(),
                    },
                })
            })
    }
}

/// A builder for directly loading untyped nested assets inside a `LoadContext`.
///
/// # Lifetimes
/// - `ctx`: the lifetime of the associated [`AssetServer`] reference
/// - `builder`: the lifetime of the temporary builder structs
/// - `reader`: the lifetime of the [`Reader`] reference used to read the asset data
pub struct UntypedDirectNestedLoader<'ctx, 'builder, 'reader> {
    base: DirectNestedLoader<'ctx, 'builder, 'reader>,
}

impl<'ctx: 'reader, 'builder, 'reader> UntypedDirectNestedLoader<'ctx, 'builder, 'reader> {
    /// Loads the asset at the given `path` directly. This is an async function that will wait until the asset is fully loaded before
    /// returning. Use this if you need the _value_ of another asset in order to load the current asset. For example, if you are
    /// deriving a new asset from the referenced asset, or you are building a collection of assets. This will add the `path` as a
    /// "load dependency".
    ///
    /// If the current loader is used in a [`Process`] "asset preprocessor", such as a [`LoadTransformAndSave`] preprocessor,
    /// changing a "load dependency" will result in re-processing of the asset.
    ///
    /// [`Process`]: crate::processor::Process
    /// [`LoadTransformAndSave`]: crate::processor::LoadTransformAndSave
    pub async fn load<'p>(
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let path = path.into().into_owned();
        self.base.load_internal(&path).await.map(|(_, asset)| asset)
    }
}
