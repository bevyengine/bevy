use crate::{FocusPolicy, Interaction, Node};
use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Unified button component that combines UI layout, interaction handling, focus management,
/// and accessibility support.
///
/// This component automatically includes all necessary components for a functional button
/// through the `#[require]` attribute:
/// - [`Node`]: For UI layout
/// - [`FocusPolicy::Block`]: For focus management
/// - [`Interaction`]: For tracking interaction state
/// - [`AccessibilityNode`]: For screen reader support
///
/// # Interactive Behavior
///
/// To enable full interactive behavior (pressed state, activation events), you need to:
/// 1. Add the [`bevy_ui_widgets::ButtonPlugin`] to your app
/// 2. Listen for [`bevy_ui_widgets::Activate`] events
/// ```
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(
    Node,
    FocusPolicy::Block,
    Interaction,
    AccessibilityNode(accesskit::Node::new(Role::Button))
)]
pub struct Button;
