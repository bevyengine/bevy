use bevy_animation::prelude::*;
use bevy_asset::prelude::*;
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::prelude::*;
use core::time::Duration;

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, Criterion, ParameterizedBenchmark,
};

const LEN: usize = 100_000;
const TICKS: usize = 100_000;
const WARM_UP_TIME: Duration = Duration::from_secs(5);
const MEASUREMENT_TIME: Duration = Duration::from_secs(55);

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "animator",
        ParameterizedBenchmark::new(
            "update",
            |b, _| {
                b.iter_batched(
                    || AnimtorTestBench::new(),
                    |mut value| black_box(value.update(black_box(TICKS))),
                    BatchSize::NumIterations(LEN as u64),
                )
            },
            vec![()],
        )
        .warm_up_time(WARM_UP_TIME)
        .measurement_time(MEASUREMENT_TIME),
    );
}

struct AnimtorTestBench {
    app: bevy_app::App,
    //binding_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>>,
    update_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>>,
}

impl AnimtorTestBench {
    fn new() -> Self {
        let mut app_builder = bevy_app::App::build();
        app_builder
            .add_plugin(bevy_type_registry::TypeRegistryPlugin::default())
            .add_plugin(bevy_core::CorePlugin::default())
            .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
            .add_plugin(bevy_asset::AssetPlugin)
            .add_plugin(bevy_transform::TransformPlugin)
            .add_plugin(bevy_animation::AnimationPlugin);

        let mut world = World::new();
        let mut world_builder = world.build();
        let base = (
            GlobalTransform::default(),
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
        );

        // Create animator and assign some clips
        let mut animator = Animator::default();
        {
            let mut clip_a = Clip::default();
            clip_a.add_animated_prop(
                "@Transform.translation",
                CurveUntyped::Vec3(Curve::from_linear(
                    0.0,
                    1.0,
                    Vec3::unit_x(),
                    -Vec3::unit_x(),
                )),
            );
            let rot = CurveUntyped::Quat(Curve::from_constant(Quat::identity()));
            clip_a.add_animated_prop("@Transform.rotation", rot.clone());
            clip_a.add_animated_prop("/Hode1@Transform.rotation", rot.clone());
            clip_a.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

            let mut clip_b = Clip::default();
            clip_b.add_animated_prop(
                "@Transform.translation",
                CurveUntyped::Vec3(Curve::from_constant(Vec3::zero())),
            );
            let rot = CurveUntyped::Quat(Curve::from_linear(
                0.0,
                1.0,
                Quat::from_axis_angle(Vec3::unit_z(), 0.1),
                Quat::from_axis_angle(Vec3::unit_z(), -0.1),
            ));
            clip_b.add_animated_prop("@Transform.rotation", rot.clone());
            clip_b.add_animated_prop("/Hode1@Transform.rotation", rot.clone());
            clip_b.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

            let mut clips = app_builder
                .resources_mut()
                .get_mut::<Assets<Clip>>()
                .unwrap();
            let clip_a = clips.add(clip_a);
            let clip_b = clips.add(clip_b);

            animator.add_layer(clip_a, 0.5);
            animator.add_layer(clip_b, 0.5);
        }

        world_builder
            .spawn(base.clone())
            .with(Name::from_str("Root"))
            .with(animator)
            .with_children(|world_builder| {
                world_builder
                    .spawn(base.clone())
                    .with(Name::from_str("Node1"))
                    .with_children(|world_builder| {
                        world_builder
                            .spawn(base.clone())
                            .with(Name::from_str("Node2"))
                            .with_children(|world_builder| {
                                world_builder
                                    .spawn(base.clone())
                                    .with(Name::from_str("Node3"));
                            });
                    });
            });

        app_builder.set_world(world);

        // let mut parent_update_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>> =
        //     Box::new(bevy_transform::hierarchy::parent_update_system.system());

        let mut binding_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>> =
            Box::new(bevy_animation::animator_binding_system.system());

        let update_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>> =
            Box::new(bevy_animation::animator_update_system.system());

        let mut app = app_builder.app;

        //parent_update_system.run((), &mut app.world, &mut app.resources);
        binding_system.run_thread_local(&mut app.world, &mut app.resources);

        Self { app, update_system }
    }

    fn update(&mut self, ticks: usize) {
        for _ in 0..ticks {
            // Time tick
            {
                let mut time = self.app.resources.get_mut::<Time>().unwrap();
                time.delta_seconds += 0.016;
                time.delta_seconds_f64 += 0.016;
            }

            black_box(
                self.update_system
                    .run_thread_local(&mut self.app.world, &mut self.app.resources),
            );

            //dbg!(app.world.get::<Transform>(root_entity).unwrap());
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
