//! Asset loader for execution plans.

use bevy_asset::{io::Reader, AssetLoader, LoadContext};
use thiserror::Error;

use super::schema::ExecutionPlan;

/// Error type for plan loading.
#[derive(Debug, Error)]
pub enum PlanLoadError {
    /// IO error reading the file.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Asset loader for ExecutionPlan JSON files.
#[derive(Default)]
pub struct PlanLoader;

impl AssetLoader for PlanLoader {
    type Asset = ExecutionPlan;
    type Settings = ();
    type Error = PlanLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let plan: ExecutionPlan = serde_json::from_slice(&bytes)?;
        Ok(plan)
    }

    fn extensions(&self) -> &[&str] {
        &["plan.json", "earthworks.json"]
    }
}
