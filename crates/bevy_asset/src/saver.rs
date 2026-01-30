use crate::{
    io::{AssetWriterError, MissingAssetSourceError, MissingAssetWriterError, Writer},
    meta::{AssetAction, AssetMeta, AssetMetaDyn, Settings},
    transformer::TransformedAsset,
    Asset, AssetContainer, AssetLoader, AssetPath, AssetServer, ErasedLoadedAsset, Handle,
    LabeledAsset, UntypedHandle,
};
use alloc::{boxed::Box, string::ToString, sync::Arc};
use atomicow::CowArc;
use bevy_ecs::error::BevyError;
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::{any::TypeId, borrow::Borrow, ops::Deref};
use futures_lite::AsyncWriteExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Saves an [`Asset`] of a given [`AssetSaver::Asset`] type. [`AssetSaver::OutputLoader`] will then be used to load the saved asset
/// in the final deployed application. The saver should produce asset bytes in a format that [`AssetSaver::OutputLoader`] can read.
///
/// This trait is generally used in concert with [`AssetWriter`](crate::io::AssetWriter) to write assets as bytes.
///
/// For a version of this trait that can load assets, see [`AssetLoader`].
///
/// Note: This is currently only leveraged by the [`AssetProcessor`](crate::processor::AssetProcessor), and does not provide a
/// suitable interface for general purpose asset persistence. See [github issue #11216](https://github.com/bevyengine/bevy/issues/11216).
///
pub trait AssetSaver: TypePath + Send + Sync + 'static {
    /// The top level [`Asset`] saved by this [`AssetSaver`].
    type Asset: Asset;
    /// The settings type used by this [`AssetSaver`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [`AssetLoader`] used to load this [`Asset`]
    type OutputLoader: AssetLoader;
    /// The type of [error](`std::error::Error`) which could be encountered by this saver.
    type Error: Into<BevyError>;

    /// Saves the given runtime [`Asset`] by writing it to a byte format using `writer`. The passed in `settings` can influence how the
    /// `asset` is saved.
    fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
    ) -> impl ConditionalSendFuture<
        Output = Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error>,
    >;
}

/// A type-erased dynamic variant of [`AssetSaver`] that allows callers to save assets without knowing the actual type of the [`AssetSaver`].
pub trait ErasedAssetSaver: Send + Sync + 'static {
    /// Saves the given runtime [`ErasedLoadedAsset`] by writing it to a byte format using `writer`. The passed in `settings` can influence how the
    /// `asset` is saved.
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), BevyError>>;

    /// The type name of the [`AssetSaver`].
    fn type_name(&self) -> &'static str;
}

impl<S: AssetSaver> ErasedAssetSaver for S {
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), BevyError>> {
        Box::pin(async move {
            let settings = settings
                .downcast_ref::<S::Settings>()
                .expect("AssetLoader settings should match the loader type");
            let saved_asset = SavedAsset::<S::Asset>::from_loaded(asset).unwrap();
            if let Err(err) = self.save(writer, saved_asset, settings).await {
                return Err(err.into());
            }
            Ok(())
        })
    }
    fn type_name(&self) -> &'static str {
        core::any::type_name::<S>()
    }
}

/// An [`Asset`] (and any labeled "sub assets") intended to be saved.
#[derive(Clone)]
pub struct SavedAsset<'a, 'b, A: Asset> {
    value: &'a A,
    labeled_assets: Moo<'b, HashMap<CowArc<'a, str>, LabeledSavedAsset<'a>>>,
}

impl<A: Asset> Deref for SavedAsset<'_, '_, A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, 'b, A: Asset> SavedAsset<'a, 'b, A> {
    fn from_value_and_labeled_saved_assets(
        value: &'a A,
        labeled_saved_assets: &'b HashMap<CowArc<'a, str>, LabeledSavedAsset<'a>>,
    ) -> Self {
        Self {
            value,
            labeled_assets: Moo::Borrowed(labeled_saved_assets),
        }
    }

    fn from_value_and_labeled_assets(
        value: &'a A,
        labeled_assets: &'a HashMap<CowArc<'static, str>, LabeledAsset>,
    ) -> Self {
        Self {
            value,
            labeled_assets: Moo::Owned(
                labeled_assets
                    .iter()
                    .map(|(label, labeled_asset)| {
                        (
                            CowArc::Borrowed(label.borrow()),
                            LabeledSavedAsset::from_labeled_asset(labeled_asset),
                        )
                    })
                    .collect(),
            ),
        }
    }

    /// Creates a new [`SavedAsset`] from `asset` if its internal value matches `A`.
    pub fn from_loaded(asset: &'a ErasedLoadedAsset) -> Option<Self> {
        let value = asset.value.downcast_ref::<A>()?;
        Some(Self::from_value_and_labeled_assets(
            value,
            &asset.labeled_assets,
        ))
    }

    /// Creates a new [`SavedAsset`] from the a [`TransformedAsset`]
    pub fn from_transformed(asset: &'a TransformedAsset<A>) -> Self {
        Self::from_value_and_labeled_assets(&asset.value, &asset.labeled_assets)
    }

    /// Creates a new [`SavedAsset`] holding only the provided value with no labeled assets.
    pub fn from_asset(value: &'a A) -> Self {
        Self {
            value,
            labeled_assets: Moo::Owned(HashMap::default()),
        }
    }

    /// Casts this typed asset into its type-erased form.
    pub fn upcast(self) -> ErasedSavedAsset<'a, 'a>
    where
        'b: 'a,
    {
        ErasedSavedAsset {
            value: self.value,
            labeled_assets: self.labeled_assets,
        }
    }

    /// Retrieves the value of this asset.
    #[inline]
    pub fn get(&self) -> &'a A {
        self.value
    }

    /// Returns the labeled asset, if it exists and matches this type.
    pub fn get_labeled<B: Asset>(&self, label: &str) -> Option<SavedAsset<'a, '_, B>> {
        let labeled = self.labeled_assets.get(label)?;
        labeled.asset.downcast()
    }

    /// Returns the type-erased labeled asset, if it exists and matches this type.
    pub fn get_erased_labeled(&self, label: &str) -> Option<&ErasedSavedAsset<'a, '_>> {
        let labeled = self.labeled_assets.get(label)?;
        Some(&labeled.asset)
    }

    /// Returns the [`UntypedHandle`] of the labeled asset with the provided 'label', if it exists.
    pub fn get_untyped_handle(&self, label: &str) -> Option<UntypedHandle> {
        let labeled = self.labeled_assets.get(label)?;
        Some(labeled.handle.clone())
    }

    /// Returns the [`Handle`] of the labeled asset with the provided 'label', if it exists and is an asset of type `B`
    pub fn get_handle<B: Asset>(&self, label: &str) -> Option<Handle<B>> {
        let labeled = self.labeled_assets.get(label)?;
        if let Ok(handle) = labeled.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }

    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.labeled_assets.keys().map(|s| &**s)
    }
}

#[derive(Clone)]
pub struct ErasedSavedAsset<'a: 'b, 'b> {
    value: &'a dyn AssetContainer,
    labeled_assets: Moo<'b, HashMap<CowArc<'a, str>, LabeledSavedAsset<'a>>>,
}

impl<'a> ErasedSavedAsset<'a, '_> {
    fn from_loaded(asset: &'a ErasedLoadedAsset) -> Self {
        Self {
            value: &*asset.value,
            labeled_assets: Moo::Owned(
                asset
                    .labeled_assets
                    .iter()
                    .map(|(label, asset)| {
                        (
                            CowArc::Borrowed(label.borrow()),
                            LabeledSavedAsset::from_labeled_asset(asset),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

impl<'a> ErasedSavedAsset<'a, '_> {
    /// Attempts to downcast this erased asset into type `A`.
    ///
    /// Returns [`None`] if the asset is the wrong type.
    pub fn downcast<'b, A: Asset>(&'b self) -> Option<SavedAsset<'a, 'b, A>> {
        let value = self.value.downcast_ref::<A>()?;
        Some(SavedAsset::from_value_and_labeled_saved_assets(
            value,
            &self.labeled_assets,
        ))
    }
}

/// Container for a single labeled asset (which also includes its labeled assets, for nested
/// assets).
#[derive(Clone)]
struct LabeledSavedAsset<'a> {
    /// The asset and its labeled assets.
    asset: ErasedSavedAsset<'a, 'a>,
    /// The handle of this labeled asset.
    handle: UntypedHandle,
}

impl<'a> LabeledSavedAsset<'a> {
    /// Creates an instance that corresponds to the same data as [`LabeledAsset`].
    fn from_labeled_asset(asset: &'a LabeledAsset) -> Self {
        Self {
            asset: ErasedSavedAsset::from_loaded(&asset.asset),
            handle: asset.handle.clone(),
        }
    }
}

/// A builder for creating [`SavedAsset`] instances (for use with asset saving).
///
/// This is commonly used in tandem with [`save_using_saver`].
pub struct SavedAssetBuilder<'a> {
    /// The labeled assets for this saved asset.
    labeled_assets: HashMap<CowArc<'a, str>, LabeledSavedAsset<'a>>,
    /// The asset path (with no label) that this saved asset is "tied" to.
    ///
    /// All labeled assets will use this asset path (with their substituted labels). Note labeled
    /// assets **of labeled assets** may not use the same asset path (to represent nested-loaded
    /// assets).
    asset_path: AssetPath<'static>,
    /// The asset server to use for creating handles.
    asset_server: AssetServer,
}

impl<'a> SavedAssetBuilder<'a> {
    /// Creates a new builder for the given `asset_path` and using the `asset_server` to back its
    /// handles.
    pub fn new(asset_server: AssetServer, mut asset_path: AssetPath<'static>) -> Self {
        asset_path.remove_label();
        Self {
            asset_server,
            asset_path,
            labeled_assets: Default::default(),
        }
    }

    /// Adds a labeled asset, creates a handle for it, and returns the handle (for use in creating
    /// an asset).
    ///
    /// This is primarily used when **constructing** a new asset to be saved. Since assets commonly
    /// store handles to their subassets, this function returns a handle that can be stored in your
    /// root asset.
    ///
    /// If you already have a root asset instance (which already contains a subasset handle), use
    /// [`Self::add_labeled_asset_with_existing_handle`] instead.
    #[must_use]
    pub fn add_labeled_asset_with_new_handle<'b: 'a, A: Asset>(
        &mut self,
        label: impl Into<CowArc<'b, str>>,
        asset: SavedAsset<'a, 'a, A>,
    ) -> Handle<A> {
        let label = label.into();
        let handle = Handle::Strong(
            self.asset_server
                .read_infos()
                .handle_providers
                .get(&TypeId::of::<A>())
                .expect("asset type has been initialized")
                .reserve_handle_internal(
                    false,
                    Some(self.asset_path.clone().with_label(label.to_string())),
                    None,
                ),
        );
        self.add_labeled_asset_with_existing_handle(label, asset, handle.clone());
        handle
    }

    /// Adds a labeled asset with a pre-existing handle.
    ///
    /// This is primarily used when attempting to save a (root) asset that you already have an
    /// instance of. Since this root asset instance already must have its fields populated
    /// (including any subasset handles), this function allows you to record the subasset that
    /// should be associated with that handle.
    ///
    /// If you do not have a root asset instance (you're creating one from scratch), use
    /// [`Self::add_labeled_asset_with_new_handle`] instead.
    pub fn add_labeled_asset_with_existing_handle<'b: 'a, A: Asset>(
        &mut self,
        label: impl Into<CowArc<'b, str>>,
        asset: SavedAsset<'a, 'a, A>,
        handle: Handle<A>,
    ) {
        self.add_labeled_asset_with_existing_handle_erased(
            label.into(),
            asset.upcast(),
            handle.untyped(),
        );
    }

    /// Same as [`Self::add_labeled_asset_with_new_handle`], but type-erased to allow for dynamic
    /// types.
    #[must_use]
    pub fn add_labeled_asset_with_new_handle_erased<'b: 'a>(
        &mut self,
        label: impl Into<CowArc<'b, str>>,
        asset: ErasedSavedAsset<'a, 'a>,
    ) -> UntypedHandle {
        let label = label.into();
        let handle = UntypedHandle::Strong(
            self.asset_server
                .read_infos()
                .handle_providers
                .get(&asset.value.type_id())
                .expect("asset type has been initialized")
                .reserve_handle_internal(
                    false,
                    Some(self.asset_path.clone().with_label(label.to_string())),
                    None,
                ),
        );
        self.add_labeled_asset_with_existing_handle_erased(label, asset, handle.clone());
        handle
    }

    /// Same as [`Self::add_labeled_asset_with_existing_handle`], but type-erased to allow for
    /// dynamic types.
    pub fn add_labeled_asset_with_existing_handle_erased<'b: 'a>(
        &mut self,
        label: impl Into<CowArc<'b, str>>,
        asset: ErasedSavedAsset<'a, 'a>,
        handle: UntypedHandle,
    ) {
        // TODO: Check asset and handle have the same type.
        self.labeled_assets
            .insert(label.into(), LabeledSavedAsset { asset, handle });
    }

    /// Creates the final saved asset from this builder.
    pub fn build<'b, A: Asset>(self, asset: &'b A) -> SavedAsset<'b, 'b, A>
    where
        'a: 'b,
    {
        SavedAsset {
            value: asset,
            labeled_assets: Moo::Owned(self.labeled_assets),
        }
    }
}

/// An alternative to [`Cow`] but simplified to just a `T` or `&T`.
///
/// Associated types are **always** considered "invariant" (see
/// <https://doc.rust-lang.org/nomicon/subtyping.html>). Since [`Cow`] uses the [`ToOwned`] trait
/// and its associated type of [`ToOwned::Owned`], this means [`Cow`] types are invariant (which
/// TL;DR means that in some cases Rust is not allowed to shorten lifetimes, causing lifetime
/// errors).
///
/// This type also allows working with any type, not just those that implement [`ToOwned`] - at the
/// cost of losing the ability to mutate the value.
///
/// `Moo` stands for maybe-owned-object.
///
/// [`Cow`]: alloc::borrow::Cow
/// [`ToOwned`]: alloc::borrow::ToOwned
/// [`ToOwned::Owned`]: alloc::borrow::ToOwned::Owned
#[derive(Clone)]
enum Moo<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

impl<T> Deref for Moo<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(t) => t,
            Self::Borrowed(t) => t,
        }
    }
}

/// Saves `asset` to `path` using the provided `saver` and `settings`.
pub async fn save_using_saver<S: AssetSaver>(
    asset_server: AssetServer,
    saver: &S,
    path: &AssetPath<'_>,
    asset: SavedAsset<'_, '_, S::Asset>,
    settings: &S::Settings,
) -> Result<(), SaveAssetError> {
    let source = asset_server.get_source(path.source())?;
    let writer = source.writer()?;

    let mut file_writer = writer.write(path.path()).await?;

    let loader_settings = saver
        .save(&mut file_writer, asset, settings)
        .await
        .map_err(|err| SaveAssetError::SaverError(Arc::new(err.into())))?;

    file_writer.flush().await.map_err(AssetWriterError::Io)?;

    let meta = AssetMeta::<S::OutputLoader, ()>::new(AssetAction::Load {
        loader: S::OutputLoader::type_path().into(),
        settings: loader_settings,
    });

    let meta = AssetMetaDyn::serialize(&meta);
    writer.write_meta_bytes(path.path(), &meta).await?;

    Ok(())
}

/// An error occurring when saving an asset.
#[derive(Error, Debug)]
pub enum SaveAssetError {
    #[error(transparent)]
    MissingSource(#[from] MissingAssetSourceError),
    #[error(transparent)]
    MissingWriter(#[from] MissingAssetWriterError),
    #[error(transparent)]
    WriterError(#[from] AssetWriterError),
    #[error("Failed to save asset due to error from saver: {0}")]
    SaverError(Arc<BevyError>),
}

#[cfg(test)]
pub(crate) mod tests {
    use alloc::{string::ToString, vec, vec::Vec};
    use bevy_reflect::TypePath;
    use bevy_tasks::block_on;
    use futures_lite::AsyncWriteExt;
    use ron::ser::PrettyConfig;

    use crate::{
        saver::{save_using_saver, AssetSaver, SavedAsset, SavedAssetBuilder},
        tests::{create_app, run_app_until, CoolText, CoolTextLoader, CoolTextRon, SubText},
        AssetApp, AssetServer, Assets,
    };

    fn new_subtext(text: &str) -> SubText {
        SubText {
            text: text.to_string(),
        }
    }

    #[derive(TypePath)]
    pub struct CoolTextSaver;

    impl AssetSaver for CoolTextSaver {
        type Asset = CoolText;
        type Settings = ();
        type OutputLoader = CoolTextLoader;
        type Error = std::io::Error;

        async fn save(
            &self,
            writer: &mut crate::io::Writer,
            asset: SavedAsset<'_, '_, Self::Asset>,
            _: &Self::Settings,
        ) -> Result<(), Self::Error> {
            let ron = CoolTextRon {
                text: asset.text.clone(),
                sub_texts: asset
                    .iter_labels()
                    .map(|label| asset.get_labeled::<SubText>(label).unwrap().text.clone())
                    .collect(),
                dependencies: asset
                    .dependencies
                    .iter()
                    .map(|handle| handle.path().unwrap().path())
                    .map(|path| path.to_str().unwrap().to_string())
                    .collect(),
                // NOTE: We can't handle embedded dependencies in any way, since we need to write to
                // another file to do so.
                embedded_dependencies: vec![],
            };
            let ron = ron::ser::to_string_pretty(&ron, PrettyConfig::new().new_line("\n")).unwrap();
            writer.write_all(ron.as_bytes()).await?;
            Ok(())
        }
    }

    #[test]
    fn builds_saved_asset_for_new_asset() {
        let mut app = create_app().0;

        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .register_asset_loader(CoolTextLoader);

        // Update a few times before saving to show that assets can be entirely created from
        // scratch.
        app.update();
        app.update();
        app.update();

        let hiya_subasset = new_subtext("hiya");
        let goodbye_subasset = new_subtext("goodbye");
        let idk_subasset = new_subtext("idk");

        let asset_server = app.world().resource::<AssetServer>().clone();
        let mut saved_asset_builder =
            SavedAssetBuilder::new(asset_server.clone(), "some/target/path.cool.ron".into());
        let hiya_handle = saved_asset_builder
            .add_labeled_asset_with_new_handle("hiya", SavedAsset::from_asset(&hiya_subasset));
        let goodbye_handle = saved_asset_builder.add_labeled_asset_with_new_handle(
            "goodbye",
            SavedAsset::from_asset(&goodbye_subasset),
        );
        let idk_handle = saved_asset_builder
            .add_labeled_asset_with_new_handle("idk", SavedAsset::from_asset(&idk_subasset));

        let main_asset = CoolText {
            text: "wassup".into(),
            sub_texts: vec![hiya_handle, goodbye_handle, idk_handle],
            ..Default::default()
        };

        let saved_asset = saved_asset_builder.build(&main_asset);
        let mut asset_labels = saved_asset
            .labeled_assets
            .keys()
            .map(|label| label.as_ref().to_string())
            .collect::<Vec<_>>();
        asset_labels.sort();
        assert_eq!(asset_labels, &["goodbye", "hiya", "idk"]);

        {
            let asset_server = asset_server.clone();
            block_on(async move {
                save_using_saver(
                    asset_server,
                    &CoolTextSaver,
                    &"some/target/path.cool.ron".into(),
                    saved_asset,
                    &(),
                )
                .await
            })
            .unwrap();
        }

        let readback = asset_server.load("some/target/path.cool.ron");
        run_app_until(&mut app, |_| {
            asset_server.is_loaded(&readback).then_some(())
        });

        let cool_text = app
            .world()
            .resource::<Assets<CoolText>>()
            .get(&readback)
            .unwrap();

        let subtexts = app.world().resource::<Assets<SubText>>();
        let mut asset_labels = cool_text
            .sub_texts
            .iter()
            .map(|handle| subtexts.get(handle).unwrap().text.clone())
            .collect::<Vec<_>>();
        asset_labels.sort();
        assert_eq!(asset_labels, &["goodbye", "hiya", "idk"]);
    }

    #[test]
    fn builds_saved_asset_for_existing_asset() {
        let (mut app, _) = create_app();

        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .register_asset_loader(CoolTextLoader);

        let mut subtexts = app.world_mut().resource_mut::<Assets<SubText>>();
        let hiya_handle = subtexts.add(new_subtext("hiya"));
        let goodbye_handle = subtexts.add(new_subtext("goodbye"));
        let idk_handle = subtexts.add(new_subtext("idk"));

        let mut cool_texts = app.world_mut().resource_mut::<Assets<CoolText>>();
        let cool_text_handle = cool_texts.add(CoolText {
            text: "wassup".into(),
            sub_texts: vec![
                hiya_handle.clone(),
                goodbye_handle.clone(),
                idk_handle.clone(),
            ],
            ..Default::default()
        });

        let subtexts = app.world().resource::<Assets<SubText>>();
        let cool_texts = app.world().resource::<Assets<CoolText>>();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let mut saved_asset_builder =
            SavedAssetBuilder::new(asset_server.clone(), "some/target/path.cool.ron".into());
        saved_asset_builder.add_labeled_asset_with_existing_handle(
            "hiya",
            SavedAsset::from_asset(subtexts.get(&hiya_handle).unwrap()),
            hiya_handle,
        );
        saved_asset_builder.add_labeled_asset_with_existing_handle(
            "goodbye",
            SavedAsset::from_asset(subtexts.get(&goodbye_handle).unwrap()),
            goodbye_handle,
        );
        saved_asset_builder.add_labeled_asset_with_existing_handle(
            "idk",
            SavedAsset::from_asset(subtexts.get(&idk_handle).unwrap()),
            idk_handle,
        );

        let saved_asset = saved_asset_builder.build(cool_texts.get(&cool_text_handle).unwrap());
        let mut asset_labels = saved_asset
            .labeled_assets
            .keys()
            .map(|label| label.as_ref().to_string())
            .collect::<Vec<_>>();
        asset_labels.sort();
        assert_eq!(asset_labels, &["goodbye", "hiya", "idk"]);

        // While this example is supported, it is **not** recommended. This currently blocks the
        // entire world from updating. A slow write could cause visible stutters. However we do this
        // here to show it's possible to use assets directly out of the Assets resources.
        {
            let asset_server = asset_server.clone();
            block_on(async move {
                save_using_saver(
                    asset_server,
                    &CoolTextSaver,
                    &"some/target/path.cool.ron".into(),
                    saved_asset,
                    &(),
                )
                .await
            })
            .unwrap();
        }

        let readback = asset_server.load("some/target/path.cool.ron");
        run_app_until(&mut app, |_| {
            asset_server.is_loaded(&readback).then_some(())
        });

        let cool_text = app
            .world()
            .resource::<Assets<CoolText>>()
            .get(&readback)
            .unwrap();

        let subtexts = app.world().resource::<Assets<SubText>>();
        let mut asset_labels = cool_text
            .sub_texts
            .iter()
            .map(|handle| subtexts.get(handle).unwrap().text.clone())
            .collect::<Vec<_>>();
        asset_labels.sort();
        assert_eq!(asset_labels, &["goodbye", "hiya", "idk"]);
    }
}
