use bevy_ecs::{entity::Entity, name::Name};

use smallvec::SmallVec;

// A helper structure for `load_node` that contains information about the
// nearest ancestor animation root.
#[cfg(feature = "bevy_animation")]
#[derive(Clone)]
pub struct AnimationContext {
    // The nearest ancestor animation root.
    pub root: Entity,
    // The path to the animation root. This is used for constructing the
    // animation target UUIDs.
    pub path: SmallVec<[Name; 8]>,
}
