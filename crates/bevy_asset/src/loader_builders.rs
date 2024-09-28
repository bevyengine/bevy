//! Implementations of the builder-pattern used for loading dependent assets via
//! [`LoadContext::loader`].

use crate::{
    io::Reader,
    meta::{meta_transform_settings, AssetMetaDyn, MetaTransform, Settings},
    Asset, AssetLoadError, AssetPath, ErasedAssetLoader, ErasedLoadedAsset, Handle, LoadContext,
    LoadDirectError, LoadedAsset, LoadedUntypedAsset, UntypedHandle,
};
use alloc::sync::Arc;
use core::any::TypeId;

// Utility type for handling the sources of reader references
enum ReaderRef<'a> {
    Borrowed(&'a mut dyn Reader),
    Boxed(Box<dyn Reader + 'a>),
}

impl ReaderRef<'_> {
    pub fn as_mut(&mut self) -> &mut dyn Reader {
        match self {
            ReaderRef::Borrowed(r) => &mut **r,
            ReaderRef::Boxed(b) => &mut **b,
        }
    }
}

/// A builder for loading nested assets inside a `LoadContext`.
///
/// # Lifetimes
/// - `ctx`: the lifetime of the associated [`AssetServer`](crate::AssetServer) reference
/// - `builder`: the lifetime of the temporary builder structs
pub struct NestedLoader<'ctx, 'builder, T, M> {
    load_context: &'builder mut LoadContext<'ctx>,
    meta_transform: Option<MetaTransform>,
    typing: T,
    mode: M,
}

mod sealed {
    pub trait Typing {}

    pub trait Mode {}
}

pub struct Untyped {
    _priv: (),
}

impl sealed::Typing for Untyped {}

pub struct Typed {
    asset_type_id: TypeId,
}

impl sealed::Typing for Typed {}

pub struct Indirect {
    _priv: (),
}

impl sealed::Mode for Indirect {}

pub struct Direct<'builder, 'reader> {
    reader: Option<&'builder mut (dyn Reader + 'reader)>,
}

impl sealed::Mode for Direct<'_, '_> {}

// common to all states

impl<'ctx, 'builder> NestedLoader<'ctx, 'builder, Untyped, Indirect> {
    pub(crate) fn new(load_context: &'builder mut LoadContext<'ctx>) -> Self {
        NestedLoader {
            load_context,
            meta_transform: None,
            typing: Untyped { _priv: () },
            mode: Indirect { _priv: () },
        }
    }
}

impl<'ctx, 'builder, T: sealed::Typing, M: sealed::Mode> NestedLoader<'ctx, 'builder, T, M> {
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
}

// convert between typed and untyped

impl<'ctx, 'builder, M: sealed::Mode> NestedLoader<'ctx, 'builder, Untyped, M> {
    /// Specify the output asset type.
    #[must_use]
    pub fn with_asset_type<A: Asset>(self) -> NestedLoader<'ctx, 'builder, Typed, M> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: Typed {
                asset_type_id: TypeId::of::<A>(),
            },
            mode: self.mode,
        }
    }

    /// Specify the output asset type.
    #[must_use]
    pub fn with_asset_type_id(
        self,
        asset_type_id: TypeId,
    ) -> NestedLoader<'ctx, 'builder, Typed, M> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: Typed { asset_type_id },
            mode: self.mode,
        }
    }
}

impl<'ctx, 'builder, M: sealed::Mode> NestedLoader<'ctx, 'builder, Typed, M> {
    // todo docs
    #[must_use]
    pub fn untyped(self) -> NestedLoader<'ctx, 'builder, Untyped, M> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: Untyped { _priv: () },
            mode: self.mode,
        }
    }
}

// convert between direct and indirect

impl<'ctx, 'builder, T: sealed::Typing> NestedLoader<'ctx, 'builder, T, Indirect> {
    /// Load assets directly, rather than creating handles.
    #[must_use]
    pub fn direct<'c>(self) -> NestedLoader<'ctx, 'builder, T, Direct<'builder, 'c>> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: self.typing,
            mode: Direct { reader: None },
        }
    }
}

impl<'ctx, 'builder, T: sealed::Typing> NestedLoader<'ctx, 'builder, T, Direct<'_, '_>> {
    // todo docs
    pub fn indirect<'c>(self) -> NestedLoader<'ctx, 'builder, T, Indirect> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: self.typing,
            mode: Indirect { _priv: () },
        }
    }
}

// indirect loading logic

impl NestedLoader<'_, '_, Untyped, Indirect> {
    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset.
    /// If the current context is a normal [`AssetServer::load`](crate::AssetServer::load), an actual asset
    /// load will be kicked off immediately, which ensures the load happens as soon as possible.
    /// "Normal loads" kicked from within a normal Bevy App will generally configure the context to kick off
    /// loads immediately.
    /// If the current context is configured to not load dependencies automatically
    /// (ex: [`AssetProcessor`](crate::processor::AssetProcessor)),
    /// a load will not be kicked off automatically. It is then the calling context's responsibility to begin
    /// a load if necessary.
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

    /// Retrieves a handle for the asset at the given path and adds that path as a dependency of the asset without knowing its type.
    pub fn load_untyped<'p>(self, path: impl Into<AssetPath<'p>>) -> Handle<LoadedUntypedAsset> {
        let path = path.into().to_owned();
        let handle = if self.load_context.should_load_dependencies {
            self.load_context
                .asset_server
                .load_untyped_with_meta_transform(path, self.meta_transform)
        } else {
            self.load_context
                .asset_server
                .get_or_create_path_handle(path, self.meta_transform)
        };
        self.load_context.dependencies.insert(handle.id().untyped());
        handle
    }
}

impl NestedLoader<'_, '_, Typed, Indirect> {
    pub fn load<'p>(self, path: impl Into<AssetPath<'p>>) -> UntypedHandle {
        todo!()
    }
}

// direct loading logic

impl<'builder, 'reader, T> NestedLoader<'_, '_, T, Direct<'builder, 'reader>> {
    /// Specify the reader to use to read the asset data.
    #[must_use]
    pub fn with_reader(mut self, reader: &'builder mut (dyn Reader + 'reader)) -> Self {
        self.mode.reader = Some(reader);
        self
    }

    async fn load_internal(
        self,
        path: &AssetPath<'static>,
        asset_type_id: Option<TypeId>,
    ) -> Result<(Arc<dyn ErasedAssetLoader>, ErasedLoadedAsset), LoadDirectError> {
        let (mut meta, loader, mut reader) = if let Some(reader) = self.mode.reader {
            let loader = if let Some(asset_type_id) = asset_type_id {
                self.load_context
                    .asset_server
                    .get_asset_loader_with_asset_type_id(asset_type_id)
                    .await
                    .map_err(|error| LoadDirectError {
                        dependency: path.clone(),
                        error: error.into(),
                    })?
            } else {
                self.load_context
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
                .load_context
                .asset_server
                .get_meta_loader_and_reader(path, asset_type_id)
                .await
                .map_err(|error| LoadDirectError {
                    dependency: path.clone(),
                    error,
                })?;
            (meta, loader, ReaderRef::Boxed(reader))
        };

        if let Some(meta_transform) = self.meta_transform {
            meta_transform(&mut *meta);
        }

        let asset = self
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
    pub async fn load_untyped<'p>(
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let path = path.into().into_owned();
        self.load_internal(&path, None)
            .await
            .map(|(_, asset)| asset)
    }
}

impl NestedLoader<'_, '_, Untyped, Direct<'_, '_>> {
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
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<LoadedAsset<A>, LoadDirectError> {
        let path = path.into().into_owned();
        self.load_internal(&path, Some(TypeId::of::<A>()))
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

impl NestedLoader<'_, '_, Typed, Direct<'_, '_>> {
    // todo docs
    pub async fn load<'p>(
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let path = path.into().into_owned();
        let asset_type_id = Some(self.typing.asset_type_id);
        self.load_internal(&path, asset_type_id)
            .await
            .map(|(_, asset)| asset)
    }
}
