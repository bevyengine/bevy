use crate::{meta::Settings, Asset, ErasedLoadedAsset, Handle, LabeledAsset, UntypedHandle};
use bevy_utils::{ConditionalSendFuture, CowArc, HashMap};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    hash::Hash,
    ops::{Deref, DerefMut},
};

/// Transforms an [`Asset`] of a given [`AssetTransformer::AssetInput`] type to an [`Asset`] of [`AssetTransformer::AssetOutput`] type.
pub trait AssetTransformer: Send + Sync + 'static {
    /// The [`Asset`] type which this [`AssetTransformer`] takes as and input.
    type AssetInput: Asset;
    /// The [`Asset`] type which this [`AssetTransformer`] outputs.
    type AssetOutput: Asset;
    /// The settings type used by this [`AssetTransformer`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [error](`std::error::Error`) which could be encountered by this transformer.
    type Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>;

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
    pub(crate) labeled_assets: HashMap<CowArc<'static, str>, LabeledAsset>,
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
            });
        }
        None
    }
    /// Creates a new [`TransformedAsset`] from `asset`, transferring the `labeled_assets` from this [`TransformedAsset`] to the new one
    pub fn replace_asset<B: Asset>(self, asset: B) -> TransformedAsset<B> {
        TransformedAsset {
            value: asset,
            labeled_assets: self.labeled_assets,
        }
    }
    /// Takes the labeled assets from `labeled_source` and places them in this [`TransformedAsset`]
    pub fn take_labeled_assets<B: Asset>(&mut self, labeled_source: TransformedAsset<B>) {
        self.labeled_assets = labeled_source.labeled_assets;
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
    pub fn get_labeled<B: Asset, Q>(&mut self, label: &Q) -> Option<TransformedSubAsset<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get_mut(label)?;
        let value = labeled.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut labeled.asset.labeled_assets,
        })
    }
    /// Returns the type-erased labeled asset, if it exists and matches this type.
    pub fn get_erased_labeled<Q>(&self, label: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        Some(&labeled.asset)
    }
    /// Returns the [`UntypedHandle`] of the labeled asset with the provided 'label', if it exists.
    pub fn get_untyped_handle<Q>(&self, label: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        Some(labeled.handle.clone())
    }
    /// Returns the [`Handle`] of the labeled asset with the provided 'label', if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, label: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
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
        self.labeled_assets.insert(label.into(), labeled);
    }
    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.labeled_assets.keys().map(|s| &**s)
    }
}

/// A labeled sub-asset of [`TransformedAsset`]
pub struct TransformedSubAsset<'a, A: Asset> {
    value: &'a mut A,
    labeled_assets: &'a mut HashMap<CowArc<'static, str>, LabeledAsset>,
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
    pub fn get_labeled<B: Asset, Q>(&mut self, label: &Q) -> Option<TransformedSubAsset<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get_mut(label)?;
        let value = labeled.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            labeled_assets: &mut labeled.asset.labeled_assets,
        })
    }
    /// Returns the type-erased labeled asset, if it exists and matches this type.
    pub fn get_erased_labeled<Q>(&self, label: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        Some(&labeled.asset)
    }
    /// Returns the [`UntypedHandle`] of the labeled asset with the provided 'label', if it exists.
    pub fn get_untyped_handle<Q>(&self, label: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        Some(labeled.handle.clone())
    }
    /// Returns the [`Handle`] of the labeled asset with the provided 'label', if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, label: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
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
        self.labeled_assets.insert(label.into(), labeled);
    }
    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.labeled_assets.keys().map(|s| &**s)
    }
}
