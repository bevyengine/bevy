use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    lifecycle::Add,
    observer::On,
    query::With,
    reflect::{ReflectComponent, ReflectEvent},
    system::{Commands, Query, SystemState},
    world::World,
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input_focus::{
    tab_navigation::{NavAction, TabGroup, TabNavigation},
    FocusCause, FocusedInput, InputFocus, InputFocusVisible,
};
use bevy_log::warn;
use bevy_picking::events::{Pointer, Press};
use bevy_reflect::Reflect;
use bevy_time::DelayedCommandsExt;

/// Component that defines a modal dialog box.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Dialog)))]
#[require(TabGroup { order: 0, modal: true })]
#[reflect(Component)]
pub struct ModalDialog;

/// Component that defines the barrier element that covers the screen behind the dialog box.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct ModalDialogBarrier;

/// Event used to indicate that the modal dialog wants to be closed. This can happen because
/// the user clicked on the barrier, hit the escape key, or clicked the close box in the dialog
/// title. This event propagates so that the owner of the dialog can despawn it.
#[derive(EntityEvent, Clone, Debug)]
#[entity_event(propagate, auto_propagate)]
#[derive(Reflect)]
#[reflect(Event)]
pub struct RequestClose {
    /// The [`ModalDialog`] that triggered this event.
    #[event_target]
    pub source: Entity,
}

fn dialog_barrier_on_click(
    mut ev: On<Pointer<Press>>,
    q_barrier: Query<(), With<ModalDialogBarrier>>,
    q_dialog: Query<(), With<ModalDialog>>,
    mut commands: Commands,
) {
    if q_barrier.contains(ev.event_target()) {
        // Clicking outside the dialog closes it.
        ev.propagate(false);
        commands.trigger(RequestClose {
            source: ev.event_target(),
        });
    } else if q_dialog.contains(ev.event_target()) {
        // Clicking on the dialog body does NOT close it - stop propagation.
        ev.propagate(false);
    }
}

fn dialog_barrier_on_keypress(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_barrier: Query<(), With<ModalDialogBarrier>>,
    mut commands: Commands,
) {
    if q_barrier.contains(ev.event_target()) {
        let event = &ev.event().input;
        if !event.repeat && event.key_code == KeyCode::Escape {
            ev.propagate(false);
            commands.trigger(RequestClose {
                source: ev.event_target(),
            });
        }
    }
}

fn dialog_barrier_on_spawn(add: On<Add, ModalDialog>, mut commands: Commands) {
    let dialog_entity = add.event_target();
    // Need to defer setting focus until children are finished spawning. Note that we don't know,
    // in this module, what API will be used to spawn the dialog, so we have to guess how long
    // this will be.
    commands
        .delayed()
        .secs(0.1)
        .queue(move |world: &mut World| {
            // Check to see if focus is already within dialog (because of autofocus). If it's
            // already set then leave it alone (don't overwrite); otherwise, set focus to the
            // last focusable descendant. Also, if in either case there's a valid focusable entity,
            // then make the focus visible.
            let input_focus = world.resource::<InputFocus>();
            let focus_already_in_modal = input_focus.get().is_some_and(|focused_entity| {
                let mut child_of = world.query::<&ChildOf>();
                child_of
                    .query(world)
                    .iter_ancestors(focused_entity)
                    .any(|e| e == dialog_entity)
            });

            if focus_already_in_modal {
                let mut focus_visible = world.resource_mut::<InputFocusVisible>();
                focus_visible.0 = true;
                return;
            }

            let mut system_state: SystemState<TabNavigation> = SystemState::new(world);
            let tab_navigation = system_state.get(world).unwrap();

            match tab_navigation.initialize(dialog_entity, NavAction::Last) {
                Ok(next) => {
                    let mut focus = world.resource_mut::<InputFocus>();
                    focus.set(next, FocusCause::Navigated);
                    let mut focus_visible = world.resource_mut::<InputFocusVisible>();
                    focus_visible.0 = true;
                }
                Err(e) => {
                    warn!(
                        "No focusable entities in modal dialog: {}, error: {:?}",
                        dialog_entity, e
                    );
                }
            }
        });
}

/// Plugin that adds the observers for the [`ModalDialog`] widget.
pub struct ModalDialogPlugin;

impl Plugin for ModalDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(dialog_barrier_on_spawn)
            .add_observer(dialog_barrier_on_click)
            .add_observer(dialog_barrier_on_keypress);
    }
}
