use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::schedule::IntoSystemDescriptor;
use bevy_ui::UiSystem;
use progress_bar::update_progress_bars;

mod progress_bar;

pub use progress_bar::*;

/// The basic plugin for Bevy Widget
#[derive(Default)]
pub struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ProgressBarWidget>()
            .register_type::<ProgressBarInner>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_progress_bars.before(UiSystem::Flex),
            );
    }
}
