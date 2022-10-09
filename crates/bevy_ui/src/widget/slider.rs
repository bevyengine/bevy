use bevy_ecs::query::With;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::schedule::SystemLabel;
use bevy_ecs::system::Query;
use bevy_ecs::{prelude::Component, query::Changed};
use bevy_hierarchy::Children;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use thiserror::Error;

use crate::{Node, PositionedInteraction, Style, Val};

/// Describes the slider-specific value, such as max and min values and step
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct Slider {
    min: f32,
    max: f32,
    step: Option<f32>,
    value: f32,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            min: 0.,
            max: 100.,
            step: None,
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
            step: None,
            value: min,
        }
    }

    // Consumes self, returning a new [`Slider`] with a given value
    pub fn with_value(self, value: f32) -> Self {
        Self { value, ..self }
    }

    // Consumes self, returning a new [`Slider`] with a given step
    pub fn with_step(self, step: f32) -> Self {
        if step == 0. {
            return Self { step: None, ..self };
        }
        Self {
            step: Some(step),
            ..self
        }
    }

    pub fn set_value(&mut self, value: f32) -> Result<(), SliderValueError> {
        // Round the value up to self.step (we have to consider that self.min can be a fraction)
        let value = if let Some(step) = self.step {
            (value / step).round() * step
        } else {
            value
        };

        if (self.min..self.max).contains(&value) {
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
        if let Some(step) = self.step {
            return step;
        }

        0.
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

/// A label for the [`update_slider_value`] system
#[derive(SystemLabel)]
pub struct UpdateSliderValue;

pub fn update_slider_value(
    mut slider_query: Query<(&mut Slider, &PositionedInteraction), Changed<PositionedInteraction>>,
) {
    for (mut slider, interaction) in slider_query.iter_mut() {
        match *interaction {
            PositionedInteraction::Pressed(pos) => {
                let min = slider.get_min();
                let max = slider.get_max();

                slider.set_value(pos.x * (max - min)).unwrap();
            }
            _ => (),
        }
    }
}

pub fn update_slider_handle(
    slider_query: Query<(&Slider, &Node, &Children), Changed<Slider>>,
    mut slider_handles_query: Query<(&Node, &mut Style), With<SliderHandle>>,
) {
    for (slider, slider_node, slider_children) in slider_query.iter() {
        for child in slider_children {
            let (slider_handle_node, mut slider_handle_style) = slider_handles_query.get_mut(*child).unwrap();

            let slider_width = slider_node.size.x - slider_handle_node.size.x;

            slider_handle_style.margin.left = Val::Px(slider.get_value() * slider_width / (slider.get_max() - slider.get_min()));
        }
    }
}
