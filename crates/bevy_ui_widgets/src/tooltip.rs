//! Headless tooltip metadata and hover/click activation.
//!
//! [`Tooltip`] describes content and behavior associated with an entity.
//! [`TooltipPlugin`] emits target-specific [`ShowTooltip`] / [`HideTooltip`]
//! events. Rendering is provided by a styling layer such as `bevy_feathers`.

use core::time::Duration;

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{entity::EntityHashSet, hierarchy::ChildOf, prelude::*, VariantDefaults};
use bevy_picking::{
    events::{Pointer, Press},
    hover::HoverMap,
    PickingSystems,
};
use bevy_reflect::prelude::*;
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{RelativeCursorPosition, UiSystems};

use crate::popover::{PopoverAlign, PopoverPlacement, PopoverSide};

/// Content displayed by a [`Tooltip`].
#[derive(Clone, Default, Debug, PartialEq, Reflect, VariantDefaults)]
#[reflect(Default, PartialEq)]
pub enum TooltipContent {
    /// Derive the text from the entity's accessibility label, falling back to
    /// its [`Name`].
    #[default]
    Auto,
    /// Display a static string.
    Text(String),
    /// Initially display `summary`, then extend it with `details` after the
    /// pointer remains over the target or tooltip popup for longer.
    Detailed {
        /// Text shown at the summary detail level.
        summary: String,
        /// Additional text shown at the detailed detail level.
        details: String,
    },
}

/// Amount of tooltip content requested by [`ShowTooltip`].
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Reflect, VariantDefaults)]
#[reflect(Default, PartialEq)]
pub enum TooltipDetail {
    /// Show the short tooltip content.
    #[default]
    Summary,
    /// Show all available tooltip content.
    Details,
}

/// Interaction that shows a [`Tooltip`].
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Reflect, VariantDefaults)]
#[reflect(Default, PartialEq)]
pub enum TooltipTrigger {
    /// Show while the pointer is over the target, subject to the configured
    /// show and hide delays.
    #[default]
    Hover,
    /// Toggle immediately when the target is clicked. Clicking elsewhere
    /// dismisses automatically opened click tooltips.
    Click,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAction {
    Show,
    Hide,
}

#[derive(Component, Default)]
struct TooltipRuntime {
    visible: bool,
    hovered: bool,
    initialized: bool,
    last_open: Option<bool>,
    last_trigger: TooltipTrigger,
    pending: Option<(PendingAction, Timer)>,
    detail: TooltipDetail,
    detail_timer: Option<Timer>,
}

/// Associates a rendered tooltip popup with its source [`Tooltip`] entity.
///
/// Styling layers should attach this to the popup root. The required
/// [`RelativeCursorPosition`] lets the headless controller keep hover tooltips
/// open while the pointer is over a non-pickable popup.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, PartialEq)]
#[require(RelativeCursorPosition)]
pub struct TooltipPopup {
    /// Entity containing the source [`Tooltip`].
    pub target: Entity,
}

/// Headless tooltip metadata associated with an entity.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
#[require(TooltipRuntime)]
pub struct Tooltip {
    /// Content displayed by the tooltip.
    pub content: TooltipContent,
    /// Preferred placement. The styling layer may try fallback placements when
    /// this placement would be clipped.
    pub placement: PopoverPlacement,
    /// Interaction that shows the tooltip when [`Self::open`] is `None`.
    pub trigger: TooltipTrigger,
    /// Controlled visibility. `None` enables automatic trigger behavior,
    /// `Some(true)` keeps the tooltip open, and `Some(false)` keeps it closed.
    pub open: Option<bool>,
    /// Per-tooltip hover show delay. `None` uses [`TooltipHoverDelay`].
    pub show_delay: Option<Duration>,
    /// Per-tooltip hover hide delay. `None` uses [`TooltipHideDelay`].
    pub hide_delay: Option<Duration>,
    /// Whether the styling layer should render an arrow.
    pub arrow: bool,
}

impl Default for Tooltip {
    fn default() -> Self {
        Self {
            content: TooltipContent::Auto,
            placement: PopoverPlacement {
                side: PopoverSide::Top,
                align: PopoverAlign::Center,
                gap: 8.0,
            },
            trigger: TooltipTrigger::Hover,
            open: None,
            show_delay: None,
            hide_delay: None,
            arrow: true,
        }
    }
}

/// Default delay before an automatically hovered tooltip is shown.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TooltipHoverDelay(pub Duration);

impl Default for TooltipHoverDelay {
    fn default() -> Self {
        Self(Duration::from_millis(500))
    }
}

/// Default delay before an automatically hovered tooltip is hidden.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TooltipHideDelay(pub Duration);

impl Default for TooltipHideDelay {
    fn default() -> Self {
        Self(Duration::from_millis(100))
    }
}

/// Hover duration before a [`TooltipContent::Detailed`] tooltip expands.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TooltipDetailDelay(pub Duration);

impl Default for TooltipDetailDelay {
    fn default() -> Self {
        Self(Duration::from_millis(1500))
    }
}

/// Request that the tooltip for `target` be shown.
#[derive(Event, Clone, Copy, Debug, Reflect)]
#[reflect(Event)]
pub struct ShowTooltip {
    /// Entity whose [`Tooltip`] should be displayed.
    pub target: Entity,
    /// Amount of content to display.
    pub detail: TooltipDetail,
}

impl ShowTooltip {
    /// Creates a request for the summary content.
    pub const fn summary(target: Entity) -> Self {
        Self {
            target,
            detail: TooltipDetail::Summary,
        }
    }

    /// Creates a request for all available content.
    pub const fn details(target: Entity) -> Self {
        Self {
            target,
            detail: TooltipDetail::Details,
        }
    }
}

/// Request that the tooltip for `target` be hidden.
#[derive(Event, Clone, Copy, Debug, Reflect)]
#[reflect(Event)]
pub struct HideTooltip {
    /// Entity whose [`Tooltip`] should be hidden.
    pub target: Entity,
}

fn tooltip_ancestor(
    entity: Entity,
    tooltips: &Query<&Tooltip>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    core::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find(|entity| tooltips.contains(*entity))
}

fn hovered_tooltips(
    hover_map: &HoverMap,
    tooltips: &Query<&Tooltip>,
    parents: &Query<&ChildOf>,
    popups: &Query<(&TooltipPopup, &RelativeCursorPosition)>,
) -> EntityHashSet {
    let mut hovered: EntityHashSet = hover_map
        .values()
        .flat_map(|hovered| hovered.keys())
        .filter_map(|entity| tooltip_ancestor(*entity, tooltips, parents))
        .collect();
    hovered.extend(
        popups
            .iter()
            .filter(|(_, cursor)| cursor.cursor_over())
            .map(|(popup, _)| popup.target)
            .filter(|target| tooltips.contains(*target)),
    );
    hovered
}

fn trigger_action(target: Entity, action: PendingAction, commands: &mut Commands) {
    match action {
        PendingAction::Show => commands.trigger(ShowTooltip::summary(target)),
        PendingAction::Hide => commands.trigger(HideTooltip { target }),
    };
}

fn schedule_action(
    target: Entity,
    runtime: &mut TooltipRuntime,
    action: PendingAction,
    delay: Duration,
    commands: &mut Commands,
) {
    runtime.pending = None;
    if delay.is_zero() {
        trigger_action(target, action, commands);
    } else {
        runtime.pending = Some((action, Timer::new(delay, TimerMode::Once)));
    }
}

fn update_tooltip_hover(
    time: Res<Time>,
    default_show_delay: Res<TooltipHoverDelay>,
    default_hide_delay: Res<TooltipHideDelay>,
    detail_delay: Res<TooltipDetailDelay>,
    hover_map: Res<HoverMap>,
    tooltips: Query<&Tooltip>,
    parents: Query<&ChildOf>,
    popups: Query<(&TooltipPopup, &RelativeCursorPosition)>,
    mut states: Query<(Entity, Ref<Tooltip>, &mut TooltipRuntime)>,
    mut commands: Commands,
) {
    let hovered = hovered_tooltips(&hover_map, &tooltips, &parents, &popups);

    for (target, tooltip, mut runtime) in &mut states {
        let is_hovered = hovered.contains(&target);
        let config_changed = tooltip.is_changed();
        let was_initialized = runtime.initialized;
        let behavior_changed = !was_initialized
            || runtime.last_open != tooltip.open
            || runtime.last_trigger != tooltip.trigger;

        if behavior_changed {
            runtime.pending = None;
        }

        let hover_changed = is_hovered != runtime.hovered;
        let is_detailed = matches!(&tooltip.content, TooltipContent::Detailed { .. });
        if !is_detailed {
            runtime.detail = TooltipDetail::Summary;
            runtime.detail_timer = None;
        } else if is_hovered
            && runtime.detail == TooltipDetail::Summary
            && runtime.detail_timer.is_none()
        {
            runtime.detail_timer = Some(Timer::new(detail_delay.0, TimerMode::Once));
        } else if hover_changed && !is_hovered && !runtime.visible {
            runtime.detail_timer = None;
        }

        match tooltip.open {
            Some(true) => {
                if !runtime.visible {
                    trigger_action(target, PendingAction::Show, &mut commands);
                }
            }
            Some(false) => {
                if runtime.visible {
                    trigger_action(target, PendingAction::Hide, &mut commands);
                }
            }
            None if tooltip.trigger == TooltipTrigger::Hover => {
                if hover_changed {
                    runtime.pending = None;
                }
                if (behavior_changed || hover_changed) && is_hovered && !runtime.visible {
                    schedule_action(
                        target,
                        &mut runtime,
                        PendingAction::Show,
                        tooltip.show_delay.unwrap_or(default_show_delay.0),
                        &mut commands,
                    );
                } else if (behavior_changed || hover_changed) && !is_hovered && runtime.visible {
                    schedule_action(
                        target,
                        &mut runtime,
                        PendingAction::Hide,
                        tooltip.hide_delay.unwrap_or(default_hide_delay.0),
                        &mut commands,
                    );
                }
            }
            None => {}
        }

        runtime.initialized = true;
        runtime.hovered = is_hovered;
        runtime.last_open = tooltip.open;
        runtime.last_trigger = tooltip.trigger;

        let should_remain_visible = match tooltip.open {
            Some(open) => open,
            None if tooltip.trigger == TooltipTrigger::Hover => is_hovered,
            None => true,
        };
        if was_initialized && config_changed && runtime.visible && should_remain_visible {
            commands.trigger(ShowTooltip {
                target,
                detail: runtime.detail,
            });
        }

        if let Some((action, timer)) = runtime.pending.as_mut() {
            timer.tick(time.delta());
            if timer.just_finished() {
                let action = *action;
                runtime.pending = None;
                trigger_action(target, action, &mut commands);
            }
        }

        let is_visible = runtime.visible;
        if is_hovered
            && runtime.detail == TooltipDetail::Summary
            && let Some(timer) = runtime.detail_timer.as_mut()
        {
            timer.tick(time.delta());
            if is_visible && timer.is_finished() {
                runtime.detail_timer = None;
                commands.trigger(ShowTooltip::details(target));
            }
        }
    }
}

fn on_pointer_press(
    trigger: On<Pointer<Press>>,
    tooltips: Query<&Tooltip>,
    parents: Query<&ChildOf>,
    states: Query<(Entity, &Tooltip, &TooltipRuntime)>,
    mut commands: Commands,
) {
    if trigger.event_target() != trigger.original_event_target() {
        return;
    }

    let clicked = tooltip_ancestor(trigger.original_event_target(), &tooltips, &parents);
    if let Some(target) = clicked
        && let Ok((_, tooltip, runtime)) = states.get(target)
        && tooltip.open.is_none()
        && tooltip.trigger == TooltipTrigger::Click
    {
        trigger_action(
            target,
            if runtime.visible {
                PendingAction::Hide
            } else {
                PendingAction::Show
            },
            &mut commands,
        );
        return;
    }

    for (target, tooltip, runtime) in &states {
        if runtime.visible && tooltip.open.is_none() && tooltip.trigger == TooltipTrigger::Click {
            trigger_action(target, PendingAction::Hide, &mut commands);
        }
    }
}

fn on_show_tooltip(
    trigger: On<ShowTooltip>,
    detail_delay: Res<TooltipDetailDelay>,
    tooltips: Query<&Tooltip>,
    mut states: Query<&mut TooltipRuntime>,
) {
    if let Ok(mut runtime) = states.get_mut(trigger.target) {
        runtime.visible = true;
        runtime.pending = None;
        runtime.detail = trigger.detail;
        if trigger.detail == TooltipDetail::Details {
            runtime.detail_timer = None;
        } else if runtime.hovered
            && runtime.detail_timer.is_none()
            && tooltips
                .get(trigger.target)
                .is_ok_and(|tooltip| matches!(&tooltip.content, TooltipContent::Detailed { .. }))
        {
            runtime.detail_timer = Some(Timer::new(detail_delay.0, TimerMode::Once));
        }
    }
}

fn on_hide_tooltip(trigger: On<HideTooltip>, mut states: Query<&mut TooltipRuntime>) {
    if let Ok(mut runtime) = states.get_mut(trigger.target) {
        runtime.visible = false;
        runtime.pending = None;
        runtime.detail = TooltipDetail::Summary;
        runtime.detail_timer = None;
    }
}

/// Plugin providing tooltip hover/click activation and show/hide events.
pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipHoverDelay>()
            .init_resource::<TooltipHideDelay>()
            .init_resource::<TooltipDetailDelay>()
            .add_observer(on_pointer_press)
            .add_observer(on_show_tooltip)
            .add_observer(on_hide_tooltip)
            .add_systems(
                PreUpdate,
                update_tooltip_hover
                    .after(PickingSystems::Hover)
                    .after(UiSystems::Focus),
            )
            .world_mut()
            .register_component_hooks::<Tooltip>()
            .on_remove(|mut world, context| {
                if world
                    .get::<TooltipRuntime>(context.entity)
                    .is_some_and(|runtime| runtime.visible)
                {
                    world.trigger(HideTooltip {
                        target: context.entity,
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_camera::NormalizedRenderTarget;
    use bevy_math::Vec2;
    use bevy_picking::{
        backend::HitData,
        pointer::{Location, PointerButton, PointerId},
    };

    #[derive(Resource, Default)]
    struct EventLog {
        shown: Vec<Entity>,
        details: Vec<TooltipDetail>,
        hidden: Vec<Entity>,
    }

    fn capture_show(trigger: On<ShowTooltip>, mut log: ResMut<EventLog>) {
        log.shown.push(trigger.target);
        log.details.push(trigger.detail);
    }

    fn capture_hide(trigger: On<HideTooltip>, mut log: ResMut<EventLog>) {
        log.hidden.push(trigger.target);
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<HoverMap>();
        app.add_plugins(TooltipPlugin);
        app.init_resource::<EventLog>();
        app.add_observer(capture_show);
        app.add_observer(capture_hide);
        app
    }

    fn hover(app: &mut App, entity: Entity) {
        let hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
        app.world_mut()
            .resource_mut::<HoverMap>()
            .entry(PointerId::Mouse)
            .or_default()
            .insert(entity, hit);
    }

    fn press(app: &mut App, entity: Entity) {
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 0,
                    height: 0,
                },
                position: Vec2::ZERO,
            },
            Press {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                count: 1,
            },
            entity,
        ));
        app.world_mut().flush();
    }

    #[test]
    fn tooltip_defaults() {
        let tooltip = Tooltip::default();
        assert_eq!(tooltip.content, TooltipContent::Auto);
        assert_eq!(tooltip.trigger, TooltipTrigger::Hover);
        assert_eq!(tooltip.open, None);
        assert_eq!(tooltip.show_delay, None);
        assert_eq!(tooltip.hide_delay, None);
        assert!(tooltip.arrow);
    }

    #[test]
    fn hover_uses_per_tooltip_show_and_hide_delays() {
        let mut app = test_app();
        let target = app
            .world_mut()
            .spawn(Tooltip {
                show_delay: Some(Duration::from_millis(10)),
                hide_delay: Some(Duration::from_millis(20)),
                ..Tooltip::default()
            })
            .id();
        hover(&mut app, target);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(9));
        app.update();
        assert!(app.world().resource::<EventLog>().shown.is_empty());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(1));
        app.update();
        assert_eq!(app.world().resource::<EventLog>().shown, vec![target]);

        app.world_mut().resource_mut::<HoverMap>().clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(19));
        app.update();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(1));
        app.update();
        assert_eq!(app.world().resource::<EventLog>().hidden, vec![target]);
    }

    #[test]
    fn hover_changes_cancel_the_opposite_pending_action() {
        let mut app = test_app();
        let target = app
            .world_mut()
            .spawn(Tooltip {
                show_delay: Some(Duration::from_millis(10)),
                hide_delay: Some(Duration::from_millis(10)),
                ..Tooltip::default()
            })
            .id();

        hover(&mut app, target);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(5));
        app.update();
        app.world_mut().resource_mut::<HoverMap>().clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(20));
        app.update();
        assert!(app.world().resource::<EventLog>().shown.is_empty());

        app.world_mut()
            .get_mut::<Tooltip>(target)
            .unwrap()
            .show_delay = Some(Duration::ZERO);
        hover(&mut app, target);
        app.update();
        assert_eq!(app.world().resource::<EventLog>().shown, vec![target]);

        app.world_mut().resource_mut::<HoverMap>().clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(5));
        app.update();
        hover(&mut app, target);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(20));
        app.update();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());
    }

    #[test]
    fn hover_uses_nearest_tooltip_ancestor() {
        let mut app = test_app();
        app.world_mut().resource_mut::<TooltipHoverDelay>().0 = Duration::ZERO;
        let target = app.world_mut().spawn(Tooltip::default()).id();
        let child = app.world_mut().spawn(ChildOf(target)).id();
        hover(&mut app, child);

        app.update();
        assert_eq!(app.world().resource::<EventLog>().shown, vec![target]);
    }

    #[test]
    fn popup_hover_keeps_the_source_tooltip_open() {
        let mut app = test_app();
        app.world_mut().resource_mut::<TooltipHoverDelay>().0 = Duration::ZERO;
        let target = app.world_mut().spawn(Tooltip::default()).id();
        hover(&mut app, target);
        app.update();

        let popup = app.world_mut().spawn(TooltipPopup { target }).id();
        app.world_mut().resource_mut::<HoverMap>().clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        app.update();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());

        app.world_mut()
            .get_mut::<RelativeCursorPosition>(popup)
            .unwrap()
            .cursor_over = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(100));
        app.update();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());

        app.world_mut()
            .get_mut::<RelativeCursorPosition>(popup)
            .unwrap()
            .cursor_over = false;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(99));
        app.update();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(1));
        app.update();
        assert_eq!(app.world().resource::<EventLog>().hidden, vec![target]);
    }

    #[test]
    fn detailed_tooltip_expands_after_the_second_hover_delay() {
        let mut app = test_app();
        app.world_mut().resource_mut::<TooltipHoverDelay>().0 = Duration::ZERO;
        app.world_mut().resource_mut::<TooltipDetailDelay>().0 = Duration::from_millis(20);
        let target = app
            .world_mut()
            .spawn(Tooltip {
                content: TooltipContent::Detailed {
                    summary: "Short".into(),
                    details: "More detail".into(),
                },
                ..Tooltip::default()
            })
            .id();
        hover(&mut app, target);
        app.update();
        assert_eq!(
            app.world().resource::<EventLog>().details,
            vec![TooltipDetail::Summary]
        );

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(19));
        app.update();
        assert_eq!(app.world().resource::<EventLog>().details.len(), 1);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(1));
        app.update();
        assert_eq!(
            app.world().resource::<EventLog>().details,
            vec![TooltipDetail::Summary, TooltipDetail::Details]
        );
    }

    #[test]
    fn click_tooltip_toggles_immediately_from_a_child() {
        let mut app = test_app();
        let target = app
            .world_mut()
            .spawn(Tooltip {
                trigger: TooltipTrigger::Click,
                ..Tooltip::default()
            })
            .id();
        let child = app.world_mut().spawn(ChildOf(target)).id();

        press(&mut app, child);
        assert_eq!(app.world().resource::<EventLog>().shown, vec![target]);
        press(&mut app, child);
        assert_eq!(app.world().resource::<EventLog>().hidden, vec![target]);
    }

    #[test]
    fn multiple_click_tooltips_can_remain_visible() {
        let mut app = test_app();
        let tooltip = || Tooltip {
            trigger: TooltipTrigger::Click,
            ..Tooltip::default()
        };
        let first = app.world_mut().spawn(tooltip()).id();
        let second = app.world_mut().spawn(tooltip()).id();

        press(&mut app, first);
        press(&mut app, second);

        assert_eq!(
            app.world().resource::<EventLog>().shown,
            vec![first, second]
        );
        assert!(app.world().resource::<EventLog>().hidden.is_empty());
    }

    #[test]
    fn clicking_outside_dismisses_automatic_click_tooltips() {
        let mut app = test_app();
        let target = app
            .world_mut()
            .spawn(Tooltip {
                trigger: TooltipTrigger::Click,
                ..Tooltip::default()
            })
            .id();
        let outside = app.world_mut().spawn_empty().id();

        press(&mut app, target);
        press(&mut app, outside);

        assert_eq!(app.world().resource::<EventLog>().hidden, vec![target]);
    }

    #[test]
    fn multiple_controlled_tooltips_can_stay_open() {
        let mut app = test_app();
        let tooltip = || Tooltip {
            open: Some(true),
            ..Tooltip::default()
        };
        let first = app.world_mut().spawn(tooltip()).id();
        let second = app.world_mut().spawn(tooltip()).id();

        app.update();

        assert_eq!(
            app.world().resource::<EventLog>().shown,
            vec![first, second]
        );
        app.world_mut().resource_mut::<HoverMap>().clear();
        app.update();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());
    }

    #[test]
    fn controlled_closed_tooltip_ignores_automatic_trigger() {
        let mut app = test_app();
        app.world_mut().resource_mut::<TooltipHoverDelay>().0 = Duration::ZERO;
        let target = app
            .world_mut()
            .spawn(Tooltip {
                open: Some(false),
                ..Tooltip::default()
            })
            .id();
        hover(&mut app, target);
        app.update();

        assert!(app.world().resource::<EventLog>().shown.is_empty());
    }

    #[test]
    fn controlled_tooltip_refreshes_and_can_be_closed() {
        let mut app = test_app();
        let target = app
            .world_mut()
            .spawn(Tooltip {
                open: Some(true),
                ..Tooltip::default()
            })
            .id();
        app.update();

        app.world_mut().get_mut::<Tooltip>(target).unwrap().content =
            TooltipContent::Text("Updated".into());
        app.update();
        assert_eq!(
            app.world().resource::<EventLog>().shown,
            vec![target, target]
        );

        app.world_mut().get_mut::<Tooltip>(target).unwrap().open = Some(false);
        app.update();
        assert_eq!(app.world().resource::<EventLog>().hidden, vec![target]);
    }

    #[test]
    fn removing_visible_tooltip_hides_only_that_target() {
        let mut app = test_app();
        let first = app.world_mut().spawn(Tooltip::default()).id();
        let second = app.world_mut().spawn(Tooltip::default()).id();
        let never_shown = app.world_mut().spawn(Tooltip::default()).id();
        app.world_mut().trigger(ShowTooltip::summary(first));
        app.world_mut().trigger(ShowTooltip::summary(second));

        app.world_mut().entity_mut(never_shown).remove::<Tooltip>();
        assert!(app.world().resource::<EventLog>().hidden.is_empty());
        app.world_mut().entity_mut(first).remove::<Tooltip>();

        assert_eq!(app.world().resource::<EventLog>().hidden, vec![first]);
    }
}
