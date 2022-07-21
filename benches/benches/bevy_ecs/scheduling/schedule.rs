use bevy_ecs::prelude::*;
use criterion::Criterion;
use rand::RngCore;

pub fn schedule(c: &mut Criterion) {
    #[derive(Component)]
    struct A(f32);
    #[derive(Component)]
    struct B(f32);
    #[derive(Component)]
    struct C(f32);
    #[derive(Component)]
    struct D(f32);
    #[derive(Component)]
    struct E(f32);

    fn ab(mut query: Query<(&mut A, &mut B)>) {
        query.for_each_mut(|(mut a, mut b)| {
            std::mem::swap(&mut a.0, &mut b.0);
        });
    }

    fn cd(mut query: Query<(&mut C, &mut D)>) {
        query.for_each_mut(|(mut c, mut d)| {
            std::mem::swap(&mut c.0, &mut d.0);
        });
    }

    fn ce(mut query: Query<(&mut C, &mut E)>) {
        query.for_each_mut(|(mut c, mut e)| {
            std::mem::swap(&mut c.0, &mut e.0);
        });
    }

    let mut group = c.benchmark_group("schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut world = World::default();

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0), D(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0), E(0.0))));

        let mut stage = SystemStage::parallel();
        stage.add_system(ab);
        stage.add_system(cd);
        stage.add_system(ce);
        stage.run(&mut world);

        b.iter(move || stage.run(&mut world));
    });
    group.finish();
}

/// performs takes a value out of a reference, applies a fn, and puts it back in.
/// stores a temporary dummy value while performing the operation.
fn map_with_temp<T>(ptr: &mut T, temp: T, f: impl FnOnce(T) -> T) {
    let val = std::mem::replace(ptr, temp);
    *ptr = f(val);
}

pub fn build_schedule(criterion: &mut Criterion) {
    use bevy_ecs::{
        prelude::*,
        schedule::{ParallelSystemDescriptor, SystemLabelId},
    };

    // Simulates a plugin that has a decent number of systems.
    // Systems have interdependencies within plugins,
    // as well as with public labels exported by other plugins.
    // Also, sometimes entire plugins have dependencies with one another, via the plugin's own label.
    struct Plugin {
        label: SystemLabelId,
        systems: [ParallelSystemDescriptor; 20],
        pub_labels: [SystemLabelId; 4],
    }

    #[derive(SystemLabel)]
    struct PluginLabel<const I: usize>;

    #[derive(SystemLabel)]
    enum PubLabel<const P: usize> {
        Short,
        LongName,
        ReallyLongName,
        ReallyReallySuperLongName,
    }

    fn my_system<const P: usize, const I: usize>() {}

    // chance of there being a dependency between any two plugins.
    const PLUGIN_DEP_CHANCE: u32 = 5;
    // chance of there being a dependency between any two systems within a plugin.
    const INNER_DEP_CHANCE: u32 = 30;
    // chance for each system in a plugin to have any given public label.
    const PUB_LABEL_CHANCE: u32 = 25;
    // chance of there being a dependency between any system and another plugin's public labels
    const OUTER_DEP_CHANCE: u32 = 10;

    impl Plugin {
        fn new<const I: usize>(rng: &mut impl RngCore) -> Self {
            let plugin_label = PluginLabel::<I>.as_label();

            let pub_labels = [
                PubLabel::<I>::Short.as_label(),
                PubLabel::<I>::LongName.as_label(),
                PubLabel::<I>::ReallyLongName.as_label(),
                PubLabel::<I>::ReallyReallySuperLongName.as_label(),
            ];

            // Initialize a list of systems with unique types.
            macro_rules! declare_systems {
                ($($J:literal),*) => {
                    [$(my_system::<I, $J>),*]
                };
            }
            let systems = declare_systems![
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
            ];

            // apply the plugin's label to each system.
            let systems = systems.map(|s| s.label(plugin_label));

            let mut i = 0;
            let systems = systems.map(|mut system| {
                // have a chance to form a dependency with every other system in this plugin.
                macro_rules! maybe_dep {
                    ($J:literal) => {
                        if i != $J && rng.next_u32() % 100 < INNER_DEP_CHANCE {
                            if i < $J {
                                system = system.before(my_system::<I, $J>);
                            } else {
                                system = system.after(my_system::<I, $J>);
                            }
                        }
                    };
                    ($($J:literal),*) => {
                        $(maybe_dep!($J);)*
                    }
                }
                maybe_dep!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19);

                // have a chance to add public labels.
                for &label in &pub_labels {
                    if rng.next_u32() % 100 < PUB_LABEL_CHANCE {
                        system = system.label(label);
                    }
                }

                i += 1;

                system
            });

            Self {
                label: plugin_label,
                systems,
                pub_labels,
            }
        }
    }

    // simulates an app with many plugins.
    struct Experiment {
        plugins: Vec<Plugin>,
    }

    impl Experiment {
        fn new(plugins: impl IntoIterator<Item = Plugin>, rng: &mut impl RngCore) -> Self {
            let mut plugins: Vec<_> = plugins.into_iter().collect();

            // Form inter-plugin dependencies
            for i in 0..plugins.len() {
                let (before, after) = plugins.split_at_mut(i);
                let (plugin, after) = after.split_first_mut().unwrap();

                // Have a chance to form a dependency with plugins coming before this one
                for other in before.iter() {
                    if rng.next_u32() % 100 < PLUGIN_DEP_CHANCE {
                        for system in &mut plugin.systems {
                            map_with_temp(system, my_system::<0, 0>.label(PluginLabel::<0>), |s| {
                                s.after(other.label)
                            });
                        }
                    }
                }
                // Have a chance to form a dependency with plugins coming after this one
                for other in after.iter() {
                    if rng.next_u32() % 100 < PLUGIN_DEP_CHANCE {
                        for system in &mut plugin.systems {
                            map_with_temp(system, my_system::<0, 0>.label(PluginLabel::<0>), |s| {
                                s.before(other.label)
                            });
                        }
                    }
                }

                // Have a chance for every system in the plugin to form a dependency
                // with every public label from every other plugin.
                for system in &mut plugin.systems {
                    for &other_label in before.iter().flat_map(|other| &other.pub_labels) {
                        if rng.next_u32() % 100 < OUTER_DEP_CHANCE {
                            map_with_temp(system, my_system::<0, 0>.label(PluginLabel::<0>), |s| {
                                s.after(other_label)
                            });
                        }
                    }
                    for &other_label in after.iter().flat_map(|other| &other.pub_labels) {
                        if rng.next_u32() % 100 < OUTER_DEP_CHANCE {
                            map_with_temp(system, my_system::<0, 0>.label(PluginLabel::<0>), |s| {
                                s.before(other_label)
                            });
                        }
                    }
                }
            }

            Self { plugins }
        }
        fn write_to(self, stage: &mut SystemStage) {
            for plugin in self.plugins {
                for system in plugin.systems {
                    stage.add_system(system);
                }
            }
        }
    }

    let mut group = criterion.benchmark_group("build_schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(15));

    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::seed_from_u64(5410);

    macro_rules! experiment {
        ($($N:literal),* $(,)?) => {{
            // this runs outside of the benchmark so we don't need to worry about `Vec::with_capacity`.
            let mut plugins = Vec::new();
            // these must be pushed one by one to avoid overflowing the stack.
            $( plugins.push(Plugin::new::<$N>(&mut rng)) ;)*
            Experiment::new(plugins, &mut rng)
        }}
    }

    group.bench_function("schedule 10 plugins", |bencher| {
        let mut world = World::new();
        bencher.iter_batched(
            || experiment!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9),
            |experiment| {
                let mut stage = SystemStage::parallel();
                experiment.write_to(&mut stage);
                stage.run(&mut world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("schedule 50 plugins", |bencher| {
        let mut world = World::new();
        bencher.iter_batched(
            || {
                experiment!(
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
                    22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41,
                    42, 43, 44, 45, 46, 47, 48, 49,
                )
            },
            |experiment| {
                let mut stage = SystemStage::parallel();
                experiment.write_to(&mut stage);
                stage.run(&mut world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("schedule 100 plugins", |bencher| {
        let mut world = World::new();
        bencher.iter_batched(
            || {
                experiment!(
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
                    22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41,
                    42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61,
                    62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81,
                    82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99,
                )
            },
            |experiment| {
                let mut stage = SystemStage::parallel();
                experiment.write_to(&mut stage);
                stage.run(&mut world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}
