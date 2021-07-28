use crate::{Asset, Assets};
use bevy_app::prelude::*;
use bevy_diagnostic::{Diagnostic, DiagnosticId, Diagnostics, MAX_DIAGNOSTIC_NAME_WIDTH};
use bevy_ecs::system::{Res, ResMut};

/// Adds "asset count" diagnostic to an App
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
        app.add_startup_system(Self::setup_system)
            .add_system(Self::diagnostic_system);
    }
}

impl<T: Asset> AssetCountDiagnosticsPlugin<T> {
    pub fn diagnostic_id() -> DiagnosticId {
        DiagnosticId(T::TYPE_UUID)
    }

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
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

    pub fn diagnostic_system(mut diagnostics: ResMut<Diagnostics>, assets: Res<Assets<T>>) {
        diagnostics.add_measurement(Self::diagnostic_id(), assets.len() as f64);
    }
}
