use core::ops::RangeInclusive;

use accesskit::{Orientation, Role};
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::event::EntityEvent;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::lifecycle::Insert;
use bevy_ecs::query::Has;
use bevy_ecs::system::{In, Res};
use bevy_ecs::world::DeferredWorld;
use bevy_ecs::{
    component::Component,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_log::warn_once;
use bevy_math::ops;
use bevy_picking::events::{Drag, DragEnd, DragStart, Pointer, Press};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{ComputedNode, ComputedNodeTarget, InteractionDisabled, UiGlobalTransform, UiScale};

use crate::{Callback, Notify, ValueChange};

/// Defines how the slider should behave when you click on the track (not the thumb).
#[derive(Debug, Default, PartialEq, Clone, Copy, Reflect)]
#[reflect(Clone, PartialEq, Default)]
pub enum TrackClick {
    /// Clicking on the track lets you drag to edit the value, just like clicking on the thumb.
    #[default]
    Drag,
    /// Clicking on the track increments or decrements the slider by [`SliderStep`].
    Step,
    /// Clicking on the track snaps the value to the clicked position.
    Snap,
}

/// A headless slider widget, which can be used to build custom sliders. Sliders have a value
/// (represented by the [`SliderValue`] component) and a range (represented by [`SliderRange`]). An
/// optional step size can be specified via [`SliderStep`], and you can control the rounding
/// during dragging with [`SliderPrecision`].
///
/// You can also control the slider remotely by triggering a [`SetSliderValue`] event on it. This
/// can be useful in a console environment for controlling the value gamepad inputs.
///
/// The presence of the `on_change` property controls whether the slider uses internal or external
/// state management. If the `on_change` property is `None`, then the slider updates its own state
/// automatically. Otherwise, the `on_change` property contains the id of a one-shot system which is
/// passed the new slider value. In this case, the slider value is not modified, it is the
/// responsibility of the callback to trigger whatever data-binding mechanism is used to update the
/// slider's value.
///
/// Typically a slider will contain entities representing the "track" and "thumb" elements. The core
/// slider makes no assumptions about the hierarchical structure of these elements, but expects that
/// the thumb will be marked with a [`CoreSliderThumb`] component.
///
/// The core slider does not modify the visible position of the thumb: that is the responsibility of
/// the stylist. This can be done either in percent or pixel units as desired. To prevent overhang
/// at the ends of the slider, the positioning should take into account the thumb width, by reducing
/// the amount of travel. So for example, in a slider 100px wide, with a thumb that is 10px, the
/// amount of travel is 90px. The core slider's calculations for clicking and dragging assume this
/// is the case, and will reduce the travel by the measured size of the thumb entity, which allows
/// the movement of the thumb to be perfectly synchronized with the movement of the mouse.
///
/// In cases where overhang is desired for artistic reasons, the thumb may have additional
/// decorative child elements, absolutely positioned, which don't affect the size measurement.
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
    /// interaction. If this value is `Callback::Ignore`, then the slider will update it's own
    /// internal [`SliderValue`] state without notification.
    pub on_change: Callback<In<ValueChange<f32>>>,
    /// Set the track-clicking behavior for this slider.
    pub track_click: TrackClick,
    // TODO: Think about whether we want a "vertical" option.
}

/// Marker component that identifies which descendant element is the slider thumb.
#[derive(Component, Debug, Default)]
pub struct CoreSliderThumb;

/// A component which stores the current value of the slider.
#[derive(Component, Debug, Default, PartialEq, Clone, Copy)]
#[component(immutable)]
pub struct SliderValue(pub f32);

/// A component which represents the allowed range of the slider value. Defaults to 0.0..=1.0.
#[derive(Component, Debug, PartialEq, Clone, Copy)]
#[component(immutable)]
pub struct SliderRange {
    /// The beginning of the allowed range for the slider value.
    start: f32,
    /// The end of the allowed range for the slider value.
    end: f32,
}

impl SliderRange {
    /// Creates a new slider range with the given start and end values.
    pub fn new(start: f32, end: f32) -> Self {
        if end < start {
            warn_once!(
                "Expected SliderRange::start ({}) <= SliderRange::end ({})",
                start,
                end
            );
        }
        Self { start, end }
    }

    /// Creates a new slider range from a Rust range.
    pub fn from_range(range: RangeInclusive<f32>) -> Self {
        let (start, end) = range.into_inner();
        Self { start, end }
    }

    /// Returns the minimum allowed value for this slider.
    pub fn start(&self) -> f32 {
        self.start
    }

    /// Return a new instance of a `SliderRange` with a new start position.
    pub fn with_start(&self, start: f32) -> Self {
        Self::new(start, self.end)
    }

    /// Returns the maximum allowed value for this slider.
    pub fn end(&self) -> f32 {
        self.end
    }

    /// Return a new instance of a `SliderRange` with a new end position.
    pub fn with_end(&self, end: f32) -> Self {
        Self::new(self.start, end)
    }

    /// Returns the full span of the range (max - min).
    pub fn span(&self) -> f32 {
        self.end - self.start
    }

    /// Returns the center value of the range.
    pub fn center(&self) -> f32 {
        (self.start + self.end) / 2.0
    }

    /// Constrain a value between the minimum and maximum allowed values for this slider.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.start, self.end)
    }

    /// Compute the position of the thumb on the slider, as a value between 0 and 1, taking
    /// into account the proportion of the value between the minimum and maximum limits.
    pub fn thumb_position(&self, value: f32) -> f32 {
        if self.end > self.start {
            (value - self.start) / (self.end - self.start)
        } else {
            0.5
        }
    }
}

impl Default for SliderRange {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
        }
    }
}

/// Defines the amount by which to increment or decrement the slider value when using keyboard
/// shortcuts. Defaults to 1.0.
#[derive(Component, Debug, PartialEq, Clone)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct SliderStep(pub f32);

impl Default for SliderStep {
    fn default() -> Self {
        Self(1.0)
    }
}

/// A component which controls the rounding of the slider value during dragging.
///
/// Stepping is not affected, although presumably the step size will be an integer multiple of the
/// rounding factor. This also doesn't prevent the slider value from being set to non-rounded values
/// by other means, such as manually entering digits via a numeric input field.
///
/// The value in this component represents the number of decimal places of desired precision, so a
/// value of 2 would round to the nearest 1/100th. A value of -3 would round to the nearest
/// thousand.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct SliderPrecision(pub i32);

impl SliderPrecision {
    fn round(&self, value: f32) -> f32 {
        let factor = ops::powf(10.0_f32, self.0 as f32);
        (value * factor).round() / factor
    }
}

/// Component used to manage the state of a slider during dragging.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
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
        Option<&SliderPrecision>,
        &ComputedNode,
        &ComputedNodeTarget,
        &UiGlobalTransform,
        Has<InteractionDisabled>,
    )>,
    q_thumb: Query<&ComputedNode, With<CoreSliderThumb>>,
    q_children: Query<&Children>,
    mut commands: Commands,
    ui_scale: Res<UiScale>,
) {
    if q_thumb.contains(trigger.target()) {
        // Thumb click, stop propagation to prevent track click.
        trigger.propagate(false);
    } else if let Ok((
        slider,
        value,
        range,
        step,
        precision,
        node,
        node_target,
        transform,
        disabled,
    )) = q_slider.get(trigger.target())
    {
        // Track click
        trigger.propagate(false);

        if disabled {
            return;
        }

        // Find thumb size by searching descendants for the first entity with CoreSliderThumb
        let thumb_size = q_children
            .iter_descendants(trigger.target())
            .find_map(|child_id| q_thumb.get(child_id).ok().map(|thumb| thumb.size().x))
            .unwrap_or(0.0);

        // Detect track click.
        let local_pos = transform.try_inverse().unwrap().transform_point2(
            trigger.event().pointer_location.position * node_target.scale_factor() / ui_scale.0,
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
            TrackClick::Snap => precision
                .map(|prec| prec.round(click_val))
                .unwrap_or(click_val),
        });

        if matches!(slider.on_change, Callback::Ignore) {
            commands
                .entity(trigger.target())
                .insert(SliderValue(new_value));
        } else {
            commands.notify_with(
                &slider.on_change,
                ValueChange {
                    source: trigger.target(),
                    value: new_value,
                },
            );
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
    if let Ok((value, mut drag, disabled)) = q_slider.get_mut(trigger.target()) {
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
        Option<&SliderPrecision>,
        &UiGlobalTransform,
        &mut CoreSliderDragState,
        Has<InteractionDisabled>,
    )>,
    q_thumb: Query<&ComputedNode, With<CoreSliderThumb>>,
    q_children: Query<&Children>,
    mut commands: Commands,
    ui_scale: Res<UiScale>,
) {
    if let Ok((node, slider, range, precision, transform, drag, disabled)) =
        q_slider.get_mut(trigger.target())
    {
        trigger.propagate(false);
        if drag.dragging && !disabled {
            let mut distance = trigger.event().distance / ui_scale.0;
            distance.y *= -1.;
            let distance = transform.transform_vector2(distance);
            // Find thumb size by searching descendants for the first entity with CoreSliderThumb
            let thumb_size = q_children
                .iter_descendants(trigger.target())
                .find_map(|child_id| q_thumb.get(child_id).ok().map(|thumb| thumb.size().x))
                .unwrap_or(0.0);
            let slider_width = ((node.size().x - thumb_size) * node.inverse_scale_factor).max(1.0);
            let span = range.span();
            let new_value = if span > 0. {
                drag.offset + (distance.x * span) / slider_width
            } else {
                range.start() + span * 0.5
            };
            let rounded_value = range.clamp(
                precision
                    .map(|prec| prec.round(new_value))
                    .unwrap_or(new_value),
            );

            if matches!(slider.on_change, Callback::Ignore) {
                commands
                    .entity(trigger.target())
                    .insert(SliderValue(rounded_value));
            } else {
                commands.notify_with(
                    &slider.on_change,
                    ValueChange {
                        source: trigger.target(),
                        value: rounded_value,
                    },
                );
            }
        }
    }
}

pub(crate) fn slider_on_drag_end(
    mut trigger: On<Pointer<DragEnd>>,
    mut q_slider: Query<(&CoreSlider, &mut CoreSliderDragState)>,
) {
    if let Ok((_slider, mut drag)) = q_slider.get_mut(trigger.target()) {
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
    if let Ok((slider, value, range, step, disabled)) = q_slider.get(trigger.target()) {
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
            if matches!(slider.on_change, Callback::Ignore) {
                commands
                    .entity(trigger.target())
                    .insert(SliderValue(new_value));
            } else {
                commands.notify_with(
                    &slider.on_change,
                    ValueChange {
                        source: trigger.target(),
                        value: new_value,
                    },
                );
            }
        }
    }
}

pub(crate) fn slider_on_insert(trigger: On<Insert, CoreSlider>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target());
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_orientation(Orientation::Horizontal);
    }
}

pub(crate) fn slider_on_insert_value(trigger: On<Insert, SliderValue>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target());
    let value = entity.get::<SliderValue>().unwrap().0;
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_numeric_value(value.into());
    }
}

pub(crate) fn slider_on_insert_range(trigger: On<Insert, SliderRange>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target());
    let range = *entity.get::<SliderRange>().unwrap();
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_min_numeric_value(range.start().into());
        accessibility.set_max_numeric_value(range.end().into());
    }
}

pub(crate) fn slider_on_insert_step(trigger: On<Insert, SliderStep>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target());
    let step = entity.get::<SliderStep>().unwrap().0;
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_numeric_value_step(step.into());
    }
}

/// An [`EntityEvent`] that can be triggered on a slider to modify its value (using the `on_change` callback).
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
///         SliderRange::new(0.0, 1.0),
///     )).id();
///
///     // Set to an absolute value
///     commands.trigger_targets(SetSliderValue::Absolute(0.75), slider);
///
///     // Adjust relatively
///     commands.trigger_targets(SetSliderValue::Relative(-0.25), slider);
/// }
/// ```
#[derive(EntityEvent, Clone)]
pub enum SetSliderValue {
    /// Set the slider value to a specific value.
    Absolute(f32),
    /// Add a delta to the slider value.
    Relative(f32),
    /// Add a delta to the slider value, multiplied by the step size.
    RelativeStep(f32),
}

fn slider_on_set_value(
    mut trigger: On<SetSliderValue>,
    q_slider: Query<(&CoreSlider, &SliderValue, &SliderRange, Option<&SliderStep>)>,
    mut commands: Commands,
) {
    if let Ok((slider, value, range, step)) = q_slider.get(trigger.target()) {
        trigger.propagate(false);
        let new_value = match trigger.event() {
            SetSliderValue::Absolute(new_value) => range.clamp(*new_value),
            SetSliderValue::Relative(delta) => range.clamp(value.0 + *delta),
            SetSliderValue::RelativeStep(delta) => {
                range.clamp(value.0 + *delta * step.map(|s| s.0).unwrap_or_default())
            }
        };
        if matches!(slider.on_change, Callback::Ignore) {
            commands
                .entity(trigger.target())
                .insert(SliderValue(new_value));
        } else {
            commands.notify_with(
                &slider.on_change,
                ValueChange {
                    source: trigger.target(),
                    value: new_value,
                },
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_precision_rounding() {
        // Test positive precision values (decimal places)
        let precision_2dp = SliderPrecision(2);
        assert_eq!(precision_2dp.round(1.234567), 1.23);
        assert_eq!(precision_2dp.round(1.235), 1.24);

        // Test zero precision (rounds to integers)
        let precision_0dp = SliderPrecision(0);
        assert_eq!(precision_0dp.round(1.4), 1.0);

        // Test negative precision (rounds to tens, hundreds, etc.)
        let precision_neg1 = SliderPrecision(-1);
        assert_eq!(precision_neg1.round(14.0), 10.0);
    }
}
