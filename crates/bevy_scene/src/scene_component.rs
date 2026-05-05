use bevy_ecs::{component::Component, reflect::ReflectComponent, template::FromTemplate};
use bevy_reflect::Reflect;

use crate::Scene;

/// Implemented for [`Component`]s that have an associated [`Scene`], which can be constructed
/// with [`Self::Props`].
///
/// In general, developers should not implement this manually. Instead, they should derive it,
/// which also derives [`Component`] and adds additional protections and assurances.
///
/// See the "Scene Components" sections of the [`Scene`] docs to see how this is used in practice.
pub trait SceneComponent: Component + FromTemplate<Template: Default> {
    /// The "properties" passed into [`Self::scene`] to build the final scene.
    type Props: Default;

    /// A function that uses the given `props` to produce a [`Scene`]
    fn scene(props: Self::Props) -> impl Scene;
}

/// Indicates that this entity includes a [`Component`] that must always be spawned with a [`Scene`].
#[derive(Component, Default, Clone, Debug, Reflect)]
#[cfg_attr(debug_assertions, component(on_add))]
#[reflect(Component)]
pub struct SceneComponentInfo {
    spawned_from_scene: bool,
    #[cfg(debug_assertions)]
    component_name: &'static str,
}

impl SceneComponentInfo {
    /// Creates a new [`SceneComponentInfo`] for the given type `C`.
    pub fn new<C: Component>(spawned_from_scene: bool) -> Self {
        SceneComponentInfo {
            spawned_from_scene,
            #[cfg(debug_assertions)]
            component_name: core::any::type_name::<C>(),
        }
    }
}

impl SceneComponentInfo {
    #[cfg(debug_assertions)]
    fn on_add(world: bevy_ecs::world::DeferredWorld, context: bevy_ecs::lifecycle::HookContext) {
        if let Ok(entity) = world.get_entity(context.entity)
            && let Some(component) = entity.get::<SceneComponentInfo>()
            && !component.spawned_from_scene
        {
            tracing::error!(
                "Entity {} was spawned with the \"scene component\" {}, but without its scene. \
                Scene components should not be spawned directly as components. Instead, they \
                should be spawned as \"scenes\" using world.spawn_scene or commands.spawn_scene. \
                Scene components should be inherited using `:{}` syntax in BSN.",
                context.entity,
                component.component_name,
                component.component_name
            );
        }
    }
}
