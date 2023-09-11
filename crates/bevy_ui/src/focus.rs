use crate::{camera_config::UiCameraConfig, CalculatedClip, Node, UiScale, UiStack};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    prelude::{Component, With},
    query::{Changed, Or, WorldQuery},
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::{camera::NormalizedRenderTarget, prelude::Camera, view::ViewVisibility};
use bevy_transform::components::GlobalTransform;

use bevy_window::{PrimaryWindow, Window};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Describes if a UI node has been pressed.
///
/// Updated in [`ui_focus_system`].
///
/// If a UI node has both [`Pressed`] and [`ViewVisibility`] components,
/// the node will be considered not pressed
/// when [`ViewVisibility::get()`] is false.
/// This ensures that hidden UI nodes are not interactable,
/// and do not end up stuck in an active state if hidden at the wrong time.
///
/// Note that you can also control the visibility of a node using the [`Display`](crate::ui_node::Display) property,
/// which fully collapses it during layout calculations.
#[derive(
    Component, Copy, Clone, Eq, PartialEq, Debug, Default, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub struct Pressed {
    /// Is the node currently pressed
    pub pressed: bool,
    /// Describes whether the component should remain in the pressed state after
    /// the cursor stops hovering over the node.
    pub press_policy: PressPolicy,
}

impl Pressed {
    pub fn new(press_policy: PressPolicy) -> Self {
        Self {
            pressed: false,
            press_policy,
        }
    }
}

/// Describes whether the [`Pressed`] component should remain in the pressed state after
/// the cursor stops hovering over the node.
///
/// When the user clicks this node and the `PressPolicy` is set to `Hold`, the `Pressed` component will remain in the clicked state after the cursor leaves the node, until the user releases the interaction button. (the default behaviour)
///
/// If instead the `PressPolicy` is set to `Release`, the `Pressed` component will be considered not clicked
/// as soon as the cursor leaves the node, even if the user still is pressing down the interaction button.
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, PartialEq)]
pub enum PressPolicy {
    /// Keep the node clicked after it stopped being hovered
    #[default]
    Hold,
    /// Release the node if the cursor stops hovering
    Release,
}

/// A component storing the position of the mouse relative to the node, (0., 0.) being the top-left corner and (1., 1.) being the bottom-right
/// If the mouse is not over the node, the value will go beyond the range of (0., 0.) to (1., 1.)
/// A None value means that the cursor position is unknown.
///
/// It can be used alongside interaction to get the position of the press.
#[derive(
    Component,
    Deref,
    DerefMut,
    Copy,
    Clone,
    Default,
    PartialEq,
    Debug,
    Reflect,
    Serialize,
    Deserialize,
)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub struct RelativeCursorPosition {
    /// Cursor position relative to size and position of the Node.
    pub normalized: Option<Vec2>,
}

impl RelativeCursorPosition {
    /// A helper function to check if the mouse is over the node
    pub fn mouse_over(&self) -> bool {
        self.normalized
            .map(|position| (0.0..1.).contains(&position.x) && (0.0..1.).contains(&position.y))
            .unwrap_or(false)
    }
}

/// A simplified interaction state calculated using the [`Pressed`] and [`RelativeCursorPosition`] components.
///
/// To see how to use this, see the [`InteractionStateHandler`] trait.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Reflect)]
pub enum InteractionState {
    None,
    Hovered,
    Pressed,
}

/// Simplified way to get the [`InteractionState`] of the node.
///
/// Example usage:
/// ```rust
/// use bevy_ecs::system::Query;
/// use bevy_ui::InteractionStateHandler;
/// use bevy_ui::InteractionState;
/// use bevy_ui::Pressed;
/// use bevy_ui::RelativeCursorPosition;
///
/// fn button_system(button_query: Query<(&Pressed, &RelativeCursorPosition)>) {
///     let button = button_query.single();
///
///     match button.interaction_state() {
///         InteractionState::None => (),
///         InteractionState::Hovered => {
///             println!("The button is being hovered over");
///         },
///         InteractionState::Pressed => {
///             println!("The button is being pressed");
///         },
///    }
/// }
/// ```
pub trait InteractionStateHandler {
    fn interaction_state(&self) -> InteractionState;
}

impl InteractionStateHandler for (&Pressed, &RelativeCursorPosition) {
    /// Get the [`InteractionState`] of the node
    fn interaction_state(&self) -> InteractionState {
        if self.0.pressed {
            return InteractionState::Pressed;
        }

        if self.1.mouse_over() {
            return InteractionState::Hovered;
        }

        InteractionState::None
    }
}

pub type InteractionStateChangedFilter = Or<(Changed<Pressed>, Changed<RelativeCursorPosition>)>;

/// Describes whether the node should block interactions with lower nodes
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub enum FocusPolicy {
    /// Blocks interaction
    Block,
    /// Lets interaction pass through
    Pass,
}

impl FocusPolicy {
    const DEFAULT: Self = Self::Pass;
}

impl Default for FocusPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Contains entities whose Interaction should be set to None
#[derive(Default)]
pub struct State {
    entities_to_reset: SmallVec<[Entity; 1]>,
}

/// Main query for [`ui_focus_system`]
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NodeQuery {
    entity: Entity,
    node: &'static Node,
    global_transform: &'static GlobalTransform,
    pressed_state: Option<&'static mut Pressed>,
    relative_cursor_position: Option<&'static mut RelativeCursorPosition>,
    focus_policy: Option<&'static FocusPolicy>,
    calculated_clip: Option<&'static CalculatedClip>,
    view_visibility: Option<&'static ViewVisibility>,
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
///
/// Entities with a hidden [`ViewVisibility`] are always treated as released.
#[allow(clippy::too_many_arguments)]
pub fn ui_focus_system(
    mut state: Local<State>,
    camera: Query<(&Camera, Option<&UiCameraConfig>)>,
    windows: Query<&Window>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    ui_scale: Res<UiScale>,
    ui_stack: Res<UiStack>,
    mut node_query: Query<NodeQuery>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let primary_window = primary_window.iter().next();

    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(mut pressed_state) = node_query.get_component_mut::<Pressed>(entity) {
            pressed_state.pressed = false;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();
    if mouse_released {
        for node in node_query.iter_mut() {
            if let Some(mut pressed_state) = node.pressed_state {
                if pressed_state.pressed {
                    pressed_state.pressed = false;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.any_just_pressed();

    let is_ui_disabled =
        |camera_ui| matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. }));

    let cursor_position = camera
        .iter()
        .filter(|(_, camera_ui)| !is_ui_disabled(*camera_ui))
        .filter_map(|(camera, _)| {
            if let Some(NormalizedRenderTarget::Window(window_ref)) =
                camera.target.normalize(primary_window)
            {
                Some(window_ref)
            } else {
                None
            }
        })
        .find_map(|window_ref| {
            windows
                .get(window_ref.entity())
                .ok()
                .and_then(|window| window.cursor_position())
        })
        .or_else(|| touches_input.first_pressed_position())
        // The cursor position returned by `Window` only takes into account the window scale factor and not `UiScale`.
        // To convert the cursor position to logical UI viewport coordinates we have to divide it by `UiScale`.
        .map(|cursor_position| cursor_position / ui_scale.0 as f32);

    // prepare an iterator that contains all the nodes that have the cursor in their rect,
    // from the top node to the bottom one. this will also reset the interaction to `None`
    // for all nodes encountered that are no longer hovered.
    let mut hovered_nodes = ui_stack
        .uinodes
        .iter()
        // reverse the iterator to traverse the tree from closest nodes to furthest
        .rev()
        .filter_map(|entity| {
            if let Ok(node) = node_query.get_mut(*entity) {
                // Nodes that are not rendered should not be interactable
                if let Some(view_visibility) = node.view_visibility {
                    if !view_visibility.get() {
                        // Reset their interaction to None to avoid strange stuck state
                        if let Some(mut pressed_state) = node.pressed_state {
                            // We cannot simply set the interaction to None, as that will trigger change detection repeatedly
                            pressed_state.set_if_neq(Pressed {
                                pressed: false,
                                ..*pressed_state
                            });
                        }

                        return None;
                    }
                }

                let position = node.global_transform.translation();
                let ui_position = position.truncate();
                let extents = node.node.size() / 2.0;
                let mut min = ui_position - extents;
                if let Some(clip) = node.calculated_clip {
                    min = Vec2::max(min, clip.clip.min);
                }

                // The mouse position relative to the node
                // (0., 0.) is the top-left corner, (1., 1.) is the bottom-right corner
                let relative_cursor_position = cursor_position
                    .map(|cursor_position| (cursor_position - min) / node.node.size());

                // If the current cursor position is within the bounds of the node, consider it for
                // clicking
                let relative_cursor_position_component = RelativeCursorPosition {
                    normalized: relative_cursor_position,
                };

                let contains_cursor = relative_cursor_position_component.mouse_over();

                // Save the relative cursor position to the correct component
                if let Some(mut node_relative_cursor_position_component) =
                    node.relative_cursor_position
                {
                    *node_relative_cursor_position_component = relative_cursor_position_component;
                }

                if contains_cursor {
                    Some(*entity)
                } else {
                    if let Some(mut pressed_state) = node.pressed_state {
                        // If the InteractionPolicy is Release, we should set the interaction to None
                        // The entity might just as well not have the InteractionPolicy component
                        // in which case we should use the default behaviour
                        let interaction_policy = pressed_state.press_policy;

                        if cursor_position.is_none() || interaction_policy == PressPolicy::Release {
                            pressed_state.set_if_neq(Pressed {
                                pressed: false,
                                ..*pressed_state
                            });
                        }
                    }
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<Entity>>()
        .into_iter();

    // set Pressed or Hovered on top nodes. as soon as a node with a `Block` focus policy is detected,
    // the iteration will stop on it because it "captures" the interaction.
    let mut iter = node_query.iter_many_mut(hovered_nodes.by_ref());
    while let Some(node) = iter.fetch_next() {
        if let Some(mut pressed_state) = node.pressed_state {
            if mouse_clicked && !pressed_state.pressed {
                pressed_state.pressed = true;
                // if the mouse was simultaneously released, reset this Interaction in the next
                // frame
                if mouse_released {
                    state.entities_to_reset.push(node.entity);
                }
            }
        }

        match node.focus_policy.unwrap_or(&FocusPolicy::Block) {
            FocusPolicy::Block => {
                break;
            }
            FocusPolicy::Pass => { /* allow the next node to be hovered/pressed */ }
        }
    }
    // reset `Interaction` for the remaining lower nodes to `None`. those are the nodes that remain in
    // `moused_over_nodes` after the previous loop is exited.
    let mut iter = node_query.iter_many_mut(hovered_nodes);
    while let Some(node) = iter.fetch_next() {
        if let Some(mut pressed_state) = node.pressed_state {
            // don't reset clicked nodes because they're handled separately
            if !pressed_state.pressed {
                pressed_state.set_if_neq(Pressed {
                    pressed: false,
                    ..*pressed_state
                });
            }
        }
    }
}
