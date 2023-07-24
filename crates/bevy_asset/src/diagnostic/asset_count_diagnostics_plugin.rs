use std::marker::PhantomData;

use crate::{Asset, Assets};
use bevy_app::prelude::*;
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore};
use bevy_ecs::prelude::*;

/// Adds an asset count diagnostic to an [`App`] for assets of type `T`.
pub struct AssetCountDiagnosticsPlugin<T: Asset> {
    marker: PhantomData<T>,
}

impl<T: Asset> Default for AssetCountDiagnosticsPlugin<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T: Asset> Plugin for AssetCountDiagnosticsPlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetDiagnosticPath {
            inner: Self::diagnostic_path(),
            marker: PhantomData::<T>,
        })
        .add_systems(Startup, Self::setup_system)
        .add_systems(Update, Self::diagnostic_system);
    }
}

impl<T: Asset> AssetCountDiagnosticsPlugin<T> {
    /// Gets diagnostic path of format `asset/{T::type_path}/count`
    pub fn diagnostic_path() -> DiagnosticPath {
        DiagnosticPath::from_components(["asset", T::type_path(), "count"])
    }

    /// Registers the asset count diagnostic for the current application.
    pub fn setup_system(
        path: Res<AssetDiagnosticPath<T>>,
        mut diagnostics: ResMut<DiagnosticsStore>,
    ) {
        diagnostics.add(Diagnostic::new(path.inner.clone()));
    }

    /// Updates the asset count of `T` assets.
    pub fn diagnostic_system(
        path: Res<AssetDiagnosticPath<T>>,
        mut diagnostics: Diagnostics,
        assets: Res<Assets<T>>,
    ) {
        diagnostics.add_measurement(&path.inner, || assets.len() as f64);
    }
}

#[derive(Debug, Clone, Resource)]
pub struct AssetDiagnosticPath<T: Asset> {
    pub inner: DiagnosticPath,
    marker: std::marker::PhantomData<T>,
}
