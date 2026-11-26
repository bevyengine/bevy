//! A test to confirm that `bevy` doesn't regress its system ambiguities count when using [`DefaultPlugins`].
//! This is run in CI.
//!
//! Note that because this test requires rendering, it isn't actually an integration test!
//! Instead, it's secretly an example: you can run this test manually using `cargo run --example ambiguity_detection`.

use bevy::{
    ecs::schedule::{InternedScheduleLabel, LogLevel, ScheduleBuildSettings},
    platform::collections::HashMap,
    prelude::*,
    render::pipelined_rendering::PipelinedRenderingPlugin,
};

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
enum GameState {
    #[default]
    Running,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            // The PipelinedRenderingPlugin takes the RenderApp out of the app during cleanup to
            // send it to another thread, but we still need access to it to count its ambiguities.
            // So disable the plugin here.
            .disable::<PipelinedRenderingPlugin>(),
    );
    // Make sure that adding a state doesn't add ambiguities.
    app.init_state::<GameState>();

    let schedule_build_settings = ScheduleBuildSettings {
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
    };
    app.configure_schedules(schedule_build_settings.clone());
    app.sub_app_mut(bevy_render::RenderApp)
        .configure_schedules(schedule_build_settings);

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
