use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::{bundle::Bundle, component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use super::{Event, WidgetBundle};

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Node, FocusPolicy::Block, Interaction)]
pub struct Button;

#[derive(Bundle)]
pub struct ButtonBundle {
    pub base: WidgetBundle,
}

/// Create a button widget with default styling
pub fn make_button<T: Event>(base_bundle: WidgetBundle) -> ButtonBundle {
    ButtonBundle { base: base_bundle }
}
