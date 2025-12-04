//! The main Earthworks plugin.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::config::EarthworksConfig;
use crate::effects::EffectsPlugin;
use crate::jobs::JobsPlugin;
use crate::machines::MachinesPlugin;
use crate::models::ModelsPlugin;
use crate::plan::PlanPlugin;
use crate::scoring::ScoringPlugin;
use crate::terrain::TerrainPlugin;
use crate::ui::{MinimapPlugin, SelectionUiPlugin, UiPlugin};
use crate::zyns::ZynsPlugin;

/// The main Earthworks plugin that provides volumetric terrain and machine simulation.
///
/// # Example
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_earthworks::prelude::*;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(EarthworksPlugin::default())
///     .run();
/// ```
#[derive(Default)]
pub struct EarthworksPlugin {
    /// Configuration for the plugin.
    pub config: EarthworksConfig,
}

impl EarthworksPlugin {
    /// Creates a new Earthworks plugin with the given configuration.
    pub fn new(config: EarthworksConfig) -> Self {
        Self { config }
    }
}

impl Plugin for EarthworksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .add_plugins(TerrainPlugin)
            .add_plugins(MachinesPlugin)
            .add_plugins(ModelsPlugin)
            .add_plugins(PlanPlugin)
            .add_plugins(ScoringPlugin)
            .add_plugins(EffectsPlugin)
            .add_plugins(ZynsPlugin)
            .add_plugins(JobsPlugin);

        // Only add UI plugins if show_ui is enabled
        if self.config.show_ui {
            app.add_plugins(UiPlugin)
                .add_plugins(SelectionUiPlugin)
                .add_plugins(MinimapPlugin);
        }
    }
}
