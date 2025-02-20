use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::query::With;
use bevy_ecs::query::Without;
use bevy_ecs::system::Query;

use crate::experimental::FlattenChildren;
use crate::Node;

pub type UiRootNodes<'w, 's> = Query<'w, 's, Entity, (With<Node>, Without<ChildOf>)>;
pub type UiChildren<'w, 's> = FlattenChildren<'w, 's, Node>;

mod tests {
    use bevy_ecs::prelude::Component;
    use bevy_ecs::system::Query;
    use bevy_ecs::system::SystemState;
    use bevy_ecs::world::World;

    use crate::Node;
    use crate::UiChildren;
    use crate::UiRootNodes;

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn iterate_ui_root_nodes() {
        let world = &mut World::new();

        // Normal root
        world
            .spawn((A(1), Node::default()))
            .with_children(|parent| {
                parent.spawn((A(2), Node::default()));
                parent.spawn(A(3)).with_child((A(4), Node::default()));
            });

        // Ghost root
        world.spawn(A(5)).with_children(|parent| {
            parent.spawn((A(6), Node::default()));
            parent
                .spawn((A(7),))
                .with_child((A(8), Node::default()))
                .with_child(A(9));
        });

        let mut system_state = SystemState::<(UiRootNodes, Query<&A>)>::new(world);
        let (ui_root_nodes, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_root_nodes.iter()).collect();

        assert_eq!([&A(1), &A(6), &A(8)], result.as_slice());
    }

    #[test]
    fn iterate_ui_children() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), Node::default())).id();
        let n2 = world.spawn((A(2),)).id();
        let n3 = world.spawn((A(3),)).id();
        let n4 = world.spawn((A(4), Node::default())).id();
        let n5 = world.spawn((A(5), Node::default())).id();

        let n6 = world.spawn((A(6),)).id();
        let n7 = world.spawn((A(7),)).id();
        let n8 = world.spawn((A(8), Node::default())).id();
        let n9 = world.spawn((A(9),)).id();
        let n10 = world.spawn((A(10), Node::default())).id();

        let no_ui = world.spawn_empty().id();

        world.entity_mut(n1).add_children(&[n2, n3, n4, n6]);
        world.entity_mut(n2).add_children(&[n5]);

        world.entity_mut(n6).add_children(&[n7, no_ui, n9]);
        world.entity_mut(n7).add_children(&[n8]);
        world.entity_mut(n9).add_children(&[n10]);

        let mut system_state = SystemState::<(UiChildren, Query<&A>)>::new(world);
        let (ui_children, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(ui_children.iter_actual_children(n1))
            .collect();

        assert_eq!([&A(5), &A(4), &A(8), &A(10)], result.as_slice());
    }
}
