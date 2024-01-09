use crate::{meta::Settings, Asset};
use serde::{Deserialize, Serialize};

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
