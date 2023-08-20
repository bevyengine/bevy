use crate::{Asset, Assets};
use bevy_app::prelude::*;
use bevy_diagnostic::{
    Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, MAX_DIAGNOSTIC_NAME_WIDTH,
};
use bevy_ecs::prelude::*;

/// Adds an asset count diagnostic to an [`App`] for assets of type `T`.
pub struct AssetCountDiagnosticsPlugin<T: Asset> {
    marker: std::marker::PhantomData<T>,
}

impl<T: Asset> Default for AssetCountDiagnosticsPlugin<T> {
    fn default() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<T: Asset> Plugin for AssetCountDiagnosticsPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup_system)
            .add_systems(Update, Self::diagnostic_system);
    }
}

impl<T: Asset> AssetCountDiagnosticsPlugin<T> {
    /// Gets unique id of this diagnostic.
    ///
    /// The diagnostic id is the type uuid of `T`.
    pub fn diagnostic_id() -> DiagnosticId {
        DiagnosticId(T::TYPE_UUID)
    }

    /// Registers the asset count diagnostic for the current application.
    pub fn setup_system(mut diagnostics: ResMut<DiagnosticsStore>) {
        let asset_type_name = std::any::type_name::<T>();
        let max_length = MAX_DIAGNOSTIC_NAME_WIDTH - "asset_count ".len();
        diagnostics.add(Diagnostic::new(
            Self::diagnostic_id(),
            format!(
                "asset_count {}",
                if asset_type_name.len() > max_length {
                    asset_type_name
                        .split_at(asset_type_name.len() - max_length + 1)
                        .1
                } else {
                    asset_type_name
                }
            ),
            20,
        ));
    }

    /// Updates the asset count of `T` assets.
    pub fn diagnostic_system(mut diagnostics: Diagnostics, assets: Res<Assets<T>>) {
        diagnostics.add_measurement(Self::diagnostic_id(), || assets.len() as f64);
    }
}
