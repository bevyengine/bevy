use bevy_app::{Plugin, PreUpdate};
use bevy_core_widgets::CoreSlider;
use bevy_ecs::{component::Component, schedule::IntoScheduleConfigs};
use bevy_picking::PickingSystems;

pub struct SliderProps {}

#[derive(Component, Default, Clone)]
#[require(CoreSlider)]
pub struct SliderStyle;

/// Plugin which registers the systems for updating the slider styles.
pub struct SliderPlugin;

fn update_slider_styles() {}

fn update_slider_styles_remove() {}

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_slider_styles, update_slider_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
