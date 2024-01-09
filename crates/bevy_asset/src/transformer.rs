use crate::{meta::Settings, Asset};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, marker::PhantomData};

/// Transforms an [`Asset`] of a given [`AssetTransformer::AssetInput`] type to an [`Asset`] of [`AssetTransformer::AssetOutput`] type.
pub trait AssetTransformer: Send + Sync + 'static {
    /// The [`Asset`] type which this [`AssetTransformer`] takes as and input.
    type AssetInput: Asset;
    /// The [`Asset`] type which this [`AssetTransformer`] outputs.
    type AssetOutput: Asset;
    /// The settings type used by this [`AssetTransformer`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [error](`std::error::Error`) which could be encountered by this saver.
    type Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>;

    fn transform<'a>(
        &'a self,
        asset: Self::AssetInput,
        settings: &'a Self::Settings,
    ) -> Result<Self::AssetOutput, Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// An [`AssetTransformer`] implementation which returns the original [`Asset`] unchanged.
/// This is useful when
pub struct NoopAssetTransformer<A: Asset> {
    marker: PhantomData<fn() -> A>,
}

impl<A: Asset> AssetTransformer for NoopAssetTransformer<A> {
    type AssetInput = A;
    type AssetOutput = A;
    type Settings = ();
    type Error = Infallible;

    fn transform<'a>(
        &'a self,
        asset: Self::AssetInput,
        _settings: &'a Self::Settings,
    ) -> Result<Self::AssetOutput, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(asset)
    }
}

impl<A: Asset> Default for NoopAssetTransformer<A> {
    fn default() -> Self {
        NoopAssetTransformer {
            marker: Default::default(),
        }
    }
}
