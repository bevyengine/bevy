use crate::{
    ui_transform::UiGlobalTransform, ComputedNode, ComputedNodeTarget, Node, OverrideClip, UiStack,
};
use bevy_camera::{visibility::InheritedVisibility, Camera, NormalizedRenderTarget};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::{ContainsEntity, Entity},
    hierarchy::ChildOf,
    prelude::{Component, With},
    query::{QueryData, Without},
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, ButtonInput};
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
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
/// If a UI node has both [`Interaction`] and [`InheritedVisibility`] components,
/// [`Interaction`] will always be [`Interaction::None`]
/// when [`InheritedVisibility::get()`] is false.
/// This ensures that hidden UI nodes are not interactable,
/// and do not end up stuck in an active state if hidden at the wrong time.
///
/// Note that you can also control the visibility of a node using the [`Display`](crate::ui_node::Display) property,
/// which fully collapses it during layout calculations.
///
/// # See also
///
/// - [`Button`](crate::widget::Button) which requires this component
/// - [`RelativeCursorPosition`] to obtain the position of the cursor relative to current node
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
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

/// A component storing the position of the mouse relative to the node, (0., 0.) being the center and (0.5, 0.5) being the bottom-right
/// If the mouse is not over the node, the value will go beyond the range of (-0.5, -0.5) to (0.5, 0.5)
///
/// It can be used alongside [`Interaction`] to get the position of the press.
///
/// The component is updated when it is in the same entity with [`Node`].
#[derive(Component, Copy, Clone, Default, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RelativeCursorPosition {
    /// True if the cursor position is over an unclipped area of the Node.
    pub cursor_over: bool,
    /// Cursor position relative to the size and position of the Node.
    /// A None value indicates that the cursor position is unknown.
    pub normalized: Option<Vec2>,
}

impl RelativeCursorPosition {
    /// A helper function to check if the mouse is over the node
    pub fn cursor_over(&self) -> bool {
        self.cursor_over
    }
}

/// Describes whether the node should block interactions with lower nodes
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
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
    node: &'static ComputedNode,
    transform: &'static UiGlobalTransform,
    interaction: Option<&'static mut Interaction>,
    relative_cursor_position: Option<&'static mut RelativeCursorPosition>,
    focus_policy: Option<&'static FocusPolicy>,
    inherited_visibility: Option<&'static InheritedVisibility>,
    target_camera: &'static ComputedNodeTarget,
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
///
/// Entities with a hidden [`InheritedVisibility`] are always treated as released.
pub fn ui_focus_system(
    mut state: Local<State>,
    camera_query: Query<(Entity, &Camera)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    touches_input: Res<Touches>,
    ui_stack: Res<UiStack>,
    mut node_query: Query<NodeQuery>,
    clipping_query: Query<(&ComputedNode, &UiGlobalTransform, &Node)>,
    child_of_query: Query<&ChildOf, Without<OverrideClip>>,
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
            if let Some(mut interaction) = node.interaction
                && *interaction == Interaction::Pressed
            {
                *interaction = Interaction::None;
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
            let window = windows.get(window_ref.entity()).ok()?;

            let viewport_position = camera
                .physical_viewport_rect()
                .map(|rect| rect.min.as_vec2())
                .unwrap_or_default();
            window
                .physical_cursor_position()
                .or_else(|| {
                    touches_input
                        .first_pressed_position()
                        .map(|pos| pos * window.scale_factor())
                })
                .map(|cursor_position| (entity, cursor_position - viewport_position))
        })
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

            let inherited_visibility = node.inherited_visibility?;
            // Nodes that are not rendered should not be interactable
            if !inherited_visibility.get() {
                // Reset their interaction to None to avoid strange stuck state
                if let Some(mut interaction) = node.interaction {
                    // We cannot simply set the interaction to None, as that will trigger change detection repeatedly
                    interaction.set_if_neq(Interaction::None);
                }
                return None;
            }
            let camera_entity = node.target_camera.camera()?;

            let cursor_position = camera_cursor_positions.get(&camera_entity);

            let contains_cursor = cursor_position.is_some_and(|point| {
                node.node.contains_point(*node.transform, *point)
                    && clip_check_recursive(*point, *entity, &clipping_query, &child_of_query)
            });

            // The mouse position relative to the node
            // (-0.5, -0.5) is the top-left corner, (0.5, 0.5) is the bottom-right corner
            // Coordinates are relative to the entire node, not just the visible region.
            let normalized_cursor_position = cursor_position.and_then(|cursor_position| {
                // ensure node size is non-zero in all dimensions, otherwise relative position will be
                // +/-inf. if the node is hidden, the visible rect min/max will also be -inf leading to
                // false positives for mouse_over (#12395)
                node.node.normalize_point(*node.transform, *cursor_position)
            });

            // If the current cursor position is within the bounds of the node's visible area, consider it for
            // clicking
            let relative_cursor_position_component = RelativeCursorPosition {
                cursor_over: contains_cursor,
                normalized: normalized_cursor_position,
            };

            // Save the relative cursor position to the correct component
            if let Some(mut node_relative_cursor_position_component) = node.relative_cursor_position
            {
                // Avoid triggering change detection when not necessary.
                node_relative_cursor_position_component
                    .set_if_neq(relative_cursor_position_component);
            }

            if contains_cursor {
                Some(*entity)
            } else {
                if let Some(mut interaction) = node.interaction
                    && (*interaction == Interaction::Hovered
                        || (normalized_cursor_position.is_none()))
                {
                    interaction.set_if_neq(Interaction::None);
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

/// Walk up the tree child-to-parent checking that `point` is not clipped by any ancestor node.
/// If `entity` has an [`OverrideClip`] component it ignores any inherited clipping and returns true.
pub fn clip_check_recursive(
    point: Vec2,
    entity: Entity,
    clipping_query: &Query<'_, '_, (&ComputedNode, &UiGlobalTransform, &Node)>,
    child_of_query: &Query<&ChildOf, Without<OverrideClip>>,
) -> bool {
    if let Ok(child_of) = child_of_query.get(entity) {
        let parent = child_of.0;
        if let Ok((computed_node, transform, node)) = clipping_query.get(parent)
            && !computed_node
                .resolve_clip_rect(node.overflow, node.overflow_clip_margin)
                .contains(transform.inverse().transform_point2(point))
        {
            // The point is clipped and should be ignored by picking
            return false;
        }
        return clip_check_recursive(point, parent, clipping_query, child_of_query);
    }
    // Reached root, point unclipped by all ancestors
    true
}
