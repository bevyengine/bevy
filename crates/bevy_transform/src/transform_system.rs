#![allow(dead_code)]
use crate::{
    components::*,
    ecs::prelude::*,
    math::{Mat4, Quat, Vec3},
};

pub fn build(_: &mut World) -> Box<dyn Schedulable> {
    SystemBuilder::<()>::new("LocalToWorldUpdateSystem")
        // Translation
        .with_query(<(Write<Transform>, Read<Translation>)>::query().filter(
            !component::<Parent>()
                & !component::<Rotation>()
                & !component::<Scale>()
                & !component::<NonUniformScale>()
                & (changed::<Translation>()),
        ))
        // Rotation
        .with_query(<(Write<Transform>, Read<Rotation>)>::query().filter(
            !component::<Parent>()
                & !component::<Translation>()
                & !component::<Scale>()
                & !component::<NonUniformScale>()
                & (changed::<Rotation>()),
        ))
        // Scale
        .with_query(<(Write<Transform>, Read<Scale>)>::query().filter(
            !component::<Parent>()
                & !component::<Translation>()
                & !component::<Rotation>()
                & !component::<NonUniformScale>()
                & (changed::<Scale>()),
        ))
        // NonUniformScale
        .with_query(<(Write<Transform>, Read<NonUniformScale>)>::query().filter(
            !component::<Parent>()
                & !component::<Translation>()
                & !component::<Rotation>()
                & !component::<Scale>()
                & (changed::<NonUniformScale>()),
        ))
        // Translation + Rotation
        .with_query(
            <(Write<Transform>, Read<Translation>, Read<Rotation>)>::query().filter(
                !component::<Parent>()
                    & !component::<Scale>()
                    & !component::<NonUniformScale>()
                    & (changed::<Translation>() | changed::<Rotation>()),
            ),
        )
        // Translation + Scale
        .with_query(
            <(Write<Transform>, Read<Translation>, Read<Scale>)>::query().filter(
                !component::<Parent>()
                    & !component::<Rotation>()
                    & !component::<NonUniformScale>()
                    & (changed::<Translation>() | changed::<Scale>()),
            ),
        )
        // Translation + NonUniformScale
        .with_query(
            <(Write<Transform>, Read<Translation>, Read<NonUniformScale>)>::query().filter(
                !component::<Parent>()
                    & !component::<Rotation>()
                    & !component::<Scale>()
                    & (changed::<Translation>() | changed::<NonUniformScale>()),
            ),
        )
        // Rotation + Scale
        .with_query(
            <(Write<Transform>, Read<Rotation>, Read<Scale>)>::query().filter(
                !component::<Parent>()
                    & !component::<Translation>()
                    & !component::<NonUniformScale>()
                    & (changed::<Rotation>() | changed::<Scale>()),
            ),
        )
        // Rotation + NonUniformScale
        .with_query(
            <(Write<Transform>, Read<Rotation>, Read<NonUniformScale>)>::query().filter(
                !component::<Parent>()
                    & !component::<Translation>()
                    & !component::<Scale>()
                    & (changed::<Rotation>() | changed::<NonUniformScale>()),
            ),
        )
        // Translation + Rotation + Scale
        .with_query(
            <(
                Write<Transform>,
                Read<Translation>,
                Read<Rotation>,
                Read<Scale>,
            )>::query()
            .filter(
                !component::<Parent>()
                    & !component::<NonUniformScale>()
                    & (changed::<Translation>() | changed::<Rotation>() | changed::<Scale>()),
            ),
        )
        // Translation + Rotation + NonUniformScale
        .with_query(
            <(
                Write<Transform>,
                Read<Translation>,
                Read<Rotation>,
                Read<NonUniformScale>,
            )>::query()
            .filter(
                !component::<Parent>()
                    & !component::<Scale>()
                    & (changed::<Translation>()
                        | changed::<Rotation>()
                        | changed::<NonUniformScale>()),
            ),
        )
        // Just to issue warnings: Scale + NonUniformScale
        .with_query(
            <(Read<Transform>, Read<Scale>, Read<NonUniformScale>)>::query()
                .filter(!component::<Parent>()),
        )
        .build(move |_commands, world, _, queries| {
            let (a, b, c, d, e, f, g, h, i, j, k, l) = queries;
            rayon::scope(|s| {
                s.spawn(|_| unsafe {
                    // Translation
                    a.for_each_unchecked(world, |(mut ltw, translation)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_translation(translation.0));
                    });
                });
                s.spawn(|_| unsafe {
                    // Rotation
                    b.for_each_unchecked(world, |(mut ltw, rotation)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_quat(rotation.0));
                    });
                });
                s.spawn(|_| unsafe {
                    // Scale
                    c.for_each_unchecked(world, |(mut ltw, scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw =
                            Transform::new(Mat4::from_scale(Vec3::new(scale.0, scale.0, scale.0)));
                    });
                });
                s.spawn(|_| unsafe {
                    // NonUniformScale
                    d.for_each_unchecked(world, |(mut ltw, non_uniform_scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_scale(non_uniform_scale.0));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + Rotation
                    e.for_each_unchecked(world, |(mut ltw, translation, rotation)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_rotation_translation(
                            rotation.0,
                            translation.0,
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + Scale
                    f.for_each_unchecked(world, |(mut ltw, translation, scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_scale_rotation_translation(
                            Vec3::new(scale.0, scale.0, scale.0),
                            Quat::default(),
                            translation.0,
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + NonUniformScale
                    g.for_each_unchecked(world, |(mut ltw, translation, non_uniform_scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_scale_rotation_translation(
                            non_uniform_scale.0,
                            Quat::default(),
                            translation.0,
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Rotation + Scale
                    h.for_each_unchecked(world, |(mut ltw, rotation, scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_scale_rotation_translation(
                            Vec3::new(scale.0, scale.0, scale.0),
                            rotation.0,
                            Vec3::default(),
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Rotation + NonUniformScale
                    i.for_each_unchecked(world, |(mut ltw, rotation, non_uniform_scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_scale_rotation_translation(
                            non_uniform_scale.0,
                            rotation.0,
                            Vec3::default(),
                        ));
                    });
                });
                s.spawn(|_| unsafe {
                    // Translation + Rotation + Scale
                    j.for_each_unchecked(world, |(mut ltw, translation, rotation, scale)| {
                        if !ltw.sync {
                            return;
                        }
                        *ltw = Transform::new(Mat4::from_scale_rotation_translation(
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
                            if !ltw.sync {
                                return;
                            }
                            *ltw = Transform::new(Mat4::from_scale_rotation_translation(
                                non_uniform_scale.0,
                                rotation.0,
                                translation.0,
                            ));
                        },
                    );
                });

                // Just to issue warnings: Scale + NonUniformScale
                #[allow(unused_unsafe)]
                unsafe {
                    l.iter_entities(world).for_each(
                        |(entity, (mut _ltw, _scale, _non_uniform_scale))| {
                            log::warn!(
                                "Entity {:?} has both a Scale and NonUniformScale component.",
                                entity
                            );
                        },
                    );
                }
            });
        })
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::math::{Mat4, Quat, Vec3};

//     #[test]
//     fn correct_world_transformation() {
//         let _ = env_logger::builder().is_test(true).try_init();

//         let mut world = Universe::new().create_world();
//         let system = build(&mut world);

//         let ltw = LocalToWorld::identity();
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
//             world.get_component::<LocalToWorld>(translation).unwrap().0,
//             Mat4::from_translation(t.0)
//         );
//         assert_eq!(
//             world.get_component::<LocalToWorld>(rotation).unwrap().0,
//             Mat4::from_quat(r.0)
//         );
//         assert_eq!(
//             world.get_component::<LocalToWorld>(scale).unwrap().0,
//             Mat4::from_scale(Vec3::new(s.0, s.0, s.0))
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(non_uniform_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale(nus.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(translation_and_rotation)
//                 .unwrap()
//                 .0,
//             Mat4::from_rotation_translation(r.0, t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(translation_and_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), Quat::default(), t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(translation_and_nus)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(nus.0, Quat::default(), t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(rotation_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, Vec3::default())
//         );
//         assert_eq!(
//             world.get_component::<LocalToWorld>(rotation_nus).unwrap().0,
//             Mat4::from_scale_rotation_translation(nus.0, r.0, Vec3::default())
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(translation_rotation_scale)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(Vec3::new(s.0, s.0, s.0), r.0, t.0)
//         );
//         assert_eq!(
//             world
//                 .get_component::<LocalToWorld>(translation_rotation_nus)
//                 .unwrap()
//                 .0,
//             Mat4::from_scale_rotation_translation(nus.0, r.0, t.0)
//         );
//     }
// }
