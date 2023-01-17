use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Component;
use bevy_ecs::query::With;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::schedule::{IntoSystemDescriptor, SystemLabel};
use bevy_ecs::system::{Query, Res};
use bevy_hierarchy::Children;
use bevy_input::prelude::MouseButton;
use bevy_input::touch::Touches;
use bevy_input::Input;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use thiserror::Error;

use crate::{Interaction, Node, RelativeCursorPosition, Style, Val};

/// A component describing the slider-specific values
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct Slider {
    min: f32,
    max: f32,
    step: f32,
    value: f32,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            min: 0.,
            max: 100.,
            // Don't round up the slider value
            step: 0.,
            value: 0.,
        }
    }
}

impl Slider {
    /// Creates a new `Slider` with `min` and `max` values
    /// `Min` and `max` don't affect the physical size of the slider, they're only used for calculating the value of the slider
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max,
            step: 0.,
            value: min,
        }
    }

    /// Consumes self, returning a new [`Slider`] with a given value or an error if the value is out of the slider range
    pub fn with_value(self, value: f32) -> Result<Self, SliderValueError> {
        if !(self.min..=self.max).contains(&value) {
            return Err(SliderValueError::ValueOutOfSliderRange);
        }

        Ok(Self { value, ..self })
    }

    /// Consumes self, returning a new [`Slider`] with a given step
    pub fn with_step(self, step: f32) -> Self {
        Self { step, ..self }
    }

    /// Sets the slider value, returning the slider new value or an error if the given value is out of the slider range
    /// It rounds up the slider value to match the value of `step`
    pub fn set_value(&mut self, value: f32) -> Result<f32, SliderValueError> {
        // Round the value up to self.step
        let value = if self.step != 0. {
            ((value - self.min) / self.step).round() * self.step + self.min
        } else {
            value
        };

        if (self.min..=self.max).contains(&value) {
            self.value = value;
            return Ok(value);
        }

        Err(SliderValueError::ValueOutOfSliderRange)
    }

    /// Retrieves the slider value
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Retrieves the minimum slider value
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Retrieves the maximum slider value
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Retrieves the slider step
    pub fn step(&self) -> f32 {
        self.step
    }
}

/// Error connected to setting the value of a slider
#[derive(Debug, Eq, PartialEq, Clone, Copy, Error)]
pub enum SliderValueError {
    #[error("the value given to the Slider is out of range")]
    ValueOutOfSliderRange,
}

/// Marker struct for slider handles
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct SliderHandle;

/// Whether the slider is currently being dragged
#[derive(Component, Debug, Default, Clone, Copy, Reflect, Deref, DerefMut)]
#[reflect(Component, Default)]
pub struct SliderDragged {
    pub dragged: bool,
}

/// A label for the [`update_slider_value`] system
#[derive(SystemLabel)]
pub struct UpdateSliderValue;

/// System for updating slider value based on the user input
pub fn update_slider_value(
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    mut slider_query: Query<(
        &mut Slider,
        &mut SliderDragged,
        &Interaction,
        &RelativeCursorPosition,
        &Node,
        Option<&Children>,
    )>,
    slider_handle_query: Query<&Node, With<SliderHandle>>,
) {
    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();

    for (mut slider, mut slider_dragged, interaction, cursor_position, node, children) in
        slider_query.iter_mut()
    {
        if mouse_released {
            slider_dragged.dragged = false;
        }

        if *interaction == Interaction::Clicked {
            slider_dragged.dragged = true;
        }

        if slider_dragged.dragged {
            let max = slider.max();
            let min = slider.min();

            let slider_width = node.size().x;

            if let Some(cursor_position) = cursor_position.normalized {
                // Get the slider handle node
                let slider_handle_node = if let Some(children) = children {
                    children.iter().find_map(|child| {
                        if let Ok(node) = slider_handle_query.get(*child) {
                            Some(node)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                let handle_width = slider_handle_node.map(|node| node.size().x).unwrap_or(0.);

                // Make it so the cursor dragging is always in the middle of the handle
                let physical_progress = (cursor_position.x - 0.5) * slider_width;
                let progress = physical_progress / (slider_width - handle_width) + 0.5;

                slider
                    .set_value(progress.clamp(0., 1.) * (max - min) + min)
                    .unwrap(); // The unwrap here is alright since the value is clamped between min and max, so it shouldn't return an error
            }
        }
    }
}

/// System for updating the slider handle position based on the parent slider value
pub fn update_slider_handle(
    slider_query: Query<(&Slider, &Node, &Children)>,
    mut slider_handles_query: Query<(&Node, &mut Style), With<SliderHandle>>,
) {
    for (slider, slider_node, slider_children) in slider_query.iter() {
        for child in slider_children {
            if let Ok((slider_handle_node, mut slider_handle_style)) =
                slider_handles_query.get_mut(*child)
            {
                let slider_width = slider_node.size().x - slider_handle_node.size().x;

                slider_handle_style.position.left = Val::Px(
                    (slider.value() - slider.min()) * slider_width / (slider.max() - slider.min()),
                );
            }
        }
    }
}

/// A plugin for adding sliders
#[derive(Default)]
pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_slider_value.label(UpdateSliderValue))
            .add_system(update_slider_handle.after(UpdateSliderValue));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_slider_set_value_test() {
        let mut slider = Slider::default();

        assert_eq!(slider.set_value(42.).unwrap(), 42.);
        assert_eq!(slider.value(), 42.);
    }

    #[test]
    fn slider_set_value_out_of_range_test() {
        let mut slider = Slider::new(10., 30.);

        assert_eq!(
            slider.set_value(42.),
            Err(SliderValueError::ValueOutOfSliderRange)
        );
    }

    #[test]
    fn slider_step_rounding_test() {
        let mut slider = Slider::default().with_step(5.);

        assert_eq!(slider.set_value(42.).unwrap(), 40.);
        assert_eq!(slider.set_value(98.3).unwrap(), 100.);
        assert_eq!(slider.set_value(50.).unwrap(), 50.);
    }

    #[test]
    fn slider_step_rounding_with_fraction_bounds_test() {
        let mut slider = Slider::new(1.32, 2.58).with_step(0.1);

        assert_eq!(slider.set_value(1.35).unwrap(), 1.32);
    }

    #[test]
    fn slider_with_value_test() {
        let slider = Slider::default().with_value(42.).unwrap();

        assert_eq!(slider.value(), 42.);
    }

    #[test]
    fn slider_with_invalid_value_test() {
        let error = Slider::default().with_value(101.).unwrap_err();

        assert_eq!(error, SliderValueError::ValueOutOfSliderRange);
    }
}
