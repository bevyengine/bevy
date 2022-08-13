#[cfg(test)]
mod test {
    // TODO: move these tests to bevy_hierarchy::Propagate module

    use bevy_app::prelude::*;
    use bevy_ecs::prelude::*;
    use bevy_ecs::system::CommandQueue;
    use bevy_math::vec3;

    use crate::components::{GlobalTransform, Transform};
    use crate::TransformBundle;
    use bevy_hierarchy::{BuildChildren, BuildWorldChildren, Children};

    #[test]
    fn did_propagate() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(bevy_hierarchy::propagate_system::<Transform>);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Root entity
        world
            .spawn()
            .insert_bundle(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)));

        let mut children = Vec::new();
        world
            .spawn()
            .insert_bundle(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.)))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.)))
                        .id(),
                );
            });
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 2.0, 0.0)
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 0.0, 3.0)
        );
    }

    #[test]
    fn did_propagate_command_buffer() {
        let mut world = World::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(bevy_hierarchy::propagate_system::<Transform>);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Root entity
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut children = Vec::new();
        commands
            .spawn_bundle(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.0)))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.0)))
                        .id(),
                );
            });
        queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 2.0, 0.0)
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 0.0, 3.0)
        );
    }

    #[test]
    fn correct_transforms_when_no_children() {
        let mut app = App::new();

        app.add_system(bevy_hierarchy::propagate_system::<Transform>);

        let translation = vec3(1.0, 0.0, 0.0);

        // These will be overwritten.
        let mut child = Entity::from_raw(0);
        let mut grandchild = Entity::from_raw(1);
        let parent = app
            .world
            .spawn()
            .insert(Transform::from_translation(translation))
            .insert(GlobalTransform::default())
            .with_children(|builder| {
                child = builder
                    .spawn_bundle((Transform::identity(), GlobalTransform::default()))
                    .with_children(|builder| {
                        grandchild = builder
                            .spawn_bundle((Transform::identity(), GlobalTransform::default()))
                            .id();
                    })
                    .id();
            })
            .id();

        app.update();

        // check the `Children` structure is spawned
        assert_eq!(&**app.world.get::<Children>(parent).unwrap(), &[child]);
        assert_eq!(&**app.world.get::<Children>(child).unwrap(), &[grandchild]);
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to `Commands` delay
        app.update();

        let mut state = app.world.query::<&GlobalTransform>();
        for global in state.iter(&app.world) {
            assert_eq!(global, &GlobalTransform::from_translation(translation));
        }
    }
}
