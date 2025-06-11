use core::ops::RangeInclusive;

use accesskit::{Orientation, Role};
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::lifecycle::Insert;
use bevy_ecs::query::Has;
use bevy_ecs::system::{In, ResMut};
use bevy_ecs::world::DeferredWorld;
use bevy_ecs::{
    component::Component,
    observer::On,
    query::With,
    system::{Commands, Query, SystemId},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy_picking::events::{Drag, DragEnd, DragStart, Pointer, Press};
use bevy_ui::{ComputedNode, InteractionDisabled};

/// A headless slider widget, which can be used to build custom sliders.
#[derive(Component, Debug, Default)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::Slider)),
    CoreSliderDragState,
    SliderValue,
    SliderRange,
    SliderStep
)]
pub struct CoreSlider {
    /// The size of the thumb element in pixels. This is used to calculate the thumb position.
    /// Note that the size doesn't have to exactly match the thumb entity's size. What this actually
    /// does is reduce the amount of travel of the thumb to account for the fact that the thumb
    /// takes up some space.
    pub thumb_size: f32,

    /// Callback which is called when the slider is dragged or the value is changed via other user
    /// interaction.
    pub on_change: Option<SystemId<In<f32>>>,
    // TODO: Think about whether we want a "vertical" option.

    // TODO: Think about whether we want an option to increment / decrement the value when
    // we click on the track. Currently we only support changing the value by dragging either the
    // thumb or the track. This will require distinguishing between track clicks and thumb clicks,
    // which will likely require a `CoreSliderThumb` marker. We'll also want a "snap to value"
    // option, which is particularly useful for color sliders.
}

/// A component which stores the current value of the slider.
#[derive(Component, Debug, Default, PartialEq, Clone)]
#[component(immutable)]
pub struct SliderValue(pub f32);

/// A component which represents the allowed range of the slider value. Defaults to 0.0..=1.0.
#[derive(Component, Debug, PartialEq, Clone)]
#[component(immutable)]
pub struct SliderRange(pub RangeInclusive<f32>);

impl SliderRange {
    /// Constrain a value between the minimum and maximum allowed values for this slider.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(*self.0.start(), *self.0.end())
    }

    /// Compute the position of the thumb on the slider, as a value between 0 and 1, taking
    /// into account the proportion of the value between the minimum and maximum limits.
    pub fn thumb_position(&self, value: f32) -> f32 {
        if self.0.end() > self.0.start() {
            (value - self.0.start()) / (self.0.end() - self.0.start())
        } else {
            0.5
        }
    }
}

impl Default for SliderRange {
    fn default() -> Self {
        Self(0.0..=1.0)
    }
}

/// Defines the amount by which to increment or decrement the slider value when using keyboard
/// shorctuts. Defaults to 1.0.
#[derive(Component, Debug, PartialEq, Clone)]
#[component(immutable)]
pub struct SliderStep(pub f32);

impl Default for SliderStep {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Component used to manage the state of a slider during dragging.
#[derive(Component, Default)]
pub struct CoreSliderDragState {
    /// Whether the slider is currently being dragged.
    pub dragging: bool,

    /// The value of the slider when dragging started.
    offset: f32,
}

pub(crate) fn slider_on_pointer_down(
    trigger: On<Pointer<Press>>,
    q_state: Query<(), With<CoreSlider>>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    if q_state.contains(trigger.target().unwrap()) {
        // Set focus to slider and hide focus ring
        focus.0 = trigger.target();
        focus_visible.0 = false;
    }
}

pub(crate) fn slider_on_drag_start(
    mut trigger: On<Pointer<DragStart>>,
    mut q_state: Query<
        (
            &SliderValue,
            &mut CoreSliderDragState,
            Has<InteractionDisabled>,
        ),
        With<CoreSlider>,
    >,
) {
    if let Ok((value, mut drag, disabled)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if !disabled {
            drag.dragging = true;
            drag.offset = value.0;
        }
    }
}

pub(crate) fn slider_on_drag(
    mut trigger: On<Pointer<Drag>>,
    mut q_state: Query<(
        &ComputedNode,
        &CoreSlider,
        &SliderRange,
        &mut CoreSliderDragState,
    )>,
    mut commands: Commands,
) {
    if let Ok((node, slider, range, drag)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if drag.dragging {
            let distance = trigger.event().distance;
            let slider_width =
                (node.size().x * node.inverse_scale_factor - slider.thumb_size).max(1.0);
            let span = range.0.end() - range.0.start();
            let new_value = if span > 0. {
                range.clamp(drag.offset + (distance.x * span) / slider_width)
            } else {
                range.0.start() + span * 0.5
            };

            if let Some(on_change) = slider.on_change {
                commands.run_system_with(on_change, new_value);
            }
        }
    }
}

pub(crate) fn slider_on_drag_end(
    mut trigger: On<Pointer<DragEnd>>,
    mut q_state: Query<(&CoreSlider, &mut CoreSliderDragState)>,
) {
    if let Ok((_slider, mut drag)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn slider_on_key_input(
    mut trigger: On<FocusedInput<KeyboardInput>>,
    q_state: Query<(
        &CoreSlider,
        &SliderValue,
        &SliderRange,
        &SliderStep,
        Has<InteractionDisabled>,
    )>,
    mut commands: Commands,
) {
    if let Ok((slider, value, range, step, disabled)) = q_state.get(trigger.target().unwrap()) {
        let event = &trigger.event().input;
        if !disabled && event.state == ButtonState::Pressed {
            let new_value = match event.key_code {
                KeyCode::ArrowLeft => range.clamp(value.0 - step.0),
                KeyCode::ArrowRight => range.clamp(value.0 + step.0),
                KeyCode::Home => *range.0.start(),
                KeyCode::End => *range.0.end(),
                _ => {
                    return;
                }
            };
            trigger.propagate(false);
            if let Some(on_change) = slider.on_change {
                commands.run_system_with(on_change, new_value);
            }
        }
    }
}

pub(crate) fn slider_on_insert(trigger: On<Insert, CoreSlider>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target().unwrap());
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_orientation(Orientation::Horizontal);
    }
}

pub(crate) fn slider_on_insert_value(trigger: On<Insert, SliderValue>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target().unwrap());
    let value = entity.get::<SliderValue>().unwrap().0;
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_numeric_value(value.into());
    }
}

pub(crate) fn slider_on_insert_range(trigger: On<Insert, SliderRange>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target().unwrap());
    let range = entity.get::<SliderRange>().unwrap().0.clone();
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_min_numeric_value((*range.start()).into());
        accessibility.set_max_numeric_value((*range.end()).into());
    }
}

pub(crate) fn slider_on_insert_step(trigger: On<Insert, SliderStep>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target().unwrap());
    let step = entity.get::<SliderStep>().unwrap().0;
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_numeric_value_step(step.into());
    }
}

/// Plugin that adds the observers and systems for the [`CoreSlider`] widget.
pub struct CoreSliderPlugin;

impl Plugin for CoreSliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(slider_on_pointer_down)
            .add_observer(slider_on_drag_start)
            .add_observer(slider_on_drag_end)
            .add_observer(slider_on_drag)
            .add_observer(slider_on_key_input)
            .add_observer(slider_on_insert)
            .add_observer(slider_on_insert_value)
            .add_observer(slider_on_insert_range)
            .add_observer(slider_on_insert_step);
    }
}
