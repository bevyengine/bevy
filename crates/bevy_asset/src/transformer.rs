use crate::{
    meta::Settings, Asset, AssetId, ErasedLoadedAsset, Handle, LabeledAsset, UntypedAssetId,
    UntypedHandle,
};
use alloc::{boxed::Box, vec::Vec};
use atomicow::CowArc;
use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_reflect::TypePath;
use bevy_tasks::ConditionalSendFuture;
use core::{
    borrow::Borrow,
    convert::Infallible,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use serde::{Deserialize, Serialize};

/// Transforms an [`Asset`] of a given [`AssetTransformer::AssetInput`] type to an [`Asset`] of [`AssetTransformer::AssetOutput`] type.
///
/// This trait is commonly used in association with [`LoadTransformAndSave`](crate::processor::LoadTransformAndSave) to accomplish common asset pipeline workflows.
pub trait AssetTransformer: TypePath + Send + Sync + 'static {
    /// The [`Asset`] type which this [`AssetTransformer`] takes as and input.
    type AssetInput: Asset;
    /// The [`Asset`] type which this [`AssetTransformer`] outputs.
    type AssetOutput: Asset;
    /// The settings type used by this [`AssetTransformer`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [error](`std::error::Error`) which could be encountered by this transformer.
    type Error: Into<Box<dyn core::error::Error + Send + Sync + 'static>>;

    /// Transforms the given [`TransformedAsset`] to [`AssetTransformer::AssetOutput`].
    /// The [`TransformedAsset`]'s `labeled_assets` can be altered to add new Labeled Sub-Assets
    /// The passed in `settings` can influence how the `asset` is transformed
    fn transform<'a>(
        &'a self,
        asset: TransformedAsset<Self::AssetInput>,
        settings: &'a Self::Settings,
    ) -> impl ConditionalSendFuture<Output = Result<TransformedAsset<Self::AssetOutput>, Self::Error>>;
}

/// An [`Asset`] (and any "sub assets") intended to be transformed
pub struct TransformedAsset<A: Asset> {
    pub(crate) value: A,
    pub(crate) labeled_assets: Vec<LabeledAsset>,
    pub(crate) label_to_asset_index: HashMap<CowArc<'static, str>, usize>,
    /// The mapping from a subasset asset IDs to their index in [`Self::labeled_assets`].
    ///
    /// This is entirely redundant with [`Self::labeled_assets`], but it allows looking up the
    /// labeled asset by its asset ID.
    pub(crate) asset_id_to_asset_index: HashMap<UntypedAssetId, usize>,
}

impl<A: Asset> Deref for TransformedAsset<A> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<A: Asset> DerefMut for TransformedAsset<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<A: Asset> TransformedAsset<A> {
    /// Creates a new [`TransformedAsset`] from `asset` if its internal value matches `A`.
    pub fn from_loaded(asset: ErasedLoadedAsset) -> Option<Self> {
        if let Ok(value) = asset.value.downcast::<A>() {
            return Some(TransformedAsset {
                value: *value,
                labeled_assets: asset.labeled_assets,
                label_to_asset_index: asset.label_to_asset_index,
                asset_id_to_asset_index: asset.asset_id_to_asset_index,
            });
        }
        None
    }

    /// Creates a new [`TransformedAsset`] from `asset`, transferring the `labeled_assets` from this [`TransformedAsset`] to the new one
    pub fn replace_asset<B: Asset>(self, asset: B) -> TransformedAsset<B> {
        TransformedAsset {
            value: asset,
            labeled_assets: self.labeled_assets,
            label_to_asset_index: self.label_to_asset_index,
            asset_id_to_asset_index: self.asset_id_to_asset_index,
        }
    }

    /// Takes the labeled assets from `labeled_source` and places them in this [`TransformedAsset`]
    pub fn take_labeled_assets<B: Asset>(&mut self, labeled_source: TransformedAsset<B>) {
        self.labeled_assets = labeled_source.labeled_assets;
        self.label_to_asset_index = labeled_source.label_to_asset_index;
        self.asset_id_to_asset_index = labeled_source.asset_id_to_asset_index;
    }

    /// Retrieves the value of this asset.
    #[inline]
    pub fn get(&self) -> &A {
        &self.value
    }

    /// Mutably retrieves the value of this asset.
    #[inline]
    pub fn get_mut(&mut self) -> &mut A {
        &mut self.value
    }

    /// Returns the labeled asset, if it exists and matches this type.
    pub fn get_labeled<B: Asset, Q>(&mut self, label: &Q) -> Option<TransformedSubAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &mut self.labeled_assets[*index];
        let value = labeled.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut labeled.asset.labeled_assets,
            label_to_asset_index: &mut labeled.asset.label_to_asset_index,
            asset_id_to_asset_index: &mut labeled.asset.asset_id_to_asset_index,
        })
    }

    /// Returns the type-erased labeled asset, if it exists.
    pub fn get_erased_labeled<Q>(&self, label: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &self.labeled_assets[*index];
        Some(&labeled.asset)
    }

    /// Returns the labeled asset given its asset ID if it exists and matches the type.
    ///
    /// This can be used to get the asset from its handle since `&Handle` implements
    /// [`Into<AssetId<B>>`].
    pub fn get_labeled_by_id<B: Asset, Q>(
        &mut self,
        id: impl Into<AssetId<B>>,
    ) -> Option<TransformedSubAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.asset_id_to_asset_index.get(&id.into().untyped())?;
        let labeled = &mut self.labeled_assets[*index];
        let value = labeled.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut labeled.asset.labeled_assets,
            label_to_asset_index: &mut labeled.asset.label_to_asset_index,
            asset_id_to_asset_index: &mut labeled.asset.asset_id_to_asset_index,
        })
    }

    /// Returns the type-erased labeled asset, if it exists.
    ///
    /// This can be used to get the asset from its handle since `&UntypedHandle` implements
    /// [`Into<UntypedAssetId>`].
    pub fn get_erased_labeled_by_id<Q>(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.asset_id_to_asset_index.get(&id.into())?;
        let labeled = &self.labeled_assets[*index];
        Some(&labeled.asset)
    }

    /// Returns the [`UntypedHandle`] of the labeled asset with the provided 'label', if it exists.
    pub fn get_untyped_handle<Q>(&self, label: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &self.labeled_assets[*index];
        Some(labeled.handle.clone())
    }

    /// Returns the [`Handle`] of the labeled asset with the provided 'label', if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, label: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &self.labeled_assets[*index];
        if let Ok(handle) = labeled.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }

    /// Adds `asset` as a labeled sub asset using `label` and `handle`
    pub fn insert_labeled(
        &mut self,
        label: impl Into<CowArc<'static, str>>,
        handle: impl Into<UntypedHandle>,
        asset: impl Into<ErasedLoadedAsset>,
    ) {
        let labeled = LabeledAsset {
            asset: asset.into(),
            handle: handle.into(),
        };
        match self.label_to_asset_index.entry(label.into()) {
            Entry::Occupied(entry) => {
                let labeled_entry = &mut self.labeled_assets[*entry.get()];
                if labeled.handle != labeled_entry.handle {
                    self.asset_id_to_asset_index
                        .remove(&labeled_entry.handle.id());
                    self.asset_id_to_asset_index
                        .insert(labeled.handle.id(), *entry.get());
                }
                *labeled_entry = labeled;
            }
            Entry::Vacant(entry) => {
                entry.insert(self.labeled_assets.len());
                self.asset_id_to_asset_index
                    .insert(labeled.handle.id(), self.labeled_assets.len());
                self.labeled_assets.push(labeled);
            }
        }
    }

    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.label_to_asset_index.keys().map(|s| &**s)
    }
}

/// A labeled sub-asset of [`TransformedAsset`]
pub struct TransformedSubAsset<'a, A: Asset> {
    value: &'a mut A,
    labeled_assets: &'a mut Vec<LabeledAsset>,
    label_to_asset_index: &'a mut HashMap<CowArc<'static, str>, usize>,
    /// The mapping from a subasset asset IDs to their index in [`Self::labeled_assets`].
    ///
    /// This is entirely redundant with [`Self::labeled_assets`], but it allows looking up the
    /// labeled asset by its asset ID.
    asset_id_to_asset_index: &'a mut HashMap<UntypedAssetId, usize>,
}

impl<'a, A: Asset> Deref for TransformedSubAsset<'a, A> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, A: Asset> DerefMut for TransformedSubAsset<'a, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<'a, A: Asset> TransformedSubAsset<'a, A> {
    /// Creates a new [`TransformedSubAsset`] from `asset` if its internal value matches `A`.
    pub fn from_loaded(asset: &'a mut ErasedLoadedAsset) -> Option<Self> {
        let value = asset.value.downcast_mut::<A>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut asset.labeled_assets,
            label_to_asset_index: &mut asset.label_to_asset_index,
            asset_id_to_asset_index: &mut asset.asset_id_to_asset_index,
        })
    }

    /// Retrieves the value of this asset.
    #[inline]
    pub fn get(&self) -> &A {
        self.value
    }

    /// Mutably retrieves the value of this asset.
    #[inline]
    pub fn get_mut(&mut self) -> &mut A {
        self.value
    }

    /// Returns the labeled asset, if it exists and matches this type.
    pub fn get_labeled<B: Asset, Q>(&mut self, label: &Q) -> Option<TransformedSubAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &mut self.labeled_assets[*index];
        let value = labeled.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut labeled.asset.labeled_assets,
            label_to_asset_index: &mut labeled.asset.label_to_asset_index,
            asset_id_to_asset_index: &mut labeled.asset.asset_id_to_asset_index,
        })
    }

    /// Returns the type-erased labeled asset, if it exists.
    pub fn get_erased_labeled<Q>(&self, label: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &self.labeled_assets[*index];
        Some(&labeled.asset)
    }

    /// Returns the labeled asset given its asset ID if it exists and matches the type.
    ///
    /// This can be used to get the asset from its handle since `&Handle` implements
    /// [`Into<AssetId<B>>`].
    pub fn get_labeled_by_id<B: Asset, Q>(
        &mut self,
        id: impl Into<AssetId<B>>,
    ) -> Option<TransformedSubAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.asset_id_to_asset_index.get(&id.into().untyped())?;
        let labeled = &mut self.labeled_assets[*index];
        let value = labeled.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut labeled.asset.labeled_assets,
            label_to_asset_index: &mut labeled.asset.label_to_asset_index,
            asset_id_to_asset_index: &mut labeled.asset.asset_id_to_asset_index,
        })
    }

    /// Returns the type-erased labeled asset given its asset ID if it exists.
    ///
    /// This can be used to get the asset from its handle since `&UntypedHandle` implements
    /// [`Into<UntypedAssetId>`].
    pub fn get_erased_labeled_by_id<Q>(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.asset_id_to_asset_index.get(&id.into())?;
        let labeled = &self.labeled_assets[*index];
        Some(&labeled.asset)
    }

    /// Returns the [`UntypedHandle`] of the labeled asset with the provided 'label', if it exists.
    pub fn get_untyped_handle<Q>(&self, label: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &self.labeled_assets[*index];
        Some(labeled.handle.clone())
    }

    /// Returns the [`Handle`] of the labeled asset with the provided 'label', if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, label: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let index = self.label_to_asset_index.get(label)?;
        let labeled = &self.labeled_assets[*index];
        if let Ok(handle) = labeled.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }

    /// Adds `asset` as a labeled sub asset using `label` and `handle`
    pub fn insert_labeled(
        &mut self,
        label: impl Into<CowArc<'static, str>>,
        handle: impl Into<UntypedHandle>,
        asset: impl Into<ErasedLoadedAsset>,
    ) {
        let labeled = LabeledAsset {
            asset: asset.into(),
            handle: handle.into(),
        };
        match self.label_to_asset_index.entry(label.into()) {
            Entry::Occupied(entry) => {
                let labeled_entry = &mut self.labeled_assets[*entry.get()];
                if labeled.handle != labeled_entry.handle {
                    self.asset_id_to_asset_index
                        .remove(&labeled_entry.handle.id());
                    self.asset_id_to_asset_index
                        .insert(labeled.handle.id(), *entry.get());
                }
                *labeled_entry = labeled;
            }
            Entry::Vacant(entry) => {
                entry.insert(self.labeled_assets.len());
                self.asset_id_to_asset_index
                    .insert(labeled.handle.id(), self.labeled_assets.len());
                self.labeled_assets.push(labeled);
            }
        }
    }
    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.label_to_asset_index.keys().map(|s| &**s)
    }
}

/// An identity [`AssetTransformer`] which infallibly returns the input [`Asset`] on transformation.]
#[derive(TypePath)]
pub struct IdentityAssetTransformer<A: Asset> {
    _phantom: PhantomData<fn(A) -> A>,
}

impl<A: Asset> IdentityAssetTransformer<A> {
    /// Creates a new [`IdentityAssetTransformer`] with the correct internal [`PhantomData`] field.
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<A: Asset> Default for IdentityAssetTransformer<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Asset> AssetTransformer for IdentityAssetTransformer<A> {
    type AssetInput = A;
    type AssetOutput = A;
    type Settings = ();
    type Error = Infallible;

    async fn transform<'a>(
        &'a self,
        asset: TransformedAsset<Self::AssetInput>,
        _settings: &'a Self::Settings,
    ) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
        Ok(asset)
    }
}
