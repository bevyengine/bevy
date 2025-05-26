#[cfg(feature = "bevy-support")]
use bevy_ecs::component::Component;

#[cfg(feature = "bevy_reflect")]
use {bevy_ecs::reflect::ReflectComponent, bevy_reflect::prelude::*};

/// An optimization for transform propagation. This ZST marker component uses change detection to
/// mark all entities of the hierarchy as "dirty" if any of their descendants have a changed
/// `Transform3d`. If this component is *not* marked `is_changed()`, propagation will halt.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy-support", derive(Component))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, PartialEq, Debug)
)]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize)
)]
pub struct TransformTreeChanged;
