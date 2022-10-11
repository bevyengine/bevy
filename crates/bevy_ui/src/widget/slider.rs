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

/// Describes the slider-specific value, such as max and min values and step
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

    // Consumes self, returning a new [`Slider`] with a given value
    pub fn with_value(self, value: f32) -> Self {
        Self { value, ..self }
    }

    // Consumes self, returning a new [`Slider`] with a given step
    pub fn with_step(self, step: f32) -> Self {
        Self { step, ..self }
    }

    pub fn set_value(&mut self, value: f32) -> Result<(), SliderValueError> {
        // Round the value up to self.step (we have to consider that self.min can be a fraction)
        let value = if self.step != 0. {
            (value / self.step).round() * self.step
        } else {
            value
        };

        if (self.min..=self.max).contains(&value) {
            self.value = value;
            return Ok(());
        }

        Err(SliderValueError::ValueOutOfSliderRange)
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    pub fn get_min(&self) -> f32 {
        self.min
    }

    pub fn get_max(&self) -> f32 {
        self.max
    }

    pub fn get_step(&self) -> f32 {
        self.step
    }
}

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
            let max = slider.get_max();
            let min = slider.get_min();

            slider
                .set_value(cursor_position.x.clamp(0., 1.) * (max - min) + min)
                .unwrap(); // The unwrap here is alright since the value is clamped between min and max, so it shouldn't return an error
        }
    }
}

pub fn update_slider_handle(
    slider_query: Query<(&Slider, &Node, &Children)>,
    mut slider_handles_query: Query<(&Node, &mut Style), With<SliderHandle>>,
) {
    for (slider, slider_node, slider_children) in slider_query.iter() {
        for child in slider_children {
            let (slider_handle_node, mut slider_handle_style) =
                slider_handles_query.get_mut(*child).unwrap();

            let slider_width = slider_node.size.x - slider_handle_node.size.x;

            slider_handle_style.margin.left = Val::Px(
                (slider.get_value() - slider.get_min()) * slider_width
                    / (slider.get_max() - slider.get_min()),
            );
        }
    }
}
