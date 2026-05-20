use crate::component::{Component, RestrictedMutable};

/// Marks a component whose mutable access must be mediated by
/// [`RestrictedMut`](crate::system::RestrictedMut).
///
/// Use cases include audit trails, save systems, undo/redo, time travel,
/// debugging, and multi-process replication.
///
/// `#[derive(RestrictedAccess)]` also implements [`Component`] with restricted
/// mutability. Do not combine it with `#[derive(Component)]` on the same type.
pub trait RestrictedAccess: Component<Mutability = RestrictedMutable> {}

#[cfg(test)]
mod tests {
    use crate::{
        component::RestrictedAccess,
        world::World,
    };

    #[test]
    fn restricted_access_derive_compiles() {
        #[derive(RestrictedAccess)]
        struct Audited;

        fn assert_restricted<T: RestrictedAccess>() {}
        assert_restricted::<Audited>();

        let mut world = World::new();
        let component_id = world.register_component::<Audited>();
        let component_info = world
            .components()
            .get_info(component_id)
            .expect("registered component info should exist");

        assert!(component_info.restricted_access());
        assert!(component_info.mutable());
    }
}
