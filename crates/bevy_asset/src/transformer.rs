use crate::{meta::Settings, Asset, ErasedLoadedAsset, Handle, LoadedSubAsset, UntypedHandle};
use alloc::boxed::Box;
use atomicow::CowArc;
use bevy_platform::collections::HashMap;
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
    /// The [`TransformedAsset`]'s `subassets` can be altered to add new Sub-Assets
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
    pub(crate) subassets: HashMap<CowArc<'static, str>, LoadedSubAsset>,
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
                subassets: asset.subassets,
            });
        }
        None
    }
    /// Creates a new [`TransformedAsset`] from `asset`, transferring the `subassets` from this [`TransformedAsset`] to the new one
    pub fn replace_asset<B: Asset>(self, asset: B) -> TransformedAsset<B> {
        TransformedAsset {
            value: asset,
            subassets: self.subassets,
        }
    }
    /// Takes the subassets from `subasset_source` and places them in this [`TransformedAsset`]
    pub fn take_subassets<B: Asset>(&mut self, subasset_source: TransformedAsset<B>) {
        self.subassets = subasset_source.subassets;
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
    /// Returns the subasset, if it exists and matches this type.
    pub fn get_subasset<B: Asset, Q>(
        &mut self,
        subasset_name: &Q,
    ) -> Option<TransformedSubAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get_mut(subasset_name)?;
        let value = subasset.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            subassets: &mut subasset.asset.subassets,
        })
    }
    /// Returns the type-erased subasset, if it exists and matches this type.
    pub fn get_erased_subasset<Q>(&self, subasset_name: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        Some(&subasset.asset)
    }
    /// Returns the [`UntypedHandle`] of the subasset with the provided `subasset_name`, if it exists.
    pub fn get_untyped_handle<Q>(&self, subasset_name: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        Some(subasset.handle.clone())
    }
    /// Returns the [`Handle`] of the subasset with the provided `subasset_name`, if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, subasset_name: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        if let Ok(handle) = subasset.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }
    /// Adds `asset` as a sub asset using `subasset_name` and `handle`
    pub fn insert_subasset(
        &mut self,
        subasset_name: impl Into<CowArc<'static, str>>,
        handle: impl Into<UntypedHandle>,
        asset: impl Into<ErasedLoadedAsset>,
    ) {
        let subasset = LoadedSubAsset {
            asset: asset.into(),
            handle: handle.into(),
        };
        self.subassets.insert(subasset_name.into(), subasset);
    }
    /// Iterate over all subasset names in this loaded asset.
    pub fn iter_subasset_names(&self) -> impl Iterator<Item = &str> {
        self.subassets.keys().map(|s| &**s)
    }
}

/// A sub-asset of [`TransformedAsset`]
pub struct TransformedSubAsset<'a, A: Asset> {
    value: &'a mut A,
    subassets: &'a mut HashMap<CowArc<'static, str>, LoadedSubAsset>,
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
            subassets: &mut asset.subassets,
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
    /// Returns the subasset, if it exists and matches this type.
    pub fn get_subasset<B: Asset, Q>(
        &mut self,
        subasset_name: &Q,
    ) -> Option<TransformedSubAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get_mut(subasset_name)?;
        let value = subasset.asset.value.downcast_mut::<B>()?;
        Some(TransformedSubAsset {
            value,
            subassets: &mut subasset.asset.subassets,
        })
    }
    /// Returns the type-erased subasset, if it exists and matches this type.
    pub fn get_erased_subasset<Q>(&self, subasset_name: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        Some(&subasset.asset)
    }
    /// Returns the [`UntypedHandle`] of the subasset with the provided `subasset_name`, if it exists.
    pub fn get_untyped_handle<Q>(&self, subasset_name: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        Some(subasset.handle.clone())
    }
    /// Returns the [`Handle`] of the subasset with the provided `subasset_name`, if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, subasset_name: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        if let Ok(handle) = subasset.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }
    /// Adds `asset` as a sub asset using `subasset_name` and `handle`
    pub fn insert_subasset(
        &mut self,
        subasset_name: impl Into<CowArc<'static, str>>,
        handle: impl Into<UntypedHandle>,
        asset: impl Into<ErasedLoadedAsset>,
    ) {
        let subasset = LoadedSubAsset {
            asset: asset.into(),
            handle: handle.into(),
        };
        self.subassets.insert(subasset_name.into(), subasset);
    }
    /// Iterate over all subasset names in this loaded asset.
    pub fn iter_subasset_names(&self) -> impl Iterator<Item = &str> {
        self.subassets.keys().map(|s| &**s)
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
