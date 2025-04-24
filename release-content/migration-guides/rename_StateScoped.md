---
title: `StateScoped` renamed to `DespawnOnExitState`
pull_requests: [18818]
---

Previously, Bevy provided the `StateScoped` component as a way to despawn an entity when **exiting** a state.

However, it can also be useful to have the opposite behavior, where an entity is despawned when **entering** a state. This is now possible with the new `DespawnOnEnterState` component.

To support despawning entities when entering a state, in Bevy 0.17 the `StateScoped` component was renamed to `DespawnOnExitState` and `clear_state_scoped_entities` was renamed to `despawn_entities_on_exit_state`. Replace all references and imports.
