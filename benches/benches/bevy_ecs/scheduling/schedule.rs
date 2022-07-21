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
        fn new<const I: usize>() -> Self {
            let plugin_label = PluginLabel::<I>.as_label();

            let pub_labels = [
                PubLabel::<I>::Short.as_label(),
                PubLabel::<I>::LongName.as_label(),
                PubLabel::<I>::ReallyLongName.as_label(),
                PubLabel::<I>::ReallyReallySuperLongName.as_label(),
            ];

            // Initialize a list of systems with unique types.
            let systems = [
                my_system::<I, 0>,
                my_system::<I, 1>,
                my_system::<I, 2>,
                my_system::<I, 3>,
                my_system::<I, 4>,
                my_system::<I, 5>,
                my_system::<I, 6>,
                my_system::<I, 7>,
                my_system::<I, 8>,
                my_system::<I, 9>,
                my_system::<I, 10>,
                my_system::<I, 11>,
                my_system::<I, 12>,
                my_system::<I, 13>,
                my_system::<I, 14>,
                my_system::<I, 15>,
                my_system::<I, 16>,
                my_system::<I, 17>,
                my_system::<I, 18>,
                my_system::<I, 19>,
            ];
            let systems = systems.map(|s| s.label(plugin_label));

            let mut rng = rand::thread_rng();

            let mut i = 0;
            let systems = systems.map(|mut system| {
                macro_rules! maybe_dep {
                    ($P:ident, $J:literal) => {
                        if i != $J && rng.next_u32() % 100 < INNER_DEP_CHANCE {
                            if i < $J {
                                system = system.before(my_system::<$P, $J>);
                            } else {
                                system = system.after(my_system::<$P, $J>);
                            }
                        }
                    };
                }

                // have a chance to form a dependency with every other system in this plugin.
                maybe_dep!(I, 0);
                maybe_dep!(I, 1);
                maybe_dep!(I, 2);
                maybe_dep!(I, 3);
                maybe_dep!(I, 4);
                maybe_dep!(I, 5);
                maybe_dep!(I, 6);
                maybe_dep!(I, 7);
                maybe_dep!(I, 8);
                maybe_dep!(I, 9);
                maybe_dep!(I, 10);
                maybe_dep!(I, 11);
                maybe_dep!(I, 12);
                maybe_dep!(I, 13);
                maybe_dep!(I, 14);
                maybe_dep!(I, 15);
                maybe_dep!(I, 16);
                maybe_dep!(I, 17);
                maybe_dep!(I, 18);
                maybe_dep!(I, 19);

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
        fn new(plugins: impl IntoIterator<Item = Plugin>) -> Self {
            let mut plugins: Vec<_> = plugins.into_iter().collect();

            let mut rng = rand::thread_rng();

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

    group.bench_function("schedule 10 plugins", |bencher| {
        let mut world = World::new();
        bencher.iter_batched(
            || {
                // these must be pushed one by one to avoid overflowing the stack.
                let mut plugins = Vec::with_capacity(10);
                plugins.push(Plugin::new::<0>());
                plugins.push(Plugin::new::<1>());
                plugins.push(Plugin::new::<2>());
                plugins.push(Plugin::new::<3>());
                plugins.push(Plugin::new::<4>());
                plugins.push(Plugin::new::<5>());
                plugins.push(Plugin::new::<6>());
                plugins.push(Plugin::new::<7>());
                plugins.push(Plugin::new::<8>());
                plugins.push(Plugin::new::<9>());
                Experiment::new(plugins)
            },
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
                let mut plugins = Vec::with_capacity(10);
                plugins.push(Plugin::new::<0>());
                plugins.push(Plugin::new::<1>());
                plugins.push(Plugin::new::<2>());
                plugins.push(Plugin::new::<3>());
                plugins.push(Plugin::new::<4>());
                plugins.push(Plugin::new::<5>());
                plugins.push(Plugin::new::<6>());
                plugins.push(Plugin::new::<7>());
                plugins.push(Plugin::new::<8>());
                plugins.push(Plugin::new::<9>());
                plugins.push(Plugin::new::<10>());
                plugins.push(Plugin::new::<11>());
                plugins.push(Plugin::new::<12>());
                plugins.push(Plugin::new::<13>());
                plugins.push(Plugin::new::<14>());
                plugins.push(Plugin::new::<15>());
                plugins.push(Plugin::new::<16>());
                plugins.push(Plugin::new::<17>());
                plugins.push(Plugin::new::<18>());
                plugins.push(Plugin::new::<19>());
                plugins.push(Plugin::new::<20>());
                plugins.push(Plugin::new::<21>());
                plugins.push(Plugin::new::<22>());
                plugins.push(Plugin::new::<23>());
                plugins.push(Plugin::new::<24>());
                plugins.push(Plugin::new::<25>());
                plugins.push(Plugin::new::<26>());
                plugins.push(Plugin::new::<27>());
                plugins.push(Plugin::new::<28>());
                plugins.push(Plugin::new::<29>());
                plugins.push(Plugin::new::<30>());
                plugins.push(Plugin::new::<31>());
                plugins.push(Plugin::new::<32>());
                plugins.push(Plugin::new::<33>());
                plugins.push(Plugin::new::<34>());
                plugins.push(Plugin::new::<35>());
                plugins.push(Plugin::new::<36>());
                plugins.push(Plugin::new::<37>());
                plugins.push(Plugin::new::<38>());
                plugins.push(Plugin::new::<39>());
                plugins.push(Plugin::new::<40>());
                plugins.push(Plugin::new::<41>());
                plugins.push(Plugin::new::<42>());
                plugins.push(Plugin::new::<43>());
                plugins.push(Plugin::new::<44>());
                plugins.push(Plugin::new::<45>());
                plugins.push(Plugin::new::<46>());
                plugins.push(Plugin::new::<47>());
                plugins.push(Plugin::new::<48>());
                plugins.push(Plugin::new::<49>());
                plugins.push(Plugin::new::<50>());
                Experiment::new(plugins)
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
                let mut plugins = Vec::with_capacity(10);
                plugins.push(Plugin::new::<0>());
                plugins.push(Plugin::new::<1>());
                plugins.push(Plugin::new::<2>());
                plugins.push(Plugin::new::<3>());
                plugins.push(Plugin::new::<4>());
                plugins.push(Plugin::new::<5>());
                plugins.push(Plugin::new::<6>());
                plugins.push(Plugin::new::<7>());
                plugins.push(Plugin::new::<8>());
                plugins.push(Plugin::new::<9>());
                plugins.push(Plugin::new::<10>());
                plugins.push(Plugin::new::<11>());
                plugins.push(Plugin::new::<12>());
                plugins.push(Plugin::new::<13>());
                plugins.push(Plugin::new::<14>());
                plugins.push(Plugin::new::<15>());
                plugins.push(Plugin::new::<16>());
                plugins.push(Plugin::new::<17>());
                plugins.push(Plugin::new::<18>());
                plugins.push(Plugin::new::<19>());
                plugins.push(Plugin::new::<20>());
                plugins.push(Plugin::new::<21>());
                plugins.push(Plugin::new::<22>());
                plugins.push(Plugin::new::<23>());
                plugins.push(Plugin::new::<24>());
                plugins.push(Plugin::new::<25>());
                plugins.push(Plugin::new::<26>());
                plugins.push(Plugin::new::<27>());
                plugins.push(Plugin::new::<28>());
                plugins.push(Plugin::new::<29>());
                plugins.push(Plugin::new::<30>());
                plugins.push(Plugin::new::<31>());
                plugins.push(Plugin::new::<32>());
                plugins.push(Plugin::new::<33>());
                plugins.push(Plugin::new::<34>());
                plugins.push(Plugin::new::<35>());
                plugins.push(Plugin::new::<36>());
                plugins.push(Plugin::new::<37>());
                plugins.push(Plugin::new::<38>());
                plugins.push(Plugin::new::<39>());
                plugins.push(Plugin::new::<40>());
                plugins.push(Plugin::new::<41>());
                plugins.push(Plugin::new::<42>());
                plugins.push(Plugin::new::<43>());
                plugins.push(Plugin::new::<44>());
                plugins.push(Plugin::new::<45>());
                plugins.push(Plugin::new::<46>());
                plugins.push(Plugin::new::<47>());
                plugins.push(Plugin::new::<48>());
                plugins.push(Plugin::new::<49>());
                plugins.push(Plugin::new::<50>());
                plugins.push(Plugin::new::<51>());
                plugins.push(Plugin::new::<52>());
                plugins.push(Plugin::new::<53>());
                plugins.push(Plugin::new::<54>());
                plugins.push(Plugin::new::<55>());
                plugins.push(Plugin::new::<56>());
                plugins.push(Plugin::new::<57>());
                plugins.push(Plugin::new::<58>());
                plugins.push(Plugin::new::<59>());
                plugins.push(Plugin::new::<60>());
                plugins.push(Plugin::new::<61>());
                plugins.push(Plugin::new::<62>());
                plugins.push(Plugin::new::<63>());
                plugins.push(Plugin::new::<64>());
                plugins.push(Plugin::new::<65>());
                plugins.push(Plugin::new::<66>());
                plugins.push(Plugin::new::<67>());
                plugins.push(Plugin::new::<68>());
                plugins.push(Plugin::new::<69>());
                plugins.push(Plugin::new::<70>());
                plugins.push(Plugin::new::<71>());
                plugins.push(Plugin::new::<72>());
                plugins.push(Plugin::new::<73>());
                plugins.push(Plugin::new::<74>());
                plugins.push(Plugin::new::<75>());
                plugins.push(Plugin::new::<76>());
                plugins.push(Plugin::new::<77>());
                plugins.push(Plugin::new::<78>());
                plugins.push(Plugin::new::<79>());
                plugins.push(Plugin::new::<80>());
                plugins.push(Plugin::new::<81>());
                plugins.push(Plugin::new::<82>());
                plugins.push(Plugin::new::<83>());
                plugins.push(Plugin::new::<84>());
                plugins.push(Plugin::new::<85>());
                plugins.push(Plugin::new::<86>());
                plugins.push(Plugin::new::<87>());
                plugins.push(Plugin::new::<88>());
                plugins.push(Plugin::new::<89>());
                plugins.push(Plugin::new::<90>());
                plugins.push(Plugin::new::<91>());
                plugins.push(Plugin::new::<92>());
                plugins.push(Plugin::new::<93>());
                plugins.push(Plugin::new::<94>());
                plugins.push(Plugin::new::<95>());
                plugins.push(Plugin::new::<96>());
                plugins.push(Plugin::new::<97>());
                plugins.push(Plugin::new::<98>());
                plugins.push(Plugin::new::<99>());
                Experiment::new(plugins)
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
