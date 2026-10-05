#[expect(
    deprecated,
    reason = "Should be removed after 0.20 is released when Interaction is removed."
)]
use crate::Interaction;
use crate::{FocusPolicy, Node};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

#[expect(
    deprecated,
    reason = "Should be removed after 0.20 is released when Interaction is removed."
)]
type InteractionTemp = Interaction;

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Node, FocusPolicy::Block)]
#[require(InteractionTemp)]
// TODO this must be removed in 0.20
pub(crate) struct DeprecatedButton;

#[deprecated(since = "0.20.0", note = "Use ui_widgets::Button.")]
#[expect(
    private_interfaces,
    reason = "We have to use a type alias here to deprecate `Button` because\
              deprecating the `DeprecatedButton` struct would cause a lint that is not `expect`able."
)]
pub type Button = DeprecatedButton;
