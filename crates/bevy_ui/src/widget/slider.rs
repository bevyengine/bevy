use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Component;
use bevy_ecs::query::With;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::schedule::SystemLabel;
use bevy_ecs::system::{Query, Res};
use bevy_hierarchy::Children;
use bevy_input::prelude::MouseButton;
use bevy_input::touch::Touches;
use bevy_input::Input;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use thiserror::Error;

use crate::{Interaction, Node, RelativeCursorPosition, Style, Val};

/// A component describing the slider-specific value, such as max and min values and step
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

    /// Consumes self, returning a new [`Slider`] with a given value
    pub fn with_value(self, value: f32) -> Self {
        Self { value, ..self }
    }

    /// Consumes self, returning a new [`Slider`] with a given step
    pub fn with_step(self, step: f32) -> Self {
        Self { step, ..self }
    }

    /// Sets the slider value, returning error if the given value is out of the slider range
    pub fn set_value(&mut self, value: f32) -> Result<(), SliderValueError> {
        // Round the value up to self.step (we have to consider that self.min can be a fraction)
        let value = if self.step != 0. {
            ((value - self.min) / self.step).round() * self.step + self.min
        } else {
            value
        };

        if (self.min..=self.max).contains(&value) {
            self.value = value;
            return Ok(());
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
pub struct SliderDragged(bool);

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
    )>,
) {
    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();

    for (mut slider, mut slider_dragged, interaction, cursor_position) in slider_query.iter_mut() {
        if mouse_released {
            slider_dragged.0 = false;
        }

        if *interaction == Interaction::Clicked {
            slider_dragged.0 = true;
        }

        if slider_dragged.0 {
            let max = slider.max();
            let min = slider.min();

            slider
                .set_value(cursor_position.x.clamp(0., 1.) * (max - min) + min)
                .unwrap(); // The unwrap here is alright since the value is clamped between min and max, so it shouldn't return an error
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
            let (slider_handle_node, mut slider_handle_style) =
                slider_handles_query.get_mut(*child).unwrap();

            let slider_width = slider_node.size().x - slider_handle_node.size().x;

            slider_handle_style.position.left = Val::Px(
                (slider.value() - slider.min()) * slider_width / (slider.max() - slider.min()),
            );
        }
    }
}