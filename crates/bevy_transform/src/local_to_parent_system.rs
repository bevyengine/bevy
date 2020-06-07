#![allow(dead_code)]
use crate::{
    components::*,
    ecs::prelude::*,
    math::{Mat4, Quat, Vec3},
};

pub fn build(_: &mut World) -> Box<dyn Schedulable> {
    SystemBuilder::<()>::new("LocalToParentUpdateSystem")
        // Translation
        .with_query(<(Write<LocalTransform>, Read<Translation>)>::query().filter(
            !component::<Rotation>()
                & !component::<Scale>()
                & !component::<NonUniformScale>()
                & (changed::<Translation>()),
        ))
        // Rotation
        .with_query(<(Write<LocalTransform>, Read<Rotation>)>::query().filter(
            !component::<Translation>()
                & !component::<Scale>()
                & !component::<NonUniformScale>()
                & (changed::<Rotation>()),
        ))
        // Scale
        .with_query(<(Write<LocalTransform>, Read<Scale>)>::query().filter(
            !component::<Translation>()
                & !component::<Rotation>()
                & !component::<NonUniformScale>()
                & (changed::<Scale>()),
        ))
        // NonUniformScale
        .with_query(
            <(Write<LocalTransform>, Read<NonUniformScale>)>::query().filter(
                !component::<Translation>()
                    & !component::<Rotation>()
                    & !component::<Scale>()
                    & (changed::<NonUniformScale>()),
            ),
        )
        // Translation + Rotation
        .with_query(
            <(Write<LocalTransform>, Read<Translation>, Read<Rotation>)>::query().filter(
                !component::<Scale>()
                    & !component::<NonUniformScale>()
                    & (changed::<Translation>() | changed::<Rotation>()),
            ),
        )
        // Translation + Scale
        .with_query(
            <(Write<LocalTransform>, Read<Translation>, Read<Scale>)>::query().filter(
                !component::<Rotation>()
                    & !component::<NonUniformScale>()
                    & (changed::<Translation>() | changed::<Scale>()),
            ),
        )
        // Translation + NonUniformScale
        .with_query(
            <(
                Write<LocalTransform>,
                Read<Translation>,
                Read<NonUniformScale>,
            )>::query()
            .filter(
                !component::<Rotation>()
                    & !component::<Scale>()
                    & (changed::<Translation>() | changed::<NonUniformScale>()),
            ),
        )
        // Rotation + Scale
        .with_query(
            <(Write<LocalTransform>, Read<Rotation>, Read<Scale>)>::query().filter(
                !component::<Translation>()
                    & !component::<NonUniformScale>()
                    & (changed::<Rotation>() | changed::<Scale>()),
            ),
        )
        // Rotation + NonUniformScale
        .with_query(
            <(Write<LocalTransform>, Read<Rotation>, Read<NonUniformScale>)>::query().filter(
                !component::<Translation>()
                    & !component::<Scale>()
                    & (changed::<Rotation>() | changed::<NonUniformScale>()),
            ),
        )
        // Translation + Rotation + Scale
        .with_query(
            <(
                Write<LocalTransform>,
                Read<Translation>,
                Read<Rotation>,
                Read<Scale>,
            )>::query()
            .filter(
                !component::<NonUniformScale>()
                    & (changed::<Translation>() | changed::<Rotation>() | changed::<Scale>()),
            ),
        )
        // Translation + Rotation + NonUniformScale
        .with_query(
            <(
                Write<LocalTransform>,
                Read<Translation>,
                Read<Rotation>,
                Read<NonUniformScale>,
            )>::query()
            .filter(
                !component::<Scale>()
                    & (changed::<Translation>()
                        | changed::<Rotation>()
                        | changed::<NonUniformScale>()),
            ),
        )
        // Just to issue warnings: Scale + NonUniformScale
        .with_query(<(Read<LocalTransform>, Read<Scale>, Read<NonUniformScale>)>::query())
        .build(move |_commands, world, _, queries| {
            let (a, b, c, d, e, f, g, h, i, j, k, l) = queries;
            rayon::scope(|s| {
                s.spawn(|_| unsafe {
                    // Translation
                    a.for_each_unchecked(world, |(mut ltw, translation)| {
                        *ltw = LocalTransform(Mat4::from_translation(translation.0));
                    });
                });
                s.spawn(|_| unsafe {
                    // Rotation
                    b.for_each_unchecked(world, |(mut ltw, rotation)| {
                        *ltw = LocalTransform(Mat4::from_quat(rotation.0));
                    });
                });
                s.spawn(|_| unsafe {
                    // Scale
                    c.for_each_unchecked(world, |(mut ltw, scale)| {
                        *ltw =
                            LocalTransform(Mat4::from_scale(Vec3::new(scale.0, scale.0, scale.0)));
                    });
                });
                s.spawn(|_| unsafe {
                    // NonUniformScale
                    d.for_each_unchecked(world, |(mut ltw, non_uniform_scale)| {
                        *ltw = LocalTransform(Mat4::from_scale(non_uniform_scale.0));
                    });

                    // Translation + Rotation
                    e.for_each_unchecked(world, |(mut ltw, translation, rotation)| {
                        *ltw = LocalTransform(Mat4::from_rotation_translation(
                            rotation.0,
                            translation.0,
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + Scale
                    f.for_each_unchecked(world, |(mut ltw, translation, scale)| {
                        *ltw = LocalTransform(Mat4::from_scale_rotation_translation(
                            Vec3::new(scale.0, scale.0, scale.0),
                            Quat::default(),
                            translation.0,
                        ));
                    });

                    // Translation + NonUniformScale
                    g.for_each_unchecked(world, |(mut ltw, translation, non_uniform_scale)| {
                        *ltw = LocalTransform(Mat4::from_scale_rotation_translation(
                            non_uniform_scale.0,
                            Quat::default(),
                            translation.0,
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Rotation + Scale
                    h.for_each_unchecked(world, |(mut ltw, rotation, scale)| {
                        *ltw = LocalTransform(Mat4::from_scale_rotation_translation(
                            Vec3::new(scale.0, scale.0, scale.0),
                            rotation.0,
                            Vec3::default(),
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Rotation + NonUniformScale
                    i.for_each_unchecked(world, |(mut ltw, rotation, non_uniform_scale)| {
                        *ltw = LocalTransform(Mat4::from_scale_rotation_translation(
                            non_uniform_scale.0,
                            rotation.0,
                            Vec3::default(),
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + Rotation + Scale
                    j.for_each_unchecked(world, |(mut ltw, translation, rotation, scale)| {
                        *ltw = LocalTransform(Mat4::from_scale_rotation_translation(
                            Vec3::new(scale.0, scale.0, scale.0),
                            rotation.0,
                            translation.0,
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + Rotation + NonUniformScale
                    k.for_each_unchecked(
                        world,
                        |(mut ltw, translation, rotation, non_uniform_scale)| {
                            *ltw = LocalTransform(Mat4::from_scale_rotation_translation(
                                non_uniform_scale.0,
                                rotation.0,
                                translation.0,
                            ));
                        },
                    );
                });
            });
            // Just to issue warnings: Scale + NonUniformScale
            l.iter_entities(world)
                .for_each(|(entity, (mut _ltw, _scale, _non_uniform_scale))| {
                    log::warn!(
                        "Entity {:?} has both a Scale and NonUniformScale component.",
                        entity
                    );
                });
        })
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn correct_parent_transformation() {
//         let _ = env_logger::builder().is_test(true).try_init();

//         let mut world = Universe::new().create_world();
//         let system = build(&mut world);

//         let ltw = LocalToParent::identity();
//         let t = Translation::new(1.0, 2.0, 3.0);
//         let r = Rotation::from_euler_angles(1.0, 2.0, 3.0);
//         let s = Scale(2.0);
//         let nus = NonUniformScale::new(1.0, 2.0, 3.0);

//         // Add every combination of transform types.
//         let translation = *world.insert((), vec![(ltw, t)]).first().unwrap();
//         let rotation = *world.insert((), vec![(ltw, r)]).first().unwrap();
//         let scale = *world.insert((), vec![(ltw, s)]).first().unwrap();
//         let non_uniform_scale = *world.insert((), vec![(ltw, nus)]).first().unwrap();
//         let translation_and_rotation = *world.insert((), vec![(ltw, t, r)]).first().unwrap();
//         let translation_and_scale = *world.insert((), vec![(ltw, t, s)]).first().unwrap();
//         let translation_and_nus = *world.insert((), vec![(ltw, t, nus)]).first().unwrap();
//         let rotation_scale = *world.insert((), vec![(ltw, r, s)]).first().unwrap();
//         let rotation_nus = *world.insert((), vec![(ltw, r, nus)]).first().unwrap();
//         let translation_rotation_scale = *world.insert((), vec![(ltw, t, r, s)]).first().unwrap();
//         let translation_rotation_nus = *world.insert((), vec![(ltw, t, r, nus)]).first().unwrap();

//         // Run the system
//         system.run(&mut world);
//         system.command_buffer_mut().write(&mut world);

//         // Verify that each was transformed correctly.
//         assert_eq!(
//             world.get_component::<LocalToParent>(translation).unwrap().0,
//             Mat4::from_translation(t.0)
//         );
//         assert_eq!(
//             world.get_component::<LocalToParent>(rotation).unwrap().0,
//             Mat4::from_quat(r.0)
//         );
//         assert_eq!(
//             world.get_component::<LocalToParent>(scale).unwrap().0,
//             Mat4::from_scale(Vec3::new(s.0, s.0, s.0))
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(non_uniform_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale(nus.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(translation_and_rotation)
//                 .unwrap()
//                 .0,
//             Mat4::from_rotation_translation(r.0, t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(translation_and_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), Quat::default(), t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(translation_and_nus)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(nus.0, Quat::default(), t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(rotation_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, Vec3::default())
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(rotation_nus)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(nus.0, r.0, Vec3::default())
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(translation_rotation_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToParent>(translation_rotation_nus)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(nus.0, r.0, t.0)
//         );
//     }
// }
