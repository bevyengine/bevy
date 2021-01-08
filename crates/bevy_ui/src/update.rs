pub const UI_Z_STEP: f32 = 0.001;

#[cfg(test)]
mod tests {
    use bevy_ecs::{Commands, IntoSystem, Resources, Schedule, SystemStage, World};
    use bevy_transform::{components::Transform, hierarchy::BuildChildren};

    use crate::Node;

    use super::UI_Z_STEP;

    fn node_with_transform(name: &str) -> (String, Node, Transform) {
        (name.to_owned(), Node::default(), Transform::default())
    }

    fn node_without_transform(name: &str) -> (String, Node) {
        (name.to_owned(), Node::default())
    }

    fn get_steps(transform: &Transform) -> u32 {
        (transform.translation.z / UI_Z_STEP).round() as u32
    }

    #[test]
    fn test_ui_z_system() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());

        commands.spawn(node_with_transform("0"));

        commands
            .spawn(node_with_transform("1"))
            .with_children(|parent| {
                parent
                    .spawn(node_with_transform("1-0"))
                    .with_children(|parent| {
                        parent.spawn(node_with_transform("1-0-0"));
                        parent.spawn(node_without_transform("1-0-1"));
                        parent.spawn(node_with_transform("1-0-2"));
                    });
                parent.spawn(node_with_transform("1-1"));
                parent
                    .spawn(node_without_transform("1-2"))
                    .with_children(|parent| {
                        parent.spawn(node_with_transform("1-2-0"));
                        parent.spawn(node_with_transform("1-2-1"));
                        parent
                            .spawn(node_with_transform("1-2-2"))
                            .with_children(|_| ());
                        parent.spawn(node_with_transform("1-2-3"));
                    });
                parent.spawn(node_with_transform("1-3"));
            });

        commands
            .spawn(node_without_transform("2"))
            .with_children(|parent| {
                parent
                    .spawn(node_with_transform("2-0"))
                    .with_children(|_parent| ());
                parent
                    .spawn(node_with_transform("2-1"))
                    .with_children(|parent| {
                        parent.spawn(node_with_transform("2-1-0"));
                    });
            });
        commands.apply(&mut world, &mut resources);

        let mut schedule = Schedule::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(ui_z_system.system()); // FIXME
        schedule.add_stage("update", update_stage);
        schedule.initialize_and_run(&mut world, &mut resources);

        let mut actual_result = world
            .query::<(&String, &Transform)>()
            .map(|(name, transform)| (name.clone(), get_steps(transform)))
            .collect::<Vec<(String, u32)>>();
        actual_result.sort_unstable_by_key(|(name, _)| name.clone());
        let expected_result = vec![
            ("0".to_owned(), 1),
            ("1".to_owned(), 1),
            ("1-0".to_owned(), 1),
            ("1-0-0".to_owned(), 1),
            // 1-0-1 has no transform
            ("1-0-2".to_owned(), 3),
            ("1-1".to_owned(), 5),
            // 1-2 has no transform
            ("1-2-0".to_owned(), 1),
            ("1-2-1".to_owned(), 2),
            ("1-2-2".to_owned(), 3),
            ("1-2-3".to_owned(), 4),
            ("1-3".to_owned(), 11),
            // 2 has no transform
            ("2-0".to_owned(), 1),
            ("2-1".to_owned(), 2),
            ("2-1-0".to_owned(), 1),
        ];
        assert_eq!(actual_result, expected_result);
    }
}
