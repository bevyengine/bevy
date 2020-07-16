#![allow(dead_code)]
use crate::components::*;
use bevy_ecs::{Commands, Entity, Query, Without};

pub fn transform_propagate_system(
    mut commands: Commands,
    // Entities with a `Children` and `Transform` but NOT a `Parent` (ie those that are
    // roots of a hierarchy).
    mut root_query: Query<Without<Parent, (&Children, &Transform)>>,
    mut children_query: Query<&Children>,
    mut local_transform_query: Query<&LocalTransform>,
) {
    for (children, local_to_world) in &mut root_query.iter() {
        for child in children.0.iter() {
            propagate_recursive(
                *local_to_world,
                &mut children_query,
                &mut local_transform_query,
                *child,
                &mut commands,
            );
        }
    }
}

fn propagate_recursive(
    parent_local_to_world: Transform,
    children_query: &mut Query<&Children>,
    local_transform_query: &mut Query<&LocalTransform>,
    entity: Entity,
    commands: &mut Commands,
) {
    log::trace!("Updating Transform for {:?}", entity);
    let local_transform = {
        if let Ok(local_transform) = local_transform_query.get::<LocalTransform>(entity) {
            *local_transform
        } else {
            log::warn!(
                "Entity {:?} is a child in the hierarchy but does not have a LocalTransform",
                entity
            );
            return;
        }
    };

    let new_transform = Transform {
        value: parent_local_to_world.value * local_transform.0,
        sync: true,
    };

    commands.insert_one(entity, new_transform);

    // Collect children
    let children = children_query
        .get::<Children>(entity)
        .map(|e| e.0.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for child in children {
        propagate_recursive(
            new_transform,
            children_query,
            local_transform_query,
            child,
            commands,
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::transform_systems;
    use bevy_ecs::{Resources, Schedule, World};
    use bevy_math::{Mat4, Vec3};

    #[test]
    fn did_propagate() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        for system in transform_systems() {
            schedule.add_system_to_stage("update", system);
        }

        // Root entity
        let parent = world.spawn((Translation::new(1.0, 0.0, 0.0), Transform::identity()));
        let children = world
            .spawn_batch(vec![
                (
                    Translation::new(0.0, 2.0, 0.0),
                    LocalTransform::identity(),
                    Transform::identity(),
                    Parent(parent),
                ),
                (
                    Translation::new(0.0, 0.0, 3.0),
                    LocalTransform::identity(),
                    Transform::identity(),
                    Parent(parent),
                ),
            ])
            .collect::<Vec<Entity>>();

        // TODO: ideally we dont need three runs to keep transforms in sync.
        // command buffers should be flushed in the appropriate places
        schedule.run(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

        assert_eq!(
            world.get::<Transform>(children[0]).unwrap().value,
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            world.get::<Transform>(children[1]).unwrap().value,
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 0.0, 3.0))
        );
    }
}
