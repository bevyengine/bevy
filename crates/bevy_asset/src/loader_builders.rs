//! Implementations of the builder-pattern used for loading dependent assets via
//! [`LoadContext::loader`].

use crate::{
    io::Reader,
    meta::{meta_transform_settings, AssetMetaDyn, MetaTransform, Settings},
    Asset, AssetLoadError, AssetPath, ErasedAssetLoader, ErasedLoadedAsset, Handle, LoadContext,
    LoadDirectError, LoadedAsset, LoadedUntypedAsset, UntypedHandle,
};
use alloc::{borrow::ToOwned, boxed::Box, sync::Arc};
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

/// A builder for loading nested assets inside a [`LoadContext`].
///
/// # Loader state
///
/// The type parameters `T` and `M` determine how this will load assets:
/// - `T`: the typing of this loader. How do we know what type of asset to load?
///
///   See [`StaticTyped`] (the default), [`DynamicTyped`], and [`UnknownTyped`].
///
/// - `M`: the load mode. Do we want to load this asset right now (in which case
///   you will have to `await` the operation), or do we just want a [`Handle`],
///   and leave the actual asset loading to later?
///
///   See [`Deferred`] (the default) and [`Immediate`].
///
/// When configuring this builder, you can freely switch between these modes
/// via functions like [`deferred`] and [`immediate`].
///
/// ## Typing
///
/// To inform the loader of what type of asset to load:
/// - in [`StaticTyped`]: statically providing a type parameter `A: Asset` to
///   [`load`].
///
///   This is the simplest way to get a [`Handle<A>`] to the loaded asset, as
///   long as you know the type of `A` at compile time.
///
/// - in [`DynamicTyped`]: providing the [`TypeId`] of the asset at runtime.
///
///   If you know the type ID of the asset at runtime, but not at compile time,
///   use [`with_dynamic_type`] followed by [`load`] to start loading an asset
///   of that type. This lets you get an [`UntypedHandle`] (via [`Deferred`]),
///   or a [`ErasedLoadedAsset`] (via [`Immediate`]).
///
/// - in [`UnknownTyped`]: loading either a type-erased version of the asset
///   ([`ErasedLoadedAsset`]), or a handle *to a handle* of the actual asset
///   ([`LoadedUntypedAsset`]).
///
///   If you have no idea what type of asset you will be loading (not even at
///   runtime with a [`TypeId`]), use this.
///
/// ## Load mode
///
/// To inform the loader how you want to load the asset:
/// - in [`Deferred`]: when you request to load the asset, you get a [`Handle`]
///   for it, but the actual loading won't be completed until later.
///
///   Use this if you only need a [`Handle`] or [`UntypedHandle`].
///
/// - in [`Immediate`]: the load request will load the asset right then and
///   there, waiting until the asset is fully loaded and giving you access to
///   it.
///
///   Note that this requires you to `await` a future, so you must be in an
///   async context to use direct loading. In an asset loader, you will be in
///   an async context.
///
///   Use this if you need the *value* of another asset in order to load the
///   current asset. For example, if you are deriving a new asset from the
///   referenced asset, or you are building a collection of assets. This will
///   add the path of the asset as a "load dependency".
///
///   If the current loader is used in a [`Process`] "asset preprocessor",
///   such as a [`LoadTransformAndSave`] preprocessor, changing a "load
///   dependency" will result in re-processing of the asset.
///
/// # Load kickoff
///
/// If the current context is a normal [`AssetServer::load`], an actual asset
/// load will be kicked off immediately, which ensures the load happens as soon
/// as possible. "Normal loads" kicked from within a normal Bevy App will
/// generally configure the context to kick off loads immediately.
///
/// If the current context is configured to not load dependencies automatically
/// (ex: [`AssetProcessor`]), a load will not be kicked off automatically. It is
/// then the calling context's responsibility to begin a load if necessary.
///
/// # Lifetimes
///
/// - `ctx`: the lifetime of the associated [`AssetServer`](crate::AssetServer) reference
/// - `builder`: the lifetime of the temporary builder structs
///
/// [`deferred`]: Self::deferred
/// [`immediate`]: Self::immediate
/// [`load`]: Self::load
/// [`with_dynamic_type`]: Self::with_dynamic_type
/// [`AssetServer::load`]: crate::AssetServer::load
/// [`AssetProcessor`]: crate::processor::AssetProcessor
/// [`Process`]: crate::processor::Process
/// [`LoadTransformAndSave`]: crate::processor::LoadTransformAndSave
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

/// [`NestedLoader`] will be provided the type of asset as a type parameter on
/// [`load`].
///
/// [`load`]: NestedLoader::load
pub struct StaticTyped(());

impl sealed::Typing for StaticTyped {}

/// [`NestedLoader`] has been configured with info on what type of asset to load
/// at runtime.
pub struct DynamicTyped {
    asset_type_id: TypeId,
}

impl sealed::Typing for DynamicTyped {}

/// [`NestedLoader`] does not know what type of asset it will be loading.
pub struct UnknownTyped(());

impl sealed::Typing for UnknownTyped {}

/// [`NestedLoader`] will create and return asset handles immediately, but only
/// actually load the asset later.
pub struct Deferred(());

impl sealed::Mode for Deferred {}

/// [`NestedLoader`] will immediately load an asset when requested.
pub struct Immediate<'builder, 'reader> {
    reader: Option<&'builder mut (dyn Reader + 'reader)>,
}

impl sealed::Mode for Immediate<'_, '_> {}

// common to all states

impl<'ctx, 'builder> NestedLoader<'ctx, 'builder, StaticTyped, Deferred> {
    pub(crate) fn new(load_context: &'builder mut LoadContext<'ctx>) -> Self {
        NestedLoader {
            load_context,
            meta_transform: None,
            typing: StaticTyped(()),
            mode: Deferred(()),
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

    // convert between `T`s

    /// When [`load`]ing, you must pass in the asset type as a type parameter
    /// statically.
    ///
    /// If you don't know the type statically (at compile time), consider
    /// [`with_dynamic_type`] or [`with_unknown_type`].
    ///
    /// [`load`]: Self::load
    /// [`with_dynamic_type`]: Self::with_dynamic_type
    /// [`with_unknown_type`]: Self::with_unknown_type
    #[must_use]
    pub fn with_static_type(self) -> NestedLoader<'ctx, 'builder, StaticTyped, M> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: StaticTyped(()),
            mode: self.mode,
        }
    }

    /// When [`load`]ing, the loader will attempt to load an asset with the
    /// given [`TypeId`].
    ///
    /// [`load`]: Self::load
    #[must_use]
    pub fn with_dynamic_type(
        self,
        asset_type_id: TypeId,
    ) -> NestedLoader<'ctx, 'builder, DynamicTyped, M> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: DynamicTyped { asset_type_id },
            mode: self.mode,
        }
    }

    /// When [`load`]ing, we will infer what type of asset to load from
    /// metadata.
    ///
    /// [`load`]: Self::load
    #[must_use]
    pub fn with_unknown_type(self) -> NestedLoader<'ctx, 'builder, UnknownTyped, M> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: UnknownTyped(()),
            mode: self.mode,
        }
    }

    // convert between `M`s

    /// When [`load`]ing, create only asset handles, rather than returning the
    /// actual asset.
    ///
    /// [`load`]: Self::load
    pub fn deferred(self) -> NestedLoader<'ctx, 'builder, T, Deferred> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: self.typing,
            mode: Deferred(()),
        }
    }

    /// The [`load`] call itself will load an asset, rather than scheduling the
    /// loading to happen later.
    ///
    /// This gives you access to the loaded asset, but requires you to be in an
    /// async context, and be able to `await` the resulting future.
    ///
    /// [`load`]: Self::load
    #[must_use]
    pub fn immediate<'c>(self) -> NestedLoader<'ctx, 'builder, T, Immediate<'builder, 'c>> {
        NestedLoader {
            load_context: self.load_context,
            meta_transform: self.meta_transform,
            typing: self.typing,
            mode: Immediate { reader: None },
        }
    }
}

// deferred loading logic

impl NestedLoader<'_, '_, StaticTyped, Deferred> {
    /// Retrieves a handle for the asset at the given path and adds that path as
    /// a dependency of this asset.
    ///
    /// This requires you to know the type of asset statically.
    /// - If you have runtime info for what type of asset you're loading (e.g. a
    ///   [`TypeId`]), use [`with_dynamic_type`].
    /// - If you do not know at all what type of asset you're loading, use
    ///   [`with_unknown_type`].
    ///
    /// [`with_dynamic_type`]: Self::with_dynamic_type
    /// [`with_unknown_type`]: Self::with_unknown_type
    pub fn load<'c, A: Asset>(self, path: impl Into<AssetPath<'c>>) -> Handle<A> {
        let path = path.into().to_owned();
        let handle = if self.load_context.should_load_dependencies {
            self.load_context.asset_server.load_with_meta_transform(
                path,
                self.meta_transform,
                (),
                true,
            )
        } else {
            self.load_context
                .asset_server
                .get_or_create_path_handle(path, self.meta_transform)
        };
        self.load_context.dependencies.insert(handle.id().untyped());
        handle
    }
}

impl NestedLoader<'_, '_, DynamicTyped, Deferred> {
    /// Retrieves a handle for the asset at the given path and adds that path as
    /// a dependency of this asset.
    ///
    /// This requires you to pass in the asset type ID into
    /// [`with_dynamic_type`].
    ///
    /// [`with_dynamic_type`]: Self::with_dynamic_type
    pub fn load<'p>(self, path: impl Into<AssetPath<'p>>) -> UntypedHandle {
        let path = path.into().to_owned();
        let handle = if self.load_context.should_load_dependencies {
            self.load_context
                .asset_server
                .load_erased_with_meta_transform(
                    path,
                    self.typing.asset_type_id,
                    self.meta_transform,
                    (),
                )
        } else {
            self.load_context
                .asset_server
                .get_or_create_path_handle_erased(
                    path,
                    self.typing.asset_type_id,
                    self.meta_transform,
                )
        };
        self.load_context.dependencies.insert(handle.id());
        handle
    }
}

impl NestedLoader<'_, '_, UnknownTyped, Deferred> {
    /// Retrieves a handle for the asset at the given path and adds that path as
    /// a dependency of this asset.
    ///
    /// This will infer the asset type from metadata.
    pub fn load<'p>(self, path: impl Into<AssetPath<'p>>) -> Handle<LoadedUntypedAsset> {
        let path = path.into().to_owned();
        let handle = if self.load_context.should_load_dependencies {
            self.load_context
                .asset_server
                .load_unknown_type_with_meta_transform(path, self.meta_transform)
        } else {
            self.load_context
                .asset_server
                .get_or_create_path_handle(path, self.meta_transform)
        };
        self.load_context.dependencies.insert(handle.id().untyped());
        handle
    }
}

// immediate loading logic

impl<'builder, 'reader, T> NestedLoader<'_, '_, T, Immediate<'builder, 'reader>> {
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
        if path.label().is_some() {
            return Err(LoadDirectError::RequestedSubasset(path.clone()));
        }
        let (mut meta, loader, mut reader) = if let Some(reader) = self.mode.reader {
            let loader = if let Some(asset_type_id) = asset_type_id {
                self.load_context
                    .asset_server
                    .get_asset_loader_with_asset_type_id(asset_type_id)
                    .await
                    .map_err(|error| LoadDirectError::LoadError {
                        dependency: path.clone(),
                        error: error.into(),
                    })?
            } else {
                self.load_context
                    .asset_server
                    .get_path_asset_loader(path)
                    .await
                    .map_err(|error| LoadDirectError::LoadError {
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
                .map_err(|error| LoadDirectError::LoadError {
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
            .load_direct_internal(path.clone(), meta.as_ref(), &*loader, reader.as_mut())
            .await?;
        Ok((loader, asset))
    }
}

impl NestedLoader<'_, '_, StaticTyped, Immediate<'_, '_>> {
    /// Attempts to load the asset at the given `path` immediately.
    ///
    /// This requires you to know the type of asset statically.
    /// - If you have runtime info for what type of asset you're loading (e.g. a
    ///   [`TypeId`]), use [`with_dynamic_type`].
    /// - If you do not know at all what type of asset you're loading, use
    ///   [`with_unknown_type`].
    ///
    /// [`with_dynamic_type`]: Self::with_dynamic_type
    /// [`with_unknown_type`]: Self::with_unknown_type
    pub async fn load<'p, A: Asset>(
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<LoadedAsset<A>, LoadDirectError> {
        let path = path.into().into_owned();
        self.load_internal(&path, Some(TypeId::of::<A>()))
            .await
            .and_then(move |(loader, untyped_asset)| {
                untyped_asset
                    .downcast::<A>()
                    .map_err(|_| LoadDirectError::LoadError {
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

impl NestedLoader<'_, '_, DynamicTyped, Immediate<'_, '_>> {
    /// Attempts to load the asset at the given `path` immediately.
    ///
    /// This requires you to pass in the asset type ID into
    /// [`with_dynamic_type`].
    ///
    /// [`with_dynamic_type`]: Self::with_dynamic_type
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

impl NestedLoader<'_, '_, UnknownTyped, Immediate<'_, '_>> {
    /// Attempts to load the asset at the given `path` immediately.
    ///
    /// This will infer the asset type from metadata.
    pub async fn load<'p>(
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<ErasedLoadedAsset, LoadDirectError> {
        let path = path.into().into_owned();
        self.load_internal(&path, None)
            .await
            .map(|(_, asset)| asset)
    }
}
