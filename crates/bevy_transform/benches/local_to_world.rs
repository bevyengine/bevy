// #![feature(test)]

// extern crate test;

// use legion::prelude::*;
// use bevy_transform::{transform_system, prelude::*};
// use test::Bencher;

// #[bench]
// fn transform_update_without_change(b: &mut Bencher) {
//     let _ = env_logger::builder().is_test(true).try_init();

//     let mut world = Universe::new().create_world();
//     let system = transform_system::build(&mut world);

//     let ltw = LocalToWorld::identity();
//     let t = Translation::new(1.0, 2.0, 3.0);
//     let r = Rotation::from_euler_angles(1.0, 2.0, 3.0);
//     let s = Scale(2.0);
//     let nus = NonUniformScale::new(1.0, 2.0, 3.0);

//     // Add N of every combination of transform types.
//     let n = 1000;
//     let _translation = *world.insert((), vec![(ltw, t); n]).first().unwrap();
//     let _rotation = *world.insert((), vec![(ltw, r); n]).first().unwrap();
//     let _scale = *world.insert((), vec![(ltw, s); n]).first().unwrap();
//     let _non_uniform_scale = *world.insert((), vec![(ltw, nus); n]).first().unwrap();
//     let _translation_and_rotation = *world.insert((), vec![(ltw, t, r); n]).first().unwrap();
//     let _translation_and_scale = *world.insert((), vec![(ltw, t, s); n]).first().unwrap();
//     let _translation_and_nus = *world.insert((), vec![(ltw, t, nus); n]).first().unwrap();
//     let _rotation_scale = *world.insert((), vec![(ltw, r, s); n]).first().unwrap();
//     let _rotation_nus = *world.insert((), vec![(ltw, r, nus); n]).first().unwrap();
//     let _translation_rotation_scale = *world.insert((), vec![(ltw, t, r, s); n]).first().unwrap();
//     let _translation_rotation_nus = *world.insert((), vec![(ltw, t, r, nus); n]).first().unwrap();

//     // Run the system once outside the test (which should compute everything and it shouldn't be
//     // touched again).
//     system.run(&mut world);
//     system.command_buffer_mut().write(&mut world);

//     // Then time the already-computed updates.
//     b.iter(|| {
//         system.run(&mut world);
//         system.command_buffer_mut().write(&mut world);
//     });
// }
