#![allow(dead_code)]
use crate::{
    components::*,
    math::{Mat4, Quat, Vec3},
};

use bevy_ecs::{IntoQuerySystem, Query, System, Without};

// TODO: "on changed" for all of these systems
pub fn local_transform_translation_system(
    mut query: Query<
        Without<
            Rotation,
            Without<Scale, Without<NonUniformScale, (&mut LocalTransform, &Translation)>>,
        >,
    >,
) {
    for (local, translation) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_translation(translation.0));
    }
}

pub fn local_transform_rotation_system(
    mut query: Query<
        Without<
            Translation,
            Without<Scale, Without<NonUniformScale, (&mut LocalTransform, &Rotation)>>,
        >,
    >,
) {
    for (local, rotation) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_quat(rotation.0));
    }
}

pub fn local_transform_scale_system(
    mut query: Query<
        Without<
            Translation,
            Without<Rotation, Without<NonUniformScale, (&mut LocalTransform, &Scale)>>,
        >,
    >,
) {
    for (local, scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale(Vec3::new(scale.0, scale.0, scale.0)));
    }
}

pub fn local_transform_non_uniform_scale_system(
    mut query: Query<
        Without<
            Translation,
            Without<Rotation, Without<Scale, (&mut LocalTransform, &NonUniformScale)>>,
        >,
    >,
) {
    for (local, non_uniform_scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale(non_uniform_scale.0));
    }
}

pub fn local_transform_translation_rotation_system(
    mut query: Query<
        Without<Scale, Without<NonUniformScale, (&mut LocalTransform, &Translation, &Rotation)>>,
    >,
) {
    for (local, translation, rotation) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_rotation_translation(rotation.0, translation.0));
    }
}

pub fn local_transform_translation_scale_system(
    mut query: Query<
        Without<Rotation, Without<NonUniformScale, (&mut LocalTransform, &Translation, &Scale)>>,
    >,
) {
    for (local, translation, scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            Quat::default(),
            translation.0,
        ));
    }
}

pub fn local_transform_translation_non_uniform_scale_system(
    mut query: Query<
        Without<Rotation, Without<Scale, (&mut LocalTransform, &Translation, &NonUniformScale)>>,
    >,
) {
    for (local, translation, non_uniform_scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            Quat::default(),
            translation.0,
        ));
    }
}

pub fn local_transform_rotation_scale_system(
    mut query: Query<
        Without<Translation, Without<NonUniformScale, (&mut LocalTransform, &Rotation, &Scale)>>,
    >,
) {
    for (local, rotation, scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            rotation.0,
            Vec3::default(),
        ));
    }
}

pub fn local_transform_rotation_non_uniform_scale_system(
    mut query: Query<
        Without<Translation, Without<Scale, (&mut LocalTransform, &Rotation, &NonUniformScale)>>,
    >,
) {
    for (local, rotation, non_uniform_scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            rotation.0,
            Vec3::default(),
        ));
    }
}

pub fn local_transform_translation_rotation_scale_system(
    mut query: Query<
        Without<NonUniformScale, (&mut LocalTransform, &Translation, &Rotation, &Scale)>,
    >,
) {
    for (local, translation, rotation, scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            rotation.0,
            translation.0,
        ));
    }
}

pub fn local_transform_translation_rotation_non_uniform_scale_system(
    mut query: Query<
        Without<
            Scale,
            (
                &mut LocalTransform,
                &Translation,
                &Rotation,
                &NonUniformScale,
            ),
        >,
    >,
) {
    for (local, translation, rotation, non_uniform_scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            rotation.0,
            translation.0,
        ));
    }
}

pub fn local_transform_systems() -> Vec<Box<dyn System>> {
    vec![
        local_transform_translation_system.system(),
        local_transform_rotation_system.system(),
        local_transform_scale_system.system(),
        local_transform_non_uniform_scale_system.system(),
        local_transform_translation_rotation_system.system(),
        local_transform_translation_scale_system.system(),
        local_transform_translation_non_uniform_scale_system.system(),
        local_transform_rotation_scale_system.system(),
        local_transform_rotation_non_uniform_scale_system.system(),
        local_transform_translation_rotation_scale_system.system(),
        local_transform_translation_rotation_non_uniform_scale_system.system(),
    ]
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::{Mat4, Quat, Vec3};
    use bevy_ecs::{Resources, Schedule, World};

    #[test]
    fn correct_local_transformation() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        for system in local_transform_systems() {
            schedule.add_system_to_stage("update", system);
        }

        let local_transform = LocalTransform::identity();
        let t = Translation::new(1.0, 2.0, 3.0);
        let r = Rotation(Quat::from_rotation_ypr(1.0, 2.0, 3.0));
        let s = Scale(2.0);
        let nus = NonUniformScale::new(1.0, 2.0, 3.0);

        // Add every combination of transform types.
        let translation = world.spawn((local_transform, t));
        let rotation = world.spawn((local_transform, r));
        let scale = world.spawn((local_transform, s));
        let non_uniform_scale = world.spawn((local_transform, nus));
        let translation_and_rotation = world.spawn((local_transform, t, r));
        let translation_and_scale = world.spawn((local_transform, t, s));
        let translation_and_nus = world.spawn((local_transform, t, nus));
        let rotation_scale = world.spawn((local_transform, r, s));
        let rotation_nus = world.spawn((local_transform, r, nus));
        let translation_rotation_scale = world.spawn((local_transform, t, r, s));
        let translation_rotation_nus = world.spawn((local_transform, t, r, nus));

        // Run the system
        schedule.run(&mut world, &mut resources);

        // Verify that each was transformed correctly.
        assert_eq!(
            world.get::<LocalTransform>(translation).unwrap().0,
            Mat4::from_translation(t.0)
        );
        assert_eq!(
            world.get::<LocalTransform>(rotation).unwrap().0,
            Mat4::from_quat(r.0)
        );
        assert_eq!(
            world.get::<LocalTransform>(scale).unwrap().0,
            Mat4::from_scale(Vec3::new(s.0, s.0, s.0))
        );
        assert_eq!(
            world.get::<LocalTransform>(non_uniform_scale).unwrap().0,
            Mat4::from_scale(nus.0)
        );
        assert_eq!(
            world
                .get::<LocalTransform>(translation_and_rotation)
                .unwrap()
                .0,
            Mat4::from_rotation_translation(r.0, t.0)
        );
        assert_eq!(
            world
                .get::<LocalTransform>(translation_and_scale)
                .unwrap()
                .0,
            Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), Quat::default(), t.0)
        );
        assert_eq!(
            world.get::<LocalTransform>(translation_and_nus).unwrap().0,
            Mat4::from_scale_rotation_translation(nus.0, Quat::default(), t.0)
        );
        assert_eq!(
            world.get::<LocalTransform>(rotation_scale).unwrap().0,
            Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, Vec3::default())
        );
        assert_eq!(
            world.get::<LocalTransform>(rotation_nus).unwrap().0,
            Mat4::from_scale_rotation_translation(nus.0, r.0, Vec3::default())
        );
        assert_eq!(
            world
                .get::<LocalTransform>(translation_rotation_scale)
                .unwrap()
                .0,
            Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, t.0)
        );
        assert_eq!(
            world
                .get::<LocalTransform>(translation_rotation_nus)
                .unwrap()
                .0,
            Mat4::from_scale_rotation_translation(nus.0, r.0, t.0)
        );
    }
}
