use crate::{CalculatedClip, DefaultUiCamera, Node, TargetCamera, UiScale, UiStack};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    prelude::{Component, With},
    query::QueryData,
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, ButtonInput};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{camera::NormalizedRenderTarget, prelude::Camera, view::ViewVisibility};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window};

use smallvec::SmallVec;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Describes what type of input interaction has occurred for a UI node.
///
/// This is commonly queried with a `Changed<Interaction>` filter.
///
/// Updated in [`ui_focus_system`].
///
/// If a UI node has both [`Interaction`] and [`ViewVisibility`] components,
/// [`Interaction`] will always be [`Interaction::None`]
/// when [`ViewVisibility::get()`] is false.
/// This ensures that hidden UI nodes are not interactable,
/// and do not end up stuck in an active state if hidden at the wrong time.
///
/// Note that you can also control the visibility of a node using the [`Display`](crate::ui_node::Display) property,
/// which fully collapses it during layout calculations.
///
/// # See also
///
/// - [`ButtonBundle`](crate::node_bundles::ButtonBundle) which includes this component
/// - [`RelativeCursorPosition`] to obtain the position of the cursor relative to current node
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum Interaction {
    /// The node has been pressed.
    ///
    /// Note: This does not capture click/press-release action.
    Pressed,
    /// The node has been hovered over
    Hovered,
    /// Nothing has happened
    None,
}

impl Interaction {
    const DEFAULT: Self = Self::None;
}

impl Default for Interaction {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A component storing the position of the mouse relative to the node, (0., 0.) being the top-left corner and (1., 1.) being the bottom-right
/// If the mouse is not over the node, the value will go beyond the range of (0., 0.) to (1., 1.)
///
/// It can be used alongside [`Interaction`] to get the position of the press.
///
/// The component is updated when it is in the same entity with [`Node`].
#[derive(Component, Copy, Clone, Default, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RelativeCursorPosition {
    /// Visible area of the Node relative to the size of the entire Node.
    pub normalized_visible_node_rect: Rect,
    /// Cursor position relative to the size and position of the Node.
    /// A None value indicates that the cursor position is unknown.
    pub normalized: Option<Vec2>,
}

impl RelativeCursorPosition {
    /// A helper function to check if the mouse is over the node
    pub fn mouse_over(&self) -> bool {
        self.normalized
            .map(|position| self.normalized_visible_node_rect.contains(position))
            .unwrap_or(false)
    }
}

/// Describes whether the node should block interactions with lower nodes
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
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
#[derive(QueryData)]
#[query_data(mutable)]
pub struct NodeQuery {
    entity: Entity,
    node: &'static Node,
    global_transform: &'static GlobalTransform,
    interaction: Option<&'static mut Interaction>,
    relative_cursor_position: Option<&'static mut RelativeCursorPosition>,
    focus_policy: Option<&'static FocusPolicy>,
    calculated_clip: Option<&'static CalculatedClip>,
    view_visibility: Option<&'static ViewVisibility>,
    target_camera: Option<&'static TargetCamera>,
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
///
/// Entities with a hidden [`ViewVisibility`] are always treated as released.
#[allow(clippy::too_many_arguments)]
pub fn ui_focus_system(
    mut state: Local<State>,
    camera_query: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    touches_input: Res<Touches>,
    ui_scale: Res<UiScale>,
    ui_stack: Res<UiStack>,
    mut node_query: Query<NodeQuery>,
) {
    let primary_window = primary_window.iter().next();

    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(NodeQueryItem {
            interaction: Some(mut interaction),
            ..
        }) = node_query.get_mut(entity)
        {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();
    if mouse_released {
        for node in &mut node_query {
            if let Some(mut interaction) = node.interaction {
                if *interaction == Interaction::Pressed {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.any_just_pressed();

    let camera_cursor_positions: HashMap<Entity, Vec2> = camera_query
        .iter()
        .filter_map(|(entity, camera)| {
            // Interactions are only supported for cameras rendering to a window.
            let Some(NormalizedRenderTarget::Window(window_ref)) =
                camera.target.normalize(primary_window)
            else {
                return None;
            };

            let viewport_position = camera
                .logical_viewport_rect()
                .map(|rect| rect.min)
                .unwrap_or_default();
            windows
                .get(window_ref.entity())
                .ok()
                .and_then(|window| window.cursor_position())
                .or_else(|| touches_input.first_pressed_position())
                .map(|cursor_position| (entity, cursor_position - viewport_position))
        })
        // The cursor position returned by `Window` only takes into account the window scale factor and not `UiScale`.
        // To convert the cursor position to logical UI viewport coordinates we have to divide it by `UiScale`.
        .map(|(entity, cursor_position)| (entity, cursor_position / ui_scale.0))
        .collect();

    // prepare an iterator that contains all the nodes that have the cursor in their rect,
    // from the top node to the bottom one. this will also reset the interaction to `None`
    // for all nodes encountered that are no longer hovered.
    let mut hovered_nodes = ui_stack
        .uinodes
        .iter()
        // reverse the iterator to traverse the tree from closest nodes to furthest
        .rev()
        .filter_map(|entity| {
            let Ok(node) = node_query.get_mut(*entity) else {
                return None;
            };

            let view_visibility = node.view_visibility?;
            // Nodes that are not rendered should not be interactable
            if !view_visibility.get() {
                // Reset their interaction to None to avoid strange stuck state
                if let Some(mut interaction) = node.interaction {
                    // We cannot simply set the interaction to None, as that will trigger change detection repeatedly
                    interaction.set_if_neq(Interaction::None);
                }
                return None;
            }
            let camera_entity = node
                .target_camera
                .map(TargetCamera::entity)
                .or(default_ui_camera.get())?;

            let node_rect = node.node.logical_rect(node.global_transform);

            // Intersect with the calculated clip rect to find the bounds of the visible region of the node
            let visible_rect = node
                .calculated_clip
                .map(|clip| node_rect.intersect(clip.clip))
                .unwrap_or(node_rect);

            // The mouse position relative to the node
            // (0., 0.) is the top-left corner, (1., 1.) is the bottom-right corner
            // Coordinates are relative to the entire node, not just the visible region.
            let relative_cursor_position =
                camera_cursor_positions
                    .get(&camera_entity)
                    .and_then(|cursor_position| {
                        // ensure node size is non-zero in all dimensions, otherwise relative position will be
                        // +/-inf. if the node is hidden, the visible rect min/max will also be -inf leading to
                        // false positives for mouse_over (#12395)
                        (node_rect.size().cmpgt(Vec2::ZERO).all())
                            .then_some((*cursor_position - node_rect.min) / node_rect.size())
                    });

            // If the current cursor position is within the bounds of the node's visible area, consider it for
            // clicking
            let relative_cursor_position_component = RelativeCursorPosition {
                normalized_visible_node_rect: visible_rect.normalize(node_rect),
                normalized: relative_cursor_position,
            };

            let contains_cursor = relative_cursor_position_component.mouse_over();

            // Save the relative cursor position to the correct component
            if let Some(mut node_relative_cursor_position_component) = node.relative_cursor_position
            {
                *node_relative_cursor_position_component = relative_cursor_position_component;
            }

            if contains_cursor {
                Some(*entity)
            } else {
                if let Some(mut interaction) = node.interaction {
                    if *interaction == Interaction::Hovered || (relative_cursor_position.is_none())
                    {
                        interaction.set_if_neq(Interaction::None);
                    }
                }
                None
            }
        })
        .collect::<Vec<Entity>>()
        .into_iter();

    // set Pressed or Hovered on top nodes. as soon as a node with a `Block` focus policy is detected,
    // the iteration will stop on it because it "captures" the interaction.
    let mut iter = node_query.iter_many_mut(hovered_nodes.by_ref());
    while let Some(node) = iter.fetch_next() {
        if let Some(mut interaction) = node.interaction {
            if mouse_clicked {
                // only consider nodes with Interaction "pressed"
                if *interaction != Interaction::Pressed {
                    *interaction = Interaction::Pressed;
                    // if the mouse was simultaneously released, reset this Interaction in the next
                    // frame
                    if mouse_released {
                        state.entities_to_reset.push(node.entity);
                    }
                }
            } else if *interaction == Interaction::None {
                *interaction = Interaction::Hovered;
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
        if let Some(mut interaction) = node.interaction {
            // don't reset pressed nodes because they're handled separately
            if *interaction != Interaction::Pressed {
                interaction.set_if_neq(Interaction::None);
            }
        }
    }
}
