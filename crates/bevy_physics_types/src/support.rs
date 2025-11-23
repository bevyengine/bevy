use bevy_ecs::component::Component;

/// tells if engine supports a thing.
trait Capabilities {
    fn supports<T : Component>() -> bool;
}