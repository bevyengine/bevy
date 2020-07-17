#![allow(dead_code)]
use crate::components::*;

use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Quat, Vec3};

// TODO: on changed for all of these systems
pub fn transform_translation_system(
    mut query: Query<
        Without<Rotation, Without<Scale, Without<NonUniformScale, (&mut Transform, &Translation)>>>,
    >,
) {
    for (transform, translation) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_translation(translation.0));
    }
}

pub fn transform_rotation_system(
    mut query: Query<
        Without<Translation, Without<Scale, Without<NonUniformScale, (&mut Transform, &Rotation)>>>,
    >,
) {
    for (transform, rotation) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_quat(rotation.0));
    }
}

pub fn transform_scale_system(
    mut query: Query<
        Without<Translation, Without<Rotation, Without<NonUniformScale, (&mut Transform, &Scale)>>>,
    >,
) {
    for (transform, scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale(Vec3::new(scale.0, scale.0, scale.0)));
    }
}

pub fn transform_non_uniform_scale_system(
    mut query: Query<
        Without<Translation, Without<Rotation, Without<Scale, (&mut Transform, &NonUniformScale)>>>,
    >,
) {
    for (transform, non_uniform_scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale(non_uniform_scale.0));
    }
}

pub fn transform_translation_rotation_system(
    mut query: Query<
        Without<Scale, Without<NonUniformScale, (&mut Transform, &Translation, &Rotation)>>,
    >,
) {
    for (transform, translation, rotation) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_rotation_translation(rotation.0, translation.0));
    }
}

pub fn transform_translation_scale_system(
    mut query: Query<
        Without<Rotation, Without<NonUniformScale, (&mut Transform, &Translation, &Scale)>>,
    >,
) {
    for (transform, translation, scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            Quat::default(),
            translation.0,
        ));
    }
}

pub fn transform_translation_non_uniform_scale_system(
    mut query: Query<
        Without<Rotation, Without<Scale, (&mut Transform, &Translation, &NonUniformScale)>>,
    >,
) {
    for (transform, translation, non_uniform_scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            Quat::default(),
            translation.0,
        ));
    }
}

pub fn transform_rotation_scale_system(
    mut query: Query<
        Without<Translation, Without<NonUniformScale, (&mut Transform, &Rotation, &Scale)>>,
    >,
) {
    for (transform, rotation, scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            rotation.0,
            Vec3::default(),
        ));
    }
}

pub fn transform_rotation_non_uniform_scale_system(
    mut query: Query<
        Without<Translation, Without<Scale, (&mut Transform, &Rotation, &NonUniformScale)>>,
    >,
) {
    for (transform, rotation, non_uniform_scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            rotation.0,
            Vec3::default(),
        ));
    }
}

pub fn transform_translation_rotation_scale_system(
    mut query: Query<Without<NonUniformScale, (&mut Transform, &Translation, &Rotation, &Scale)>>,
) {
    for (transform, translation, rotation, scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            rotation.0,
            translation.0,
        ));
    }
}

pub fn transform_translation_rotation_non_uniform_scale_system(
    mut query: Query<Without<Scale, (&mut Transform, &Translation, &Rotation, &NonUniformScale)>>,
) {
    for (transform, translation, rotation, non_uniform_scale) in &mut query.iter() {
        if !transform.sync {
            continue;
        }

        *transform = Transform::new(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            rotation.0,
            translation.0,
        ));
    }
}

pub fn transform_systems() -> Vec<Box<dyn System>> {
    vec![
        transform_translation_system.system(),
        transform_rotation_system.system(),
        transform_scale_system.system(),
        transform_non_uniform_scale_system.system(),
        transform_translation_rotation_system.system(),
        transform_translation_scale_system.system(),
        transform_translation_non_uniform_scale_system.system(),
        transform_rotation_scale_system.system(),
        transform_rotation_non_uniform_scale_system.system(),
        transform_translation_rotation_scale_system.system(),
        transform_translation_rotation_non_uniform_scale_system.system(),
    ]
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_ecs::{Resources, Schedule, World};
    use bevy_math::{Mat4, Quat, Vec3};

    #[test]
    fn correct_world_transformation() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        for system in transform_systems() {
            schedule.add_system_to_stage("update", system);
        }

        let transform = Transform::identity();
        let t = Translation::new(1.0, 2.0, 3.0);
        let r = Rotation(Quat::from_rotation_ypr(1.0, 2.0, 3.0));
        let s = Scale(2.0);
        let nus = NonUniformScale::new(1.0, 2.0, 3.0);

        // Add every combination of transform types.
        let translation = world.spawn((transform, t));
        let rotation = world.spawn((transform, r));
        let scale = world.spawn((transform, s));
        let non_uniform_scale = world.spawn((transform, nus));
        let translation_and_rotation = world.spawn((transform, t, r));
        let translation_and_scale = world.spawn((transform, t, s));
        let translation_and_nus = world.spawn((transform, t, nus));
        let rotation_scale = world.spawn((transform, r, s));
        let rotation_nus = world.spawn((transform, r, nus));
        let translation_rotation_scale = world.spawn((transform, t, r, s));
        let translation_rotation_nus = world.spawn((transform, t, r, nus));

        // Run the system
        schedule.run(&mut world, &mut resources);

        // Verify that each was transformed correctly.
        assert_eq!(
            world.get::<Transform>(translation).unwrap().value,
            Mat4::from_translation(t.0)
        );
        assert_eq!(
            world.get::<Transform>(rotation).unwrap().value,
            Mat4::from_quat(r.0)
        );
        assert_eq!(
            world.get::<Transform>(scale).unwrap().value,
            Mat4::from_scale(Vec3::new(s.0, s.0, s.0))
        );
        assert_eq!(
            world.get::<Transform>(non_uniform_scale).unwrap().value,
            Mat4::from_scale(nus.0)
        );
        assert_eq!(
            world
                .get::<Transform>(translation_and_rotation)
                .unwrap()
                .value,
            Mat4::from_rotation_translation(r.0, t.0)
        );
        assert_eq!(
            world.get::<Transform>(translation_and_scale).unwrap().value,
            Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), Quat::default(), t.0)
        );
        assert_eq!(
            world.get::<Transform>(translation_and_nus).unwrap().value,
            Mat4::from_scale_rotation_translation(nus.0, Quat::default(), t.0)
        );
        assert_eq!(
            world.get::<Transform>(rotation_scale).unwrap().value,
            Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, Vec3::default())
        );
        assert_eq!(
            world.get::<Transform>(rotation_nus).unwrap().value,
            Mat4::from_scale_rotation_translation(nus.0, r.0, Vec3::default())
        );
        assert_eq!(
            world
                .get::<Transform>(translation_rotation_scale)
                .unwrap()
                .value,
            Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, t.0)
        );
        assert_eq!(
            world
                .get::<Transform>(translation_rotation_nus)
                .unwrap()
                .value,
            Mat4::from_scale_rotation_translation(nus.0, r.0, t.0)
        );
    }
}
