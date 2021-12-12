use super::Node;
use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::Query,
};
use bevy_transform::prelude::{Children, Parent, Transform};

pub const UI_Z_STEP: f32 = 0.001;

pub fn ui_z_system(
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    mut node_query: Query<&mut Transform, With<Node>>,
    children_query: Query<&Children>,
) {
    let mut current_global_z = 0.0;
    for entity in root_node_query.iter() {
        current_global_z = update_hierarchy(
            &children_query,
            &mut node_query,
            entity,
            current_global_z,
            current_global_z,
        );
    }
}

fn update_hierarchy(
    children_query: &Query<&Children>,
    node_query: &mut Query<&mut Transform, With<Node>>,
    entity: Entity,
    parent_global_z: f32,
    mut current_global_z: f32,
) -> f32 {
    current_global_z += UI_Z_STEP;
    if let Ok(mut transform) = node_query.get_mut(entity) {
        transform.translation.z = current_global_z - parent_global_z;
    }
    if let Ok(children) = children_query.get(entity) {
        let current_parent_global_z = current_global_z;
        for child in children.iter().cloned() {
            current_global_z = update_hierarchy(
                children_query,
                node_query,
                child,
                current_parent_global_z,
                current_global_z,
            );
        }
    }
    current_global_z
}
#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        schedule::{Schedule, Stage, SystemStage},
        system::{CommandQueue, Commands},
        world::World,
    };
    use bevy_transform::{components::Transform, hierarchy::BuildChildren};

    use crate::Node;

    use super::{ui_z_system, UI_Z_STEP};

    #[derive(Component, PartialEq, Debug, Clone)]
    struct Label(&'static str);

    fn node_with_transform(name: &'static str) -> (Label, Node, Transform) {
        (Label(name), Node::default(), Transform::identity())
    }

    fn node_without_transform(name: &'static str) -> (Label, Node) {
        (Label(name), Node::default())
    }

    fn get_steps(transform: &Transform) -> u32 {
        (transform.translation.z / UI_Z_STEP).round() as u32
    }

    #[test]
    fn test_ui_z_system() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands.spawn_bundle(node_with_transform("0"));

        commands
            .spawn_bundle(node_with_transform("1"))
            .with_children(|parent| {
                parent
                    .spawn_bundle(node_with_transform("1-0"))
                    .with_children(|parent| {
                        parent.spawn_bundle(node_with_transform("1-0-0"));
                        parent.spawn_bundle(node_without_transform("1-0-1"));
                        parent.spawn_bundle(node_with_transform("1-0-2"));
                    });
                parent.spawn_bundle(node_with_transform("1-1"));
                parent
                    .spawn_bundle(node_without_transform("1-2"))
                    .with_children(|parent| {
                        parent.spawn_bundle(node_with_transform("1-2-0"));
                        parent.spawn_bundle(node_with_transform("1-2-1"));
                        parent
                            .spawn_bundle(node_with_transform("1-2-2"))
                            .with_children(|_| ());
                        parent.spawn_bundle(node_with_transform("1-2-3"));
                    });
                parent.spawn_bundle(node_with_transform("1-3"));
            });

        commands
            .spawn_bundle(node_without_transform("2"))
            .with_children(|parent| {
                parent
                    .spawn_bundle(node_with_transform("2-0"))
                    .with_children(|_parent| ());
                parent
                    .spawn_bundle(node_with_transform("2-1"))
                    .with_children(|parent| {
                        parent.spawn_bundle(node_with_transform("2-1-0"));
                    });
            });
        queue.apply(&mut world);

        let mut schedule = Schedule::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(ui_z_system);
        schedule.add_stage("update", update_stage);
        schedule.run(&mut world);

        let mut actual_result = world
            .query::<(&Label, &Transform)>()
            .iter(&world)
            .map(|(name, transform)| (name.clone(), get_steps(transform)))
            .collect::<Vec<(Label, u32)>>();
        actual_result.sort_unstable_by_key(|(name, _)| name.0);
        let expected_result = vec![
            (Label("0"), 1),
            (Label("1"), 1),
            (Label("1-0"), 1),
            (Label("1-0-0"), 1),
            // 1-0-1 has no transform
            (Label("1-0-2"), 3),
            (Label("1-1"), 5),
            // 1-2 has no transform
            (Label("1-2-0"), 1),
            (Label("1-2-1"), 2),
            (Label("1-2-2"), 3),
            (Label("1-2-3"), 4),
            (Label("1-3"), 11),
            // 2 has no transform
            (Label("2-0"), 1),
            (Label("2-1"), 2),
            (Label("2-1-0"), 1),
        ];
        assert_eq!(actual_result, expected_result);
    }
}
