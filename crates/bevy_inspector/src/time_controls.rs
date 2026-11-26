//! Controls pausing, stepping and changing the speed of the local app's [`Time<Virtual>`].
//!
//! The controls act on the world the inspector runs in. They are hidden while that world has no
//! [`Time<Virtual>`]. The inspector's own panels pace themselves by [`Time<Real>`](bevy_time::Real),
//! so they keep updating while virtual time is paused or slowed down.

use bevy_app::{App, First, Last, PostUpdate};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut, Ref},
    component::Component,
    entity::Entity,
    hierarchy::Children,
    message::message_update_system,
    observer::On,
    query::Has,
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
    template::template,
};
use bevy_feathers::{
    controls::{ButtonVariant, FeathersButton, FeathersNumberInput, NumberInputStep, SoftLimit},
    display::caption,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, on, Scene};
use bevy_time::{Time, TimeSystems, Virtual};
use bevy_ui::{
    px, AlignItems, Display, FlexDirection, FlexWrap, InteractionDisabled, Node, UiSystems,
};
use bevy_ui_widgets::{Activate, NumericValue, ValueChange};

use crate::entity_tree::InspectorUi;

/// Marker for the root of [`time_controls`].
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct TimeControls;

/// One of the controls of [`time_controls`].
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, Clone, PartialEq)]
pub enum TimeControl {
    /// A button that pauses virtual time, or resumes it while paused. It is selected while paused.
    #[default]
    Pause,
    /// A button that runs one frame of paused virtual time, which advances by that frame's real
    /// time scaled by the relative speed.
    Step,
    /// A number input showing the relative speed of virtual time, and setting it between
    /// [`MIN_SPEED`] and [`MAX_SPEED`].
    Speed,
}

/// The lowest relative speed the [`TimeControl::Speed`] input sets.
pub const MIN_SPEED: f32 = 0.05;

/// The highest relative speed the [`TimeControl::Speed`] input sets.
pub const MAX_SPEED: f32 = 100.0;

/// A one frame step of paused virtual time.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
struct FrameStep {
    requested: bool,
    running: bool,
}

/// Buttons that pause, resume and step virtual time, and an input for its speed.
///
/// The controls act on the [`Time<Virtual>`] of the app the inspector runs in, and need
/// [`InspectorPlugin`](crate::InspectorPlugin).
pub fn time_controls() -> impl Scene {
    bsn! {
        TimeControls
        InspectorUi
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6),
            row_gap: px(3),
        }
        Children [
            @FeathersButton {
                @caption: bsn! { @caption("Pause") },
            }
            TimeControl::Pause
            on(time_control_activated)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Step") },
            }
            TimeControl::Step
            InteractionDisabled
            on(time_control_activated)
            --
            @FeathersNumberInput {
                @interior_label_text: {Some("Speed")},
            }
            TimeControl::Speed
            NumberInputStep(0.25)
            template(|_| Ok(SoftLimit::f32(MIN_SPEED..=4.0)))
            Node { width: px(104) }
            on(speed_changed)
        ]
    }
}

pub(crate) fn add_time_controls(app: &mut App) {
    app.init_resource::<FrameStep>()
        .add_systems(
            First,
            begin_frame_step
                .before(TimeSystems)
                .ambiguous_with(message_update_system),
        )
        .add_systems(Last, end_frame_step)
        .add_systems(PostUpdate, sync_time_controls.before(UiSystems::Prepare));
}

fn time_control_activated(
    activate: On<Activate>,
    controls: Query<&TimeControl>,
    time: Option<ResMut<Time<Virtual>>>,
    step: Option<ResMut<FrameStep>>,
) {
    let (Ok(control), Some(mut time)) = (controls.get(activate.entity), time) else {
        return;
    };
    let paused = time.is_paused() || step.as_ref().is_some_and(|step| step.running);
    match control {
        TimeControl::Pause if paused => {
            time.unpause();
            if let Some(mut step) = step {
                *step = FrameStep::default();
            }
        }
        TimeControl::Pause => time.pause(),
        TimeControl::Step if paused => {
            if let Some(mut step) = step {
                step.requested = true;
            }
        }
        TimeControl::Step | TimeControl::Speed => {}
    }
}

fn speed_changed(
    change: On<ValueChange<f32>>,
    controls: Query<&TimeControl>,
    time: Option<ResMut<Time<Virtual>>>,
) {
    let (Ok(TimeControl::Speed), Some(mut time)) = (controls.get(change.source), time) else {
        return;
    };
    if change.value.is_nan() {
        return;
    }
    let speed = change.value.clamp(MIN_SPEED, MAX_SPEED);
    if time.relative_speed() != speed {
        time.set_relative_speed(speed);
    }
}

/// Unpauses virtual time for the frame of a requested step, before it advances.
fn begin_frame_step(mut step: ResMut<FrameStep>, time: Option<ResMut<Time<Virtual>>>) {
    if !step.requested {
        return;
    }
    step.requested = false;
    if let Some(mut time) = time
        && time.is_paused()
    {
        time.unpause();
        step.running = true;
    }
}

/// Pauses virtual time again at the end of the frame of a step.
fn end_frame_step(mut step: ResMut<FrameStep>, time: Option<ResMut<Time<Virtual>>>) {
    if !step.running {
        return;
    }
    step.running = false;
    if let Some(mut time) = time {
        time.pause();
    }
}

fn sync_time_controls(
    time: Option<Res<Time<Virtual>>>,
    step: Res<FrameStep>,
    mut was_available: Local<bool>,
    roots: Query<(Entity, Ref<TimeControls>)>,
    mut controls: Query<(
        Entity,
        &TimeControl,
        Option<&mut ButtonVariant>,
        Option<&NumericValue>,
        Has<InteractionDisabled>,
    )>,
    mut commands: Commands,
) {
    let available = time.is_some();
    let availability_changed = available != *was_available;
    *was_available = available;
    for (root, marker) in &roots {
        if availability_changed || marker.is_added() {
            let display = if available {
                Display::Flex
            } else {
                Display::None
            };
            commands
                .entity(root)
                .entry::<Node>()
                .and_modify(move |mut node| node.display = display);
        }
    }

    let Some(time) = time else {
        return;
    };
    let paused = time.is_paused() || step.running;
    let speed = NumericValue::F32(time.relative_speed());
    for (entity, control, variant, value, disabled) in &mut controls {
        match control {
            TimeControl::Pause => {
                if let Some(mut variant) = variant {
                    variant.set_if_neq(ButtonVariant::selected(paused));
                }
            }
            TimeControl::Step if paused && disabled => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            TimeControl::Step if !paused && !disabled => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            TimeControl::Speed if value != Some(&speed) => {
                commands.entity(entity).insert(speed);
            }
            TimeControl::Step | TimeControl::Speed => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{TaskPoolPlugin, Update};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::{query::With, world::World};
    use bevy_feathers::{
        controls::{Dimensionless, UnitsRegistry},
        theme::UiTheme,
    };
    use bevy_input_focus::InputFocus;
    use bevy_scene::WorldSceneExt;
    use bevy_time::{TimePlugin, TimeUpdateStrategy};
    use core::time::Duration;

    use crate::InspectorPlugin;

    const FRAME: Duration = Duration::from_millis(20);

    fn test_app(with_time: bool) -> App {
        app_with(with_time, true)
    }

    fn app_with(with_time: bool, with_plugin: bool) -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
        ));
        if with_plugin {
            app.add_plugins(InspectorPlugin);
        }
        if with_time {
            app.add_plugins(TimePlugin)
                .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        }
        let mut units = UnitsRegistry::default();
        units.insert(&Dimensionless);
        app.insert_resource(units);
        app.init_resource::<InputFocus>();
        app.init_resource::<UiTheme>();
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app.world_mut().spawn_scene(time_controls()).unwrap();
        app.update();
        app
    }

    fn control(app: &mut App, kind: TimeControl) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &TimeControl)>()
            .iter(world)
            .find(|(_, control)| **control == kind)
            .unwrap()
            .0
    }

    fn click(app: &mut App, kind: TimeControl) {
        let entity = control(app, kind);
        app.world_mut().trigger(Activate { entity });
        app.update();
    }

    fn set_speed(app: &mut App, value: f32) {
        let source = control(app, TimeControl::Speed);
        app.world_mut().trigger(ValueChange {
            source,
            value,
            is_final: true,
        });
        app.update();
    }

    fn virtual_time(app: &App) -> &Time<Virtual> {
        app.world().resource::<Time<Virtual>>()
    }

    fn pause_variant(app: &mut App) -> ButtonVariant {
        let pause = control(app, TimeControl::Pause);
        app.world().get::<ButtonVariant>(pause).unwrap().clone()
    }

    fn step_disabled(app: &mut App) -> bool {
        let step = control(app, TimeControl::Step);
        app.world().get::<InteractionDisabled>(step).is_some()
    }

    fn root_display(world: &mut World) -> Display {
        let mut roots = world.query_filtered::<&Node, With<TimeControls>>();
        roots.single(world).unwrap().display
    }

    #[test]
    fn the_pause_button_pauses_and_resumes_virtual_time_and_shows_it() {
        let mut app = test_app(true);
        assert!(!virtual_time(&app).is_paused());
        assert_eq!(pause_variant(&mut app), ButtonVariant::Normal);
        assert!(step_disabled(&mut app));

        click(&mut app, TimeControl::Pause);
        assert!(virtual_time(&app).is_paused());
        assert_eq!(pause_variant(&mut app), ButtonVariant::Primary);
        assert!(!step_disabled(&mut app));

        click(&mut app, TimeControl::Pause);
        assert!(!virtual_time(&app).is_paused());
        assert_eq!(pause_variant(&mut app), ButtonVariant::Normal);
        assert!(step_disabled(&mut app));
    }

    #[test]
    fn a_pause_made_by_the_app_shows_on_the_button() {
        let mut app = test_app(true);
        app.add_systems(Update, |mut time: ResMut<Time<Virtual>>| time.pause());
        app.update();
        assert_eq!(pause_variant(&mut app), ButtonVariant::Primary);
        assert!(!step_disabled(&mut app));
    }

    #[test]
    fn the_speed_input_sets_the_relative_speed_and_shows_the_apps_speed() {
        let mut app = test_app(true);
        let input = control(&mut app, TimeControl::Speed);
        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(1.0))
        );

        set_speed(&mut app, 0.25);
        assert_eq!(virtual_time(&app).relative_speed(), 0.25);

        set_speed(&mut app, f32::NAN);
        assert_eq!(virtual_time(&app).relative_speed(), 0.25);

        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(3.0);
        app.update();
        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(3.0))
        );
    }

    #[test]
    fn the_speed_input_keeps_the_speed_above_zero_and_shows_an_app_speed_of_zero() {
        let mut app = test_app(true);
        set_speed(&mut app, 0.0);
        assert_eq!(virtual_time(&app).relative_speed(), MIN_SPEED);
        set_speed(&mut app, -1.0);
        assert_eq!(virtual_time(&app).relative_speed(), MIN_SPEED);

        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(0.0);
        app.update();
        app.update();
        let input = control(&mut app, TimeControl::Speed);
        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(0.0))
        );
        assert_eq!(virtual_time(&app).relative_speed(), 0.0);
    }

    #[test]
    fn a_huge_or_infinite_speed_is_clamped_and_time_keeps_running() {
        let mut app = test_app(true);
        set_speed(&mut app, 1e30);
        assert_eq!(virtual_time(&app).relative_speed(), MAX_SPEED);
        set_speed(&mut app, f32::INFINITY);
        assert_eq!(virtual_time(&app).relative_speed(), MAX_SPEED);
        let before = virtual_time(&app).elapsed();
        app.update();
        assert!(virtual_time(&app).elapsed() > before);
    }

    #[test]
    fn a_step_advances_paused_virtual_time_by_one_frame() {
        let mut app = test_app(true);
        click(&mut app, TimeControl::Pause);
        app.update();
        let paused_at = virtual_time(&app).elapsed();
        app.update();
        assert_eq!(virtual_time(&app).elapsed(), paused_at);

        click(&mut app, TimeControl::Step);
        assert_eq!(virtual_time(&app).elapsed(), paused_at + FRAME);
        assert_eq!(virtual_time(&app).delta(), FRAME);
        assert!(virtual_time(&app).is_paused());
        assert_eq!(pause_variant(&mut app), ButtonVariant::Primary);
        assert!(!step_disabled(&mut app));

        app.update();
        assert_eq!(virtual_time(&app).elapsed(), paused_at + FRAME);
        assert_eq!(virtual_time(&app).delta(), Duration::ZERO);
    }

    #[test]
    fn a_step_follows_the_relative_speed() {
        let mut app = test_app(true);
        set_speed(&mut app, 0.5);
        click(&mut app, TimeControl::Pause);
        let paused_at = virtual_time(&app).elapsed();
        click(&mut app, TimeControl::Step);
        assert_eq!(virtual_time(&app).elapsed(), paused_at + FRAME / 2);
        assert!(virtual_time(&app).is_paused());
    }

    #[derive(Resource, Default)]
    struct RunningFrames(Vec<(Duration, bool)>);

    #[test]
    fn a_step_runs_one_whole_frame_of_unpaused_time() {
        let mut app = test_app(true);
        app.init_resource::<RunningFrames>().add_systems(
            Update,
            (|time: Res<Time<Virtual>>, mut frames: ResMut<RunningFrames>| {
                frames.0.push((time.delta(), time.was_paused()));
            })
            .run_if(|time: Res<Time<Virtual>>| !time.is_paused()),
        );
        click(&mut app, TimeControl::Pause);
        app.update();
        app.world_mut().resource_mut::<RunningFrames>().0.clear();

        click(&mut app, TimeControl::Step);
        assert_eq!(app.world().resource::<RunningFrames>().0, [(FRAME, false)]);
        assert!(virtual_time(&app).is_paused());

        app.update();
        app.update();
        assert_eq!(app.world().resource::<RunningFrames>().0.len(), 1);

        click(&mut app, TimeControl::Step);
        assert_eq!(app.world().resource::<RunningFrames>().0.len(), 2);
    }

    #[test]
    fn a_step_advances_by_at_most_the_max_delta() {
        let mut app = test_app(true);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(2)));
        click(&mut app, TimeControl::Pause);
        let paused_at = virtual_time(&app).elapsed();
        let max_delta = virtual_time(&app).max_delta();
        click(&mut app, TimeControl::Step);
        assert_eq!(virtual_time(&app).elapsed(), paused_at + max_delta);
    }

    #[test]
    fn without_the_inspector_plugin_clicks_do_not_panic_and_do_not_step() {
        let mut app = app_with(true, false);
        click(&mut app, TimeControl::Pause);
        assert!(virtual_time(&app).is_paused());
        let paused_at = virtual_time(&app).elapsed();
        click(&mut app, TimeControl::Step);
        set_speed(&mut app, 2.0);
        assert_eq!(virtual_time(&app).elapsed(), paused_at);
        assert!(virtual_time(&app).is_paused());
    }

    #[test]
    fn a_step_while_running_does_nothing() {
        let mut app = test_app(true);
        let before = virtual_time(&app).elapsed();
        click(&mut app, TimeControl::Step);
        assert_eq!(virtual_time(&app).elapsed(), before + FRAME);
        assert!(!virtual_time(&app).is_paused());
        app.update();
        assert!(!virtual_time(&app).is_paused());
    }

    #[test]
    fn the_controls_hide_without_virtual_time() {
        let mut app = test_app(false);
        assert_eq!(root_display(app.world_mut()), Display::None);
        click(&mut app, TimeControl::Pause);
        click(&mut app, TimeControl::Step);
        set_speed(&mut app, 2.0);
        assert!(app.world().get_resource::<Time<Virtual>>().is_none());

        app.insert_resource(Time::<Virtual>::default());
        app.update();
        assert_eq!(root_display(app.world_mut()), Display::Flex);
    }
}
