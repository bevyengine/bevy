---
title: Renamed state scoped entities and events
pull_requests: [18818, 19435]
---

Previously, Bevy provided the `StateScoped` component and `add_state_scoped_event` method
as a way to remove entities/events when **exiting** a state.

However, it can also be useful to have the opposite behavior,
where entities/events are removed when **entering** a state.
This is now possible with the new `DespawnOnEnterState` component and `clear_events_on_enter_state` method.

To support this addition, the previous method and component have been renamed.
Also, `clear_event_on_exit_state` no longer adds the event automatically, so you must call `App::add_event` manually.

| Before                        | After                                      |
|-------------------------------|--------------------------------------------|
| `StateScoped`                 | `DespawnOnExitState`                       |
| `clear_state_scoped_entities` | `despawn_entities_on_exit_state`           |
| `add_state_scoped_event`      | `add_event` + `clear_events_on_exit_state` |
