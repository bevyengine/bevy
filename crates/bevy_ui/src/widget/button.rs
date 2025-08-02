use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

// the #[reflect] attribute most likely comes from bevy_reflect
//
// understanding what #[reflect()] is doing in this scenario.
//
// Fields can be given default values for when a field is missing in the passed value or even ignored. Ignored fields must either implement Default or have a default function specified using #[reflect(default = "path::to::function")].
//
// In essence, #[reflect()] seems to represent that the traits that should be included by default
//
// #[derive(Reflect)] automatically generates the basic Reflect trait implementation for the Button struct.
// #[reflect(some_traits)] --> This part explicitly registers these traits as being reflected. This means Bevy will be aware that Button implements these traits dynamically. For example, ReflectComponent allows Bevy to dynamically work with the Button struct as a component
//
// understanding what it means for a trait to be implemented dynamically:
// In rust, a trait being implemented "dynamically" means that the specific implementation of the trait's method is determined at runtime --> rather than compile time (the default is static implementation)
/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Node, FocusPolicy::Block, Interaction)]
pub struct Button;
