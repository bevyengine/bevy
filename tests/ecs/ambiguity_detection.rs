//! A test to confirm that `bevy` doesn't regress its system ambiguities count when using [`DefaultPlugins`].
//! This is run in CI.
//!
//! Note that because this test requires rendering, it isn't actually an integration test!
//! Instead, it's secretly an example: you can run this test manually using `cargo run --example ambiguity_detection`.

use bevy::{
    ecs::schedule::{InternedScheduleLabel, LogLevel, ScheduleBuildSettings},
    platform::collections::HashMap,
    prelude::*,
    render::{pipelined_rendering::PipelinedRenderingPlugin, RenderPlugin},
};

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(RenderPlugin {
                // llvmpipe driver can cause segfaults when aborting the binary while pipelines are being
                // compiled (which happens very quickly in this example since we only run for a single
                // frame). Synchronous pipeline compilation helps prevent these segfaults as the
                // rendering thread blocks on these pipeline compilations.
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            // We also have to disable pipelined rendering to ensure the test doesn't end while the
            // rendering frame is still executing in another thread.
            .disable::<PipelinedRenderingPlugin>(),
    );

    let main_app = app.main_mut();
    configure_ambiguity_detection(main_app);

    let sub_app = app.sub_app_mut(bevy_render::RenderApp);
    configure_ambiguity_detection(sub_app);

    // Make sure all the system stuff is added.
    app.finish();
    app.cleanup();

    let main_app_ambiguities = count_ambiguities(app.main_mut());
    assert_eq!(
        main_app_ambiguities.total(),
        0,
        "Main app has unexpected ambiguities among the following schedules: \n{main_app_ambiguities:#?}.",
    );

    let render_app = app.sub_app_mut(bevy_render::RenderApp);
    // Initialize the MainWorld so the render world systems don't fail initialization.
    render_app.init_resource::<bevy_render::MainWorld>();
    let render_app_ambiguities = count_ambiguities(render_app);
    assert_eq!(
        render_app_ambiguities.total(),
        0,
        "Render app has unexpected ambiguities among the following schedules: \n{render_app_ambiguities:#?}.",
    );
}

/// Contains the number of conflicting systems per schedule.
#[derive(Debug, Deref, DerefMut)]
struct AmbiguitiesCount(pub HashMap<InternedScheduleLabel, usize>);

impl AmbiguitiesCount {
    fn total(&self) -> usize {
        self.values().sum()
    }
}

fn configure_ambiguity_detection(sub_app: &mut SubApp) {
    let mut schedules = sub_app.world_mut().resource_mut::<Schedules>();
    for (_, schedule) in schedules.iter_mut() {
        schedule.set_build_settings(ScheduleBuildSettings {
            // NOTE: you can change this to `LogLevel::Ignore` to easily see the current number of ambiguities.
            ambiguity_detection: LogLevel::Warn,
            // With auto-inserted apply_deferred stages, these can cause two ambiguous systems to
            // become accidentally ordered by one of the apply_deferred stages. Disabling requires
            // us to meet a higher bar. We don't just want no ambiguities - we also don't want
            // changes to systems or the auto-insert code from "creating" new ambiguities (by
            // reordering the graph). However, the cost is that the graph is no longer runnable,
            // since Bevy crates often rely on auto-insert apply_deferred to not panic (e.g.,
            // because a resource wasn't inserted).
            auto_insert_apply_deferred: false,
            use_shortnames: false,
            ..default()
        });
    }
}

/// Returns the number of conflicting systems per schedule.
fn count_ambiguities(sub_app: &mut SubApp) -> AmbiguitiesCount {
    let schedule_labels = sub_app
        .world()
        .resource::<Schedules>()
        .iter()
        .map(|(_, schedule)| schedule.label())
        .collect::<Vec<_>>();
    let mut ambiguities = <HashMap<_, _>>::default();
    for label in schedule_labels {
        let ambiguities_in_schedule =
            sub_app
                .world_mut()
                .schedule_scope(label, |world, schedule| {
                    schedule.initialize(world).unwrap().unwrap();
                    schedule.graph().conflicting_systems().len()
                });
        ambiguities.insert(label, ambiguities_in_schedule);
    }
    AmbiguitiesCount(ambiguities)
}
