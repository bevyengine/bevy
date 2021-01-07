use std::{mem::drop, thread::sleep, time::Duration};

use bevy::{
    animation::{Animator, Clip},
    asset::LoadState,
    prelude::*,
    render::{pipeline::RenderPipelines, render_graph::base::MainPass},
};

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, Criterion, ParameterizedBenchmark,
};

const LEN: usize = 1_000;
const TICKS: usize = 1_000;
const WARM_UP_TIME: Duration = Duration::from_secs(5);
const MEASUREMENT_TIME: Duration = Duration::from_secs(35);

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "animation",
        ParameterizedBenchmark::new(
            "tick",
            |b, _| {
                b.iter_batched(
                    run_idle_setup_bench,
                    |mut value| {
                        black_box(value.run(black_box(TICKS)));
                    },
                    BatchSize::NumIterations(LEN as u64),
                )
            },
            vec![()],
        )
        .warm_up_time(WARM_UP_TIME)
        .measurement_time(MEASUREMENT_TIME),
    );
}

fn run_idle_setup_bench() -> bevy::animation::Bench {
    let mut asset_folder = std::env::current_dir().unwrap();
    asset_folder.pop();
    let mut asset_folder = asset_folder.to_str().unwrap().to_owned();
    asset_folder.push_str("/assets");

    let mut bench = bevy::animation::Bench::build(Some(asset_folder));
    bench
        .builder
        // To load
        .add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::gltf::GltfPlugin::default())
        // These are need to load the GLTF
        .add_asset::<Mesh>()
        .register_type::<Draw>()
        .register_type::<MainPass>()
        .register_type::<Visible>()
        .register_type::<RenderPipelines>()
        .add_asset::<StandardMaterial>();

    bench.warm();

    let app = &mut bench.builder.app;
    let asset_server = app.resources.get::<AssetServer>().unwrap();
    let scene: Handle<Scene> = asset_server.load("models/character_medium/character_medium.gltf");
    let idle: Handle<Clip> = asset_server.load("models/character_medium/idle.gltf#Anim0");
    let run: Handle<Clip> = asset_server.load("models/character_medium/run.gltf#Anim0");
    drop(asset_server);

    let t = Duration::from_millis(50);
    for h in &[
        scene.clone_untyped(),
        idle.clone_untyped(),
        run.clone_untyped(),
    ] {
        loop {
            app.update();

            let asset_server = app.resources.get::<AssetServer>().unwrap();
            let state = asset_server.get_load_state(h);
            drop(asset_server);

            // println!("{:?}", state);
            if state == LoadState::Loaded {
                break;
            }
            sleep(t);
        }
    }

    let mut spawner = app.resources.get_mut::<SceneSpawner>().unwrap();
    spawner.spawn(scene);
    drop(spawner);
    app.update();

    let mut ok = false;
    for (mut animator,) in app.world.query_mut::<(&mut Animator,)>() {
        animator.add_layer(idle.clone(), 0.5);
        animator.add_layer(run.clone(), 0.5);
        ok = true;
    }
    assert!(ok, "scene not spawned or doesn't contain an animator");

    bench
}

// fn simple_setup_bench() -> bevy::animation::Bench {
//     let mut b = bevy::animation::Bench::build(None);

//     let mut world = World::new();
//     let mut world_builder = world.build();
//     let base = (
//         GlobalTransform::default(),
//         Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
//     );

//     // Create animator and assign some clips
//     let mut animator = Animator::default();
//     {
//         let mut clip_a = Clip::default();
//         clip_a.add_curve_at_path(
//             "@Transform.translation",
//             Curve::from_line(0.0, 1.0, Vec3::unit_x(), -Vec3::unit_x()),
//         );
//         let rot = Curve::from_constant(Quat::identity());
//         clip_a.add_curve_at_path("@Transform.rotation", rot.clone());
//         clip_a.add_curve_at_path("/Hode1@Transform.rotation", rot.clone());
//         clip_a.add_curve_at_path("/Node1/Node2@Transform.rotation", rot);

//         let mut clip_b = Clip::default();
//         clip_b.add_curve_at_path("@Transform.translation", Curve::from_constant(Vec3::zero()));
//         let rot = Curve::from_line(
//             0.0,
//             1.0,
//             Quat::from_axis_angle(Vec3::unit_z(), 0.1),
//             Quat::from_axis_angle(Vec3::unit_z(), -0.1),
//         );
//         clip_b.add_curve_at_path("@Transform.rotation", rot.clone());
//         clip_b.add_curve_at_path("/Hode1@Transform.rotation", rot.clone());
//         clip_b.add_curve_at_path("/Node1/Node2@Transform.rotation", rot);

//         let mut clips = b.builder.resources_mut().get_mut::<Assets<Clip>>().unwrap();
//         let clip_a = clips.add(clip_a);
//         let clip_b = clips.add(clip_b);

//         animator.add_layer(clip_a, 0.5);
//         animator.add_layer(clip_b, 0.5);
//     }

//     world_builder
//         .spawn(base.clone())
//         .with(Name::from_str("Root"))
//         .with(animator)
//         .with_children(|world_builder| {
//             world_builder
//                 .spawn(base.clone())
//                 .with(Name::from_str("Node1"))
//                 .with_children(|world_builder| {
//                     world_builder
//                         .spawn(base.clone())
//                         .with(Name::from_str("Node2"))
//                         .with_children(|world_builder| {
//                             world_builder
//                                 .spawn(base.clone())
//                                 .with(Name::from_str("Node3"));
//                         });
//                 });
//         });

//     b.builder.set_world(world);
//     b.warm();

//     b
// }

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
