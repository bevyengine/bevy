use core::ops::RangeInclusive;

use accesskit::{Orientation, Role};
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::event::Event;
use bevy_ecs::hierarchy::Children;
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
use bevy_ui::{ComputedNode, ComputedNodeTarget, InteractionDisabled, UiGlobalTransform};

/// Defines how the slider should behave when you click on the track (not the thumb).
#[derive(Debug, Default)]
pub enum TrackClick {
    /// Clicking on the track lets you drag to edit the value, just like clicking on the thumb.
    #[default]
    Drag,
    /// Clicking on the track increments or decrements the slider by [`StepSize`].
    Step,
    /// Clicking on the track snaps the value to the clicked position.
    Snap,
}

/// A headless slider widget, which can be used to build custom sliders. Sliders have a value
/// (represented by the [`SliderValue`] component) and a range (represented by [`SliderRange`]).
/// An optional step size can be specified via [`SliderStep`].
///
/// Typically a slider will contain entities representing the "track" and "thumb" elements.
/// The core slider makes no assumptions about the hierarchical structure of these elements,
/// but expects that the thumb will be marked with a [`CoreSliderThumb`] component.
#[derive(Component, Debug, Default)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::Slider)),
    CoreSliderDragState,
    SliderValue,
    SliderRange,
    SliderStep
)]
pub struct CoreSlider {
    /// Callback which is called when the slider is dragged or the value is changed via other user
    /// interaction. If this value is `None`, then the slider will self-update.
    pub on_change: Option<SystemId<In<f32>>>,
    /// Set the track-clicking behavior for this slider.
    pub track_click: TrackClick,
    // TODO: Think about whether we want a "vertical" option.
}

/// Marker component that identifies which descendant element is the slider thumb.
#[derive(Component, Debug, Default)]
pub struct CoreSliderThumb;

/// A component which stores the current value of the slider.
#[derive(Component, Debug, Default, PartialEq, Clone)]
#[component(immutable)]
pub struct SliderValue(pub f32);

/// A component which represents the allowed range of the slider value. Defaults to 0.0..=1.0.
#[derive(Component, Debug, PartialEq, Clone)]
#[component(immutable)]
pub struct SliderRange(pub RangeInclusive<f32>);

impl SliderRange {
    /// Returns the minimum allowed value for this slider.
    pub fn start(&self) -> f32 {
        *self.0.start()
    }

    /// Returns the maximum allowed value for this slider.
    pub fn end(&self) -> f32 {
        *self.0.end()
    }

    /// Returns the full span of the range (max - min).
    pub fn span(&self) -> f32 {
        self.end() - self.start()
    }

    /// Returns the center value of the range.
    pub fn center(&self) -> f32 {
        (self.start() + self.end()) / 2.0
    }

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
    mut trigger: On<Pointer<Press>>,
    q_slider: Query<(
        &CoreSlider,
        &SliderValue,
        &SliderRange,
        &SliderStep,
        &ComputedNode,
        &ComputedNodeTarget,
        &UiGlobalTransform,
    )>,
    q_thumb: Query<&ComputedNode, With<CoreSliderThumb>>,
    q_children: Query<&Children>,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    if q_thumb.contains(trigger.target().unwrap()) {
        // Thumb click, stop propagation to prevent track click.
        trigger.propagate(false);

        // Set focus to slider and hide focus ring
        if let Some(mut focus) = focus {
            focus.0 = trigger.target();
        }
        if let Some(mut focus_visible) = focus_visible {
            focus_visible.0 = false;
        }
    } else if let Ok((slider, value, range, step, node, node_target, transform)) =
        q_slider.get(trigger.target().unwrap())
    {
        // Track click
        trigger.propagate(false);

        // Set focus to slider and hide focus ring
        if let Some(mut focus) = focus {
            focus.0 = trigger.target();
        }
        if let Some(mut focus_visible) = focus_visible {
            focus_visible.0 = false;
        }

        // Find thumb size by searching descendants for the first entity with CoreSliderThumb
        let thumb_size = q_children
            .iter_descendants(trigger.target().unwrap())
            .find_map(|child_id| q_thumb.get(child_id).ok().map(|thumb| thumb.size().x))
            .unwrap_or(0.0);

        // Detect track click.
        let local_pos = transform.try_inverse().unwrap().transform_point2(
            trigger.event().pointer_location.position * node_target.scale_factor(),
        );
        let track_width = node.size().x - thumb_size;
        // Avoid division by zero
        let click_val = if track_width > 0. {
            local_pos.x * range.span() / track_width + range.center()
        } else {
            0.
        };

        // Compute new value from click position
        let new_value = range.clamp(match slider.track_click {
            TrackClick::Drag => {
                return;
            }
            TrackClick::Step => {
                if click_val < value.0 {
                    value.0 - step.0
                } else {
                    value.0 + step.0
                }
            }
            TrackClick::Snap => click_val,
        });

        if let Some(on_change) = slider.on_change {
            commands.run_system_with(on_change, new_value);
        } else {
            commands
                .entity(trigger.target().unwrap())
                .insert(SliderValue(new_value));
        }
    }
}

pub(crate) fn slider_on_drag_start(
    mut trigger: On<Pointer<DragStart>>,
    mut q_slider: Query<
        (
            &SliderValue,
            &mut CoreSliderDragState,
            Has<InteractionDisabled>,
        ),
        With<CoreSlider>,
    >,
) {
    if let Ok((value, mut drag, disabled)) = q_slider.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if !disabled {
            drag.dragging = true;
            drag.offset = value.0;
        }
    }
}

pub(crate) fn slider_on_drag(
    mut trigger: On<Pointer<Drag>>,
    mut q_slider: Query<(
        &ComputedNode,
        &CoreSlider,
        &SliderRange,
        &mut CoreSliderDragState,
    )>,
    q_thumb: Query<&ComputedNode, With<CoreSliderThumb>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if let Ok((node, slider, range, drag)) = q_slider.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if drag.dragging {
            let distance = trigger.event().distance;
            // Find thumb size by searching descendants for the first entity with CoreSliderThumb
            let thumb_size = q_children
                .iter_descendants(trigger.target().unwrap())
                .find_map(|child_id| q_thumb.get(child_id).ok().map(|thumb| thumb.size().x))
                .unwrap_or(0.0);
            let slider_width = ((node.size().x - thumb_size) * node.inverse_scale_factor).max(1.0);
            let span = range.span();
            let new_value = if span > 0. {
                range.clamp(drag.offset + (distance.x * span) / slider_width)
            } else {
                range.start() + span * 0.5
            };

            if let Some(on_change) = slider.on_change {
                commands.run_system_with(on_change, new_value);
            } else {
                commands
                    .entity(trigger.target().unwrap())
                    .insert(SliderValue(new_value));
            }
        }
    }
}

pub(crate) fn slider_on_drag_end(
    mut trigger: On<Pointer<DragEnd>>,
    mut q_slider: Query<(&CoreSlider, &mut CoreSliderDragState)>,
) {
    if let Ok((_slider, mut drag)) = q_slider.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn slider_on_key_input(
    mut trigger: On<FocusedInput<KeyboardInput>>,
    q_slider: Query<(
        &CoreSlider,
        &SliderValue,
        &SliderRange,
        &SliderStep,
        Has<InteractionDisabled>,
    )>,
    mut commands: Commands,
) {
    if let Ok((slider, value, range, step, disabled)) = q_slider.get(trigger.target().unwrap()) {
        let event = &trigger.event().input;
        if !disabled && event.state == ButtonState::Pressed {
            let new_value = match event.key_code {
                KeyCode::ArrowLeft => range.clamp(value.0 - step.0),
                KeyCode::ArrowRight => range.clamp(value.0 + step.0),
                KeyCode::Home => range.start(),
                KeyCode::End => range.end(),
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

/// Event which can be triggered on a slider to modify the value (using the `on_change` callback).
/// This can be used to control the slider via gamepad buttons or other inputs. The value will be
/// clamped when the event is processed.
///
/// # Example:
///
/// ```
/// use bevy_ecs::system::Commands;
/// use bevy_core_widgets::{CoreSlider, SliderRange, SliderValue, SetSliderValue};
///
/// fn setup(mut commands: Commands) {
///     // Create a slider
///     let slider = commands.spawn((
///         CoreSlider::default(),
///         SliderValue(0.5),
///         SliderRange(0.0..=1.0),
///     )).id();
///
///     // Set to an absolute value
///     commands.trigger_targets(SetSliderValue::Absolute(0.75), slider);
///
///     // Adjust relatively
///     commands.trigger_targets(SetSliderValue::Relative(-0.25), slider);
/// }
/// ```
#[derive(Event)]
pub enum SetSliderValue {
    /// Set the slider value to a specific value.
    Absolute(f32),
    /// Add a delta to the slider value.
    Relative(f32),
}

fn slider_on_set_value(
    mut trigger: On<SetSliderValue>,
    q_slider: Query<(&CoreSlider, &SliderValue, &SliderRange)>,
    mut commands: Commands,
) {
    if let Ok((slider, value, range)) = q_slider.get(trigger.target().unwrap()) {
        trigger.propagate(false);
        let new_value = match trigger.event() {
            SetSliderValue::Absolute(new_value) => range.clamp(*new_value),
            SetSliderValue::Relative(delta) => range.clamp(value.0 + *delta),
        };
        if let Some(on_change) = slider.on_change {
            commands.run_system_with(on_change, new_value);
        } else {
            commands
                .entity(trigger.target().unwrap())
                .insert(SliderValue(new_value));
        }
    }
}

/// Plugin that adds the observers for the [`CoreSlider`] widget.
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
            .add_observer(slider_on_insert_step)
            .add_observer(slider_on_set_value);
    }
}
