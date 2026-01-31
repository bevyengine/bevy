use core::ops::{Deref, DerefMut};

use crate::{
    sync_world::{despawn_temporary_render_entities, entity_sync_system, SyncWorldPlugin},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_app::{App, Plugin, SubApp};
use bevy_ecs::{
    resource::Resource,
    schedule::{IntoScheduleConfigs, Schedule, ScheduleBuildSettings, ScheduleLabel, Schedules},
    world::{Mut, World},
};
#[cfg(feature = "trace")]
use bevy_log::tracing;
use bevy_utils::default;

/// Plugin that sets up the render subapp and handles extracting data from the
/// main world to the render world.
#[derive(Default)]
pub struct ExtractPlugin;

impl Plugin for ExtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncWorldPlugin);
        app.init_resource::<ScratchMainWorld>();

        let mut render_app = SubApp::new();
        render_app.update_schedule = Some(Render.intern());

        let mut extract_schedule = Schedule::new(ExtractSchedule);
        // We skip applying any commands during the ExtractSchedule
        // so commands can be applied on the render thread.
        extract_schedule.set_build_settings(ScheduleBuildSettings {
            auto_insert_apply_deferred: false,
            ..default()
        });
        extract_schedule.set_apply_final_deferred(false);

        render_app.add_schedule(extract_schedule);
        render_app.add_schedule(Render::base_schedule());
        render_app.add_schedule(Schedule::new(RenderStartup));
        render_app.add_systems(
            Render,
            (
                // This set applies the commands from the extract schedule while the render schedule
                // is running in parallel with the main app.
                apply_extract_commands.in_set(RenderSystems::ExtractCommands),
                despawn_temporary_render_entities.in_set(RenderSystems::PostCleanup),
            ),
        );

        render_app.set_extract({
            let mut should_run_startup = true;
            move |main_world, render_world: &mut World| {
                if should_run_startup {
                    // Run the `RenderStartup` if it hasn't run yet. This does mean `RenderStartup` blocks
                    // the rest of the app extraction, but this is necessary since extraction itself can
                    // depend on resources initialized in `RenderStartup`.
                    render_world.run_schedule(RenderStartup);
                    should_run_startup = false;
                }

                {
                    #[cfg(feature = "trace")]
                    let _stage_span = tracing::info_span!("entity_sync").entered();
                    entity_sync_system(main_world, render_world);
                }

                // run extract schedule
                extract(main_world, render_world);
            }
        });

        let (sender, receiver) = bevy_time::create_time_channels();
        render_app.insert_resource(sender);
        app.insert_resource(receiver);
        app.insert_sub_app(RenderApp, render_app);
    }
}

/// Schedule in which data from the main world is 'extracted' into the render world.
///
/// This step should be kept as short as possible to increase the "pipelining potential" for
/// running the next frame while rendering the current frame.
///
/// This schedule is run on the render world, but it also has access to the main world.
/// See [`MainWorld`] and [`Extract`] for details on how to access main world data from this schedule.
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
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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
    use bevy_app::{App, Startup};
    use bevy_ecs::prelude::*;

    use crate::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_plugin::ExtractPlugin,
        sync_world::MainEntity,
        RenderApp,
    };

    #[derive(Component, Clone, Debug)]
    struct RenderComponent;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentExtra;

    #[derive(Component, Clone, Debug, ExtractComponent)]
    struct RenderComponentSeparate;

    #[derive(Component, Clone, Debug)]
    struct RenderComponentNoExtract;

    impl ExtractComponent for RenderComponent {
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
    fn test_extract() {
        let mut app = App::new();

        app.add_plugins(ExtractPlugin);
        app.add_plugins(ExtractComponentPlugin::<RenderComponent>::default());
        app.add_plugins(ExtractComponentPlugin::<RenderComponentSeparate>::default());
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn((RenderComponent, RenderComponentSeparate));
        });

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
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
                        // TODO: this is a bug
                        // assert!(entity.4.is_some());
                    },
                )
                .unwrap();
        }
    }
}
