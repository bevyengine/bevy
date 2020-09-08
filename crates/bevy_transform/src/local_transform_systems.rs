use crate::components::*;
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Quat, Vec3};

// TODO: "on changed" for all of these systems
pub fn local_transform_translation_system(
    mut query: Query<
        Without<
            Rotation,
            Without<Scale, Without<NonUniformScale, (&mut LocalTransform, Changed<Translation>)>>,
        >,
    >,
) {
    for (mut local, translation) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_translation(translation.0));
    }
}

pub fn local_transform_rotation_system(
    mut query: Query<
        Without<
            Translation,
            Without<Scale, Without<NonUniformScale, (&mut LocalTransform, Changed<Rotation>)>>,
        >,
    >,
) {
    for (mut local, rotation) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_quat(rotation.0));
    }
}

pub fn local_transform_scale_system(
    mut query: Query<
        Without<
            Translation,
            Without<Rotation, Without<NonUniformScale, (&mut LocalTransform, Changed<Scale>)>>,
        >,
    >,
) {
    for (mut local, scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale(Vec3::new(scale.0, scale.0, scale.0)));
    }
}

pub fn local_transform_non_uniform_scale_system(
    mut query: Query<
        Without<
            Translation,
            Without<Rotation, Without<Scale, (&mut LocalTransform, Changed<NonUniformScale>)>>,
        >,
    >,
) {
    for (mut local, non_uniform_scale) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale(non_uniform_scale.0));
    }
}

pub fn local_transform_translation_rotation_system(
    mut query: Query<
        Without<
            Scale,
            Without<
                NonUniformScale,
                (
                    &mut LocalTransform,
                    Or<(Changed<Translation>, Changed<Rotation>)>,
                ),
            >,
        >,
    >,
) {
    for (mut local, (translation, rotation)) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_rotation_translation(rotation.0, translation.0));
    }
}

pub fn local_transform_translation_scale_system(
    mut query: Query<
        Without<
            Rotation,
            Without<
                NonUniformScale,
                (
                    &mut LocalTransform,
                    Or<(Changed<Translation>, Changed<Scale>)>,
                ),
            >,
        >,
    >,
) {
    for (mut local, (translation, scale)) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            Quat::default(),
            translation.0,
        ));
    }
}

pub fn local_transform_translation_non_uniform_scale_system(
    mut query: Query<
        Without<
            Rotation,
            Without<
                Scale,
                (
                    &mut LocalTransform,
                    Or<(Changed<Translation>, Changed<NonUniformScale>)>,
                ),
            >,
        >,
    >,
) {
    for (mut local, (translation, non_uniform_scale)) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            Quat::default(),
            translation.0,
        ));
    }
}

pub fn local_transform_rotation_scale_system(
    mut query: Query<
        Without<
            Translation,
            Without<
                NonUniformScale,
                (&mut LocalTransform, Or<(Changed<Rotation>, Changed<Scale>)>),
            >,
        >,
    >,
) {
    for (mut local, (rotation, scale)) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            Vec3::new(scale.0, scale.0, scale.0),
            rotation.0,
            Vec3::default(),
        ));
    }
}

pub fn local_transform_rotation_non_uniform_scale_system(
    mut query: Query<
        Without<
            Translation,
            Without<
                Scale,
                (
                    &mut LocalTransform,
                    Or<(Changed<Rotation>, Changed<NonUniformScale>)>,
                ),
            >,
        >,
    >,
) {
    for (mut local, (rotation, non_uniform_scale)) in &mut query.iter() {
        *local = LocalTransform(Mat4::from_scale_rotation_translation(
            non_uniform_scale.0,
            rotation.0,
            Vec3::default(),
        ));
    }
}

pub fn local_transform_translation_rotation_scale_system(
    mut query: Query<
        Without<
            NonUniformScale,
            (
                &mut LocalTransform,
                Or<(Changed<Translation>, Changed<Rotation>, Changed<Scale>)>,
            ),
        >,
    >,
) {
    for (mut local, (translation, rotation, scale)) in &mut query.iter() {
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
                Or<(
                    Changed<Translation>,
                    Changed<Rotation>,
                    Changed<NonUniformScale>,
                )>,
            ),
        >,
    >,
) {
    for (mut local, (translation, rotation, non_uniform_scale)) in &mut query.iter() {
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
    use bevy_ecs::{Resources, Schedule, World};
    use bevy_math::{Mat4, Quat, Vec3};

    #[test]
    fn correct_local_transformation() {
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

    #[test]
    fn only_propagates_local_transform_on_change() {
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
        world.spawn((local_transform, t));
        world.spawn((local_transform, r));
        world.spawn((local_transform, s));
        world.spawn((local_transform, nus));
        world.spawn((local_transform, t, r));
        world.spawn((local_transform, t, s));
        world.spawn((local_transform, t, nus));
        world.spawn((local_transform, r, s));
        world.spawn((local_transform, r, nus));
        world.spawn((local_transform, t, r, s));
        world.spawn((local_transform, t, r, nus));

        // Run the system, local transforms should mutate since they are new
        schedule.run(&mut world, &mut resources);

        // Verify that the local transform is not mutated on the second frame
        fn assert_no_local_transforms_changed_system(_: Changed<LocalTransform>) {
            assert!(false)
        }

        schedule.add_system_to_stage("update", assert_no_local_transforms_changed_system.system());
        schedule.run(&mut world, &mut resources);
    }
}
