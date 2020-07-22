use crate::components::*;
use bevy_ecs::prelude::*;

pub fn transform_propagate_system(
    mut root_query: Query<
        Without<Parent, (Option<&Children>, &mut Transform, Option<&LocalTransform>)>,
    >,
    mut local_transform_query: Query<(&mut Transform, &LocalTransform, Option<&Children>)>,
) {
    for (children, mut transform, local_transform) in &mut root_query.iter() {
        if let Some(local_transform) = local_transform {
            transform.value = local_transform.0;
        }

        if let Some(children) = children {
            for child in children.0.iter() {
                propagate_recursive(*transform, &mut local_transform_query, *child);
            }
        }
    }
}

fn propagate_recursive(
    parent_local_to_world: Transform,
    local_transform_query: &mut Query<(&mut Transform, &LocalTransform, Option<&Children>)>,
    entity: Entity,
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

    {
        let mut transform = local_transform_query.get_mut::<Transform>(entity).unwrap();
        transform.value = new_transform.value;
    }

    // Collect children
    let children = local_transform_query
        .get::<Children>(entity)
        .map(|e| e.0.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for child in children {
        propagate_recursive(new_transform, local_transform_query, child);
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
