use core::marker::PhantomData;

use crate::sync_world::{despawn_temporary_entities, entity_sync_system, SyncWorldPlugin};
use bevy_app::{App, AppLabel, Plugin, SubApp};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::{
        InternedScheduleLabel, InternedSystemSet, IntoScheduleConfigs, Schedule,
        ScheduleBuildSettings, ScheduleLabel, Schedules,
    },
    world::{Mut, World},
};
use bevy_utils::default;

/// Plugin that sets up the [`RenderApp`](`crate::RenderApp`) and handles extracting data from the
/// main world to the render world.
pub struct ExtractPlugin<L: AppLabel + Default> {
    /// Function that gets run at the beginning of each extraction.
    ///
    /// Gets the main world and render world as arguments (in that order).
    pub pre_extract: fn(&mut World, &mut World),

    marker: PhantomData<L>,

    pub base_schedule: fn() -> Schedule,
    pub schedule_label: InternedScheduleLabel,

    pub extract_set: InternedSystemSet,
    pub despawn_set: InternedSystemSet,
}

impl<L: AppLabel + Default> ExtractPlugin<L> {
    pub fn new(
        pre_extract: fn(&mut World, &mut World),
        base_schedule: fn() -> Schedule,
        schedule_label: InternedScheduleLabel,
        extract_set: InternedSystemSet,
        despawn_set: InternedSystemSet,
    ) -> Self {
        Self {
            pre_extract,
            marker: PhantomData,
            base_schedule,
            schedule_label,
            extract_set,
            despawn_set,
        }
    }
}

impl<L: AppLabel + Default + Copy + Eq> Plugin for ExtractPlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncWorldPlugin::<L>::default());
        app.init_resource::<ScratchMainWorld>();

        let mut render_app = SubApp::new();

        let mut extract_schedule = Schedule::new(ExtractSchedule);
        // We skip applying any commands during the ExtractSchedule
        // so commands can be applied on the render thread.
        extract_schedule.set_build_settings(ScheduleBuildSettings {
            auto_insert_apply_deferred: false,
            ..default()
        });
        extract_schedule.set_apply_final_deferred(false);

        render_app
            .add_schedule((self.base_schedule)())
            .add_schedule(extract_schedule)
            .allow_ambiguous_resource::<MainWorld>()
            .add_systems(
                self.schedule_label,
                (
                    // This set applies the commands from the extract schedule while the render schedule
                    // is running in parallel with the main app.
                    apply_extract_commands.in_set(self.extract_set),
                    despawn_temporary_entities::<L>.in_set(self.despawn_set),
                ),
            );

        let pre_extract = self.pre_extract;
        render_app.set_extract(move |main_world, render_world| {
            pre_extract(main_world, render_world);

            {
                #[cfg(feature = "trace")]
                let _stage_span = bevy_log::info_span!("entity_sync").entered();
                entity_sync_system::<L>(main_world, render_world);
            }

            // run extract schedule
            extract(main_world, render_world);
        });

        app.insert_sub_app(L::default(), render_app);
    }
}

/// Schedule in which data from the main world is 'extracted' into the render world.
///
/// This step should be kept as short as possible to increase the "pipelining potential" for
/// running the next frame while rendering the current frame.
///
/// This schedule is run on the render world, but it also has access to the main world.
/// See [`MainWorld`] and [`Extract`](crate::Extract) for details on how to access main world data from this schedule.
#[derive(ScheduleLabel, PartialEq, Eq, Debug, Clone, Hash, Default)]
pub struct ExtractSchedule;

/// Applies the commands from the extract schedule. This happens during
/// the render schedule rather than during extraction to allow the commands to run in parallel with the
/// main app when pipelined rendering is enabled.
fn apply_extract_commands(render_world: &mut World) {
    render_world.resource_scope(|render_world, mut schedules: Mut<Schedules>| {
        schedules
            .get_mut(ExtractSchedule)
            .unwrap()
            .apply_deferred(render_world);
    });
}
/// The simulation [`World`] of the application, stored as a resource.
///
/// This resource is only available during [`ExtractSchedule`] and not
/// during command application of that schedule.
/// See [`Extract`](crate::Extract) for more details.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct MainWorld(World);

/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`ExtractSchedule`].
#[derive(Resource, Default)]
struct ScratchMainWorld(World);

/// Executes the [`ExtractSchedule`] step of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
pub fn extract(main_world: &mut World, render_world: &mut World) {
    // temporarily add the app world to the render world as a resource
    let scratch_world = main_world.remove_resource::<ScratchMainWorld>().unwrap();
    let inserted_world = core::mem::replace(main_world, scratch_world.0);
    render_world.insert_resource(MainWorld(inserted_world));
    render_world.run_schedule(ExtractSchedule);

    // move the app world back, as if nothing happened.
    let inserted_world = render_world.remove_resource::<MainWorld>().unwrap();
    let scratch_world = core::mem::replace(main_world, inserted_world.0);
    main_world.insert_resource(ScratchMainWorld(scratch_world));
}

#[cfg(test)]
mod test {
    use bevy_app::{App, AppLabel, Startup};
    use bevy_ecs::{prelude::*, schedule::ScheduleLabel};

    use crate::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_plugin::ExtractPlugin,
        sync_component::SyncComponent,
        sync_world::MainEntity,
        RenderApp,
    };

    #[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
    pub enum MyScheduleSystems {
        ExtractCommands,
        PostCleanup,
    }

    #[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
    pub struct MySchedule;

    impl MySchedule {
        /// Sets up the base structure of the rendering [`Schedule`].
        ///
        /// The sets defined in this enum are configured to run in order.
        pub fn base_schedule() -> Schedule {
            use MyScheduleSystems::*;

            let mut schedule = Schedule::new(Self);

            schedule.configure_sets((ExtractCommands, PostCleanup).chain());

            schedule
        }
    }

    #[derive(Component, Clone, Debug)]
    struct RenderComponent;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentExtra;

    #[derive(Component, Clone, Debug, ExtractComponent)]
    #[extract_app(RenderApp)]
    struct RenderComponentSeparate;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentNoExtract;

    impl SyncComponent<RenderApp> for RenderComponent {
        type Target = (RenderComponent, RenderComponentExtra);
    }

    impl ExtractComponent<RenderApp> for RenderComponent {
        type QueryData = &'static Self;
        type QueryFilter = ();
        type Out = (RenderComponent, RenderComponentExtra);

        fn extract_component(
            _item: bevy_ecs::query::QueryItem<'_, '_, Self::QueryData>,
        ) -> Option<Self::Out> {
            Some((RenderComponent, RenderComponentExtra))
        }
    }

    #[test]
    fn extraction_works() {
        let mut app = App::new();

        app.add_plugins(ExtractPlugin::<RenderApp>::new(
            |_, _| {},
            MySchedule::base_schedule,
            MySchedule.intern(),
            MyScheduleSystems::ExtractCommands.intern(),
            MyScheduleSystems::PostCleanup.intern(),
        ));
        app.add_plugins(ExtractComponentPlugin::<RenderComponent>::default());
        app.add_plugins(ExtractComponentPlugin::<RenderComponentSeparate>::default());
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn((RenderComponent, RenderComponentSeparate));
        });

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        // Normally RenderPlugin sets the RenderRecovery schedule as update, but for
        // testing we just use the Render schedule directly.
        render_app.update_schedule = Some(MySchedule.intern());

        render_app.world_mut().add_observer(
            |event: On<Add, (RenderComponent, RenderComponentExtra)>, mut commands: Commands| {
                // Simulate data that's not extracted
                commands
                    .entity(event.entity)
                    .insert(RenderComponentNoExtract);
            },
        );

        app.update();

        // Check that all components have been extracted
        {
            let render_app = app.get_sub_app_mut(RenderApp).unwrap();
            render_app
                .world_mut()
                .run_system_cached(
                    |entity: Single<(
                        &MainEntity,
                        Option<&RenderComponent>,
                        Option<&RenderComponentExtra>,
                        Option<&RenderComponentSeparate>,
                        Option<&RenderComponentNoExtract>,
                    )>| {
                        assert!(entity.1.is_some());
                        assert!(entity.2.is_some());
                        assert!(entity.3.is_some());
                        assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }

        // Remove RenderComponent
        app.world_mut()
            .run_system_cached(
                |mut commands: Commands, query: Query<Entity, With<RenderComponent>>| {
                    for entity in query {
                        commands.entity(entity).remove::<RenderComponent>();
                    }
                },
            )
            .unwrap();

        app.update();

        // Check that the extracted components have been removed
        {
            let render_app = app.get_sub_app_mut(RenderApp).unwrap();
            render_app
                .world_mut()
                .run_system_cached(
                    |entity: Single<(
                        &MainEntity,
                        Option<&RenderComponent>,
                        Option<&RenderComponentExtra>,
                        Option<&RenderComponentSeparate>,
                        Option<&RenderComponentNoExtract>,
                    )>| {
                        assert!(entity.1.is_none());
                        assert!(entity.2.is_none());
                        assert!(entity.3.is_some());
                        assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }
    }

    #[derive(AppLabel, Debug, Hash, PartialEq, Eq, Clone, Default, Copy)]
    pub struct ExtractAppA;

    #[derive(AppLabel, Debug, Hash, PartialEq, Eq, Clone, Default, Copy)]
    pub struct ExtractAppB;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentSeparateA;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentSeparateB;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentSeparateBoth;

    #[derive(Component, Clone, Debug, ExtractComponent)]
    #[extract_app(ExtractAppA, ExtractAppB)]
    struct RenderComponentDual;

    impl SyncComponent<ExtractAppA> for RenderComponentSeparateA {
        type Target = RenderComponentSeparateA;
    }

    impl ExtractComponent<ExtractAppA> for RenderComponentSeparateA {
        type QueryData = &'static Self;
        type QueryFilter = ();
        type Out = Self;

        fn extract_component(
            _item: bevy_ecs::query::QueryItem<'_, '_, Self::QueryData>,
        ) -> Option<Self::Out> {
            Some(RenderComponentSeparateA)
        }
    }

    impl SyncComponent<ExtractAppB> for RenderComponentSeparateB {
        type Target = RenderComponentSeparateB;
    }

    impl ExtractComponent<ExtractAppB> for RenderComponentSeparateB {
        type QueryData = &'static Self;
        type QueryFilter = ();
        type Out = Self;

        fn extract_component(
            _item: bevy_ecs::query::QueryItem<'_, '_, Self::QueryData>,
        ) -> Option<Self::Out> {
            Some(RenderComponentSeparateB)
        }
    }

    impl SyncComponent<ExtractAppA> for RenderComponentSeparateBoth {
        type Target = RenderComponentSeparateBoth;
    }

    impl ExtractComponent<ExtractAppA> for RenderComponentSeparateBoth {
        type QueryData = &'static Self;
        type QueryFilter = ();
        type Out = Self;

        fn extract_component(
            _item: bevy_ecs::query::QueryItem<'_, '_, Self::QueryData>,
        ) -> Option<Self::Out> {
            Some(RenderComponentSeparateBoth)
        }
    }

    impl SyncComponent<ExtractAppB> for RenderComponentSeparateBoth {
        type Target = RenderComponentSeparateBoth;
    }

    impl ExtractComponent<ExtractAppB> for RenderComponentSeparateBoth {
        type QueryData = &'static Self;
        type QueryFilter = ();
        type Out = Self;

        fn extract_component(
            _item: bevy_ecs::query::QueryItem<'_, '_, Self::QueryData>,
        ) -> Option<Self::Out> {
            Some(RenderComponentSeparateBoth)
        }
    }

    #[test]
    fn dual_extraction_works() {
        let mut app = App::new();

        app.add_plugins(ExtractPlugin::<ExtractAppA>::new(
            |_, _| {},
            MySchedule::base_schedule,
            MySchedule.intern(),
            MyScheduleSystems::ExtractCommands.intern(),
            MyScheduleSystems::PostCleanup.intern(),
        ));
        app.add_plugins(ExtractPlugin::<ExtractAppB>::new(
            |_, _| {},
            MySchedule::base_schedule,
            MySchedule.intern(),
            MyScheduleSystems::ExtractCommands.intern(),
            MyScheduleSystems::PostCleanup.intern(),
        ));

        app.add_plugins(ExtractComponentPlugin::<
            RenderComponentSeparateA,
            ExtractAppA,
        >::default());
        app.add_plugins(ExtractComponentPlugin::<
            RenderComponentSeparateB,
            ExtractAppB,
        >::default());
        app.add_plugins(ExtractComponentPlugin::<
            RenderComponentSeparateBoth,
            ExtractAppA,
        >::default());
        app.add_plugins(ExtractComponentPlugin::<
            RenderComponentSeparateBoth,
            ExtractAppB,
        >::default());
        app.add_plugins(ExtractComponentPlugin::<RenderComponentDual, ExtractAppA>::default());
        app.add_plugins(ExtractComponentPlugin::<RenderComponentDual, ExtractAppB>::default());

        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn((
                RenderComponentSeparateA,
                RenderComponentSeparateB,
                RenderComponentSeparateBoth,
                RenderComponentDual,
            ));
        });

        let sub_app_a = app.get_sub_app_mut(ExtractAppA).unwrap();
        sub_app_a.update_schedule = Some(MySchedule.intern());

        let sub_app_b = app.get_sub_app_mut(ExtractAppB).unwrap();
        sub_app_b.update_schedule = Some(MySchedule.intern());

        app.update();

        // Check that all components have been extracted
        {
            let sub_app_a = app.get_sub_app_mut(ExtractAppA).unwrap();
            sub_app_a
                .world_mut()
                .run_system_cached(
                    |entity: Single<(
                        &MainEntity,
                        Option<&RenderComponentSeparateA>,
                        Option<&RenderComponentSeparateB>,
                        Option<&RenderComponentSeparateBoth>,
                        Option<&RenderComponentDual>,
                    )>| {
                        assert!(entity.1.is_some());
                        assert!(entity.2.is_none());
                        assert!(entity.3.is_some());
                        assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }

        {
            let sub_app_b = app.get_sub_app_mut(ExtractAppB).unwrap();
            sub_app_b
                .world_mut()
                .run_system_cached(
                    |entity: Single<(
                        &MainEntity,
                        Option<&RenderComponentSeparateA>,
                        Option<&RenderComponentSeparateB>,
                        Option<&RenderComponentSeparateBoth>,
                        Option<&RenderComponentDual>,
                    )>| {
                        assert!(entity.1.is_none());
                        assert!(entity.2.is_some());
                        assert!(entity.3.is_some());
                        assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }

        // Remove RenderComponentSeparateA
        app.world_mut()
            .run_system_cached(
                |mut commands: Commands, query: Query<Entity, With<RenderComponentSeparateA>>| {
                    for entity in query {
                        commands.entity(entity).remove::<RenderComponentSeparateA>();
                    }
                },
            )
            .unwrap();

        app.update();

        // Check that the extracted components have been removed
        {
            let sub_app_a = app.get_sub_app_mut(ExtractAppA).unwrap();
            sub_app_a
                .world_mut()
                .run_system_cached(
                    |entity: Single<(
                        &MainEntity,
                        Option<&RenderComponentSeparateA>,
                        Option<&RenderComponentSeparateB>,
                        Option<&RenderComponentSeparateBoth>,
                        Option<&RenderComponentDual>,
                    )>| {
                        assert!(entity.1.is_none());
                        assert!(entity.2.is_none());
                        assert!(entity.3.is_some());
                        assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }

        {
            let sub_app_b = app.get_sub_app_mut(ExtractAppB).unwrap();
            sub_app_b
                .world_mut()
                .run_system_cached(
                    |entity: Single<(
                        &MainEntity,
                        Option<&RenderComponentSeparateA>,
                        Option<&RenderComponentSeparateB>,
                        Option<&RenderComponentSeparateBoth>,
                        Option<&RenderComponentDual>,
                    )>| {
                        assert!(entity.1.is_none());
                        assert!(entity.2.is_some());
                        assert!(entity.3.is_some());
                        assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }
    }
}
