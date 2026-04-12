//! Contains [`FocusGained`] and [`FocusLost`] events,
//! as well as [`process_recorded_focus_changes`] to send them when the focused entity changes.

use super::InputFocus;
use bevy_ecs::prelude::*;

/// An [`EntityEvent`] that is sent when an entity gains [`InputFocus`].
///
/// This event bubbles up the entity hierarchy, so if a child entity gains focus, its parents will also receive this event.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(auto_propagate)]
pub struct FocusGained {
    /// The entity that gained focus.
    pub entity: Entity,
}

/// An [`EntityEvent`] that is sent when an entity loses [`InputFocus`].
///
/// This event bubbles up the entity hierarchy, so if a child entity loses focus, its parents will also receive this event.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(auto_propagate)]
pub struct FocusLost {
    /// The entity that lost focus.
    pub entity: Entity,
}

/// Reads the recorded focus changes from the [`InputFocus`] resource and sends the appropriate [`FocusGained`] and [`FocusLost`] events.
///
/// This system is part of [`InputFocusPlugin`](super::InputFocusPlugin).
pub fn process_recorded_focus_changes(mut focus: ResMut<InputFocus>, mut commands: Commands) {
    // This function does not actually mutate the `focus.current_focus`, which is
    // what is exposed to the user via `InputFocus::get`. Other fields are not exposed.
    // So, we `bypass_change_detection` when accessing `focus` to avoid false signaling
    // that we changed the `current_focus`. That is what users would care about if
    // they were to be checking `focus.is_changed()`.

    // We need to track the previous focus as we go,
    // so we can send the correct FocusLost events when focus changes.
    let mut previous_focus = focus.original_focus;
    for change in focus.bypass_change_detection().recorded_changes.drain(..) {
        match change {
            Some(new_focus) => {
                if let Some(old_focus) = previous_focus {
                    commands.trigger(FocusLost { entity: old_focus });
                }
                commands.trigger(FocusGained { entity: new_focus });
                previous_focus = Some(new_focus);
            }
            None => {
                if let Some(old_focus) = previous_focus {
                    commands.trigger(FocusLost { entity: old_focus });
                }
                previous_focus = None;
            }
        }
    }

    focus.bypass_change_detection().original_focus = focus.current_focus;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
    use bevy_app::App;
    use bevy_ecs::observer::On;
    use bevy_input::InputPlugin;

    /// Tracks the sequence of [`FocusGained`] and [`FocusLost`] events for assertions.
    #[derive(Debug, Clone, PartialEq)]
    enum FocusEvent {
        Gained(Entity),
        Lost(Entity),
    }

    #[derive(Resource, Default)]
    struct FocusEventLog(Vec<FocusEvent>);

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins((InputPlugin, super::super::InputFocusPlugin));
        app.init_resource::<FocusEventLog>();

        app.add_observer(|trigger: On<FocusGained>, mut log: ResMut<FocusEventLog>| {
            log.0.push(FocusEvent::Gained(trigger.entity));
        });
        app.add_observer(|trigger: On<FocusLost>, mut log: ResMut<FocusEventLog>| {
            log.0.push(FocusEvent::Lost(trigger.entity));
        });

        // Run once to finish startup
        app.update();

        app
    }

    // Convenience method to extract and clear the log values for assertions
    fn take_log(app: &mut App) -> Vec<FocusEvent> {
        core::mem::take(&mut app.world_mut().resource_mut::<FocusEventLog>().0)
    }

    #[test]
    fn no_changes_no_events() {
        let mut app = setup_app();

        app.update();
        assert!(take_log(&mut app).is_empty());
    }

    #[test]
    fn gain_focus_from_none() {
        let mut app = setup_app();

        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<InputFocus>().set(entity);
        app.update();

        assert_eq!(take_log(&mut app), vec![FocusEvent::Gained(entity)]);
    }

    #[test]
    fn lose_focus_to_none() {
        let mut app = setup_app();
        let entity = app.world_mut().spawn_empty().id();

        // Establish initial focus.
        app.world_mut().resource_mut::<InputFocus>().set(entity);
        app.update();
        take_log(&mut app);

        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();

        assert_eq!(take_log(&mut app), vec![FocusEvent::Lost(entity)]);
    }

    #[test]
    fn switch_focus_between_entities() {
        let mut app = setup_app();
        let a = app.world_mut().spawn_empty().id();
        let b = app.world_mut().spawn_empty().id();

        app.world_mut().resource_mut::<InputFocus>().set(a);
        app.update();
        take_log(&mut app);

        app.world_mut().resource_mut::<InputFocus>().set(b);
        app.update();

        assert_eq!(
            take_log(&mut app),
            vec![FocusEvent::Lost(a), FocusEvent::Gained(b)]
        );
    }

    #[test]
    fn multiple_changes_in_single_frame() {
        let mut app = setup_app();
        take_log(&mut app);

        let a = app.world_mut().spawn_empty().id();
        let b = app.world_mut().spawn_empty().id();
        let c = app.world_mut().spawn_empty().id();

        let mut focus = app.world_mut().resource_mut::<InputFocus>();
        focus.set(a);
        focus.set(b);
        focus.clear();
        focus.set(c);

        app.update();

        assert_eq!(
            take_log(&mut app),
            vec![
                FocusEvent::Gained(a),
                FocusEvent::Lost(a),
                FocusEvent::Gained(b),
                FocusEvent::Lost(b),
                FocusEvent::Gained(c),
            ]
        );
    }

    #[test]
    fn set_focus_to_same_entity() {
        let mut app = setup_app();
        let entity = app.world_mut().spawn_empty().id();

        app.world_mut().resource_mut::<InputFocus>().set(entity);
        app.update();
        take_log(&mut app);

        // Setting focus to the already-focused entity still records a change.
        app.world_mut().resource_mut::<InputFocus>().set(entity);
        app.update();

        assert_eq!(
            take_log(&mut app),
            vec![FocusEvent::Lost(entity), FocusEvent::Gained(entity)]
        );
    }

    #[test]
    fn clear_when_already_none() {
        let mut app = setup_app();
        take_log(&mut app);

        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();

        // No entity was focused, so no FocusLost should fire.
        assert!(take_log(&mut app).is_empty());
    }

    #[test]
    fn double_clear() {
        let mut app = setup_app();
        let entity = app.world_mut().spawn_empty().id();

        app.world_mut().resource_mut::<InputFocus>().set(entity);
        app.update();
        take_log(&mut app);

        // Clear twice — only one FocusLost should fire (the second clear has no previous focus).
        let mut focus = app.world_mut().resource_mut::<InputFocus>();
        focus.clear();
        focus.clear();
        app.update();

        assert_eq!(take_log(&mut app), vec![FocusEvent::Lost(entity)]);
    }

    #[test]
    fn events_propagate_to_parent() {
        let mut app = setup_app();
        take_log(&mut app);

        let child = app.world_mut().spawn_empty().id();
        let parent = app.world_mut().spawn_empty().add_child(child).id();

        app.world_mut().resource_mut::<InputFocus>().set(child);
        app.update();

        // The event fires on the child, then bubbles to the parent.
        let log = take_log(&mut app);
        assert!(
            log.contains(&FocusEvent::Gained(child)),
            "child should receive FocusGained"
        );
        assert!(
            log.contains(&FocusEvent::Gained(parent)),
            "parent should receive FocusGained via propagation"
        );

        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();

        let log = take_log(&mut app);
        assert!(
            log.contains(&FocusEvent::Lost(child)),
            "child should receive FocusLost"
        );
        assert!(
            log.contains(&FocusEvent::Lost(parent)),
            "parent should receive FocusLost via propagation"
        );
    }

    #[test]
    fn focus_lost_on_despawned_entity() {
        let mut app = setup_app();
        let entity = app.world_mut().spawn_empty().id();

        app.world_mut().resource_mut::<InputFocus>().set(entity);
        app.update();
        take_log(&mut app);

        // Record a focus change away from the entity, then despawn it before processing.
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.world_mut().entity_mut(entity).despawn();
        app.update();

        // FocusLost should still fire (and not panic).
        let log = take_log(&mut app);
        assert_eq!(log, vec![FocusEvent::Lost(entity)]);
    }

    #[test]
    fn from_entity_fires_gained_event() {
        let mut app = setup_app();
        take_log(&mut app);

        let entity = app.world_mut().spawn_empty().id();
        app.world_mut()
            .insert_resource(InputFocus::from_entity(entity));
        app.update();

        let log = take_log(&mut app);
        assert!(
            log.contains(&FocusEvent::Gained(entity)),
            "from_entity should record a change that fires FocusGained"
        );
    }
}
