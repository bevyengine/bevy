use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
/// Additional untyped data that can be present on most glTF types at the scene level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfSceneExtras {
    /// Content of the extra data.
    pub value: String,
}
