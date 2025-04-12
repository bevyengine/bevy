---
title: `StateScoped` renamed to `DespawnOnExitState`
pull_requests: [18818]
---

Adding the `StateScoped` component to an entity causes it to be despawned when **exiting** a state.

It can be useful to have the opposite behavior, where entities are despawned when **entering** a state. This is now possible with the new `DespawnOnEnterState` component.

To support despawning entities when entering a state, in Bevy 0.17 `StateScoped` is now `DespawnOnExitState` and `clear_state_scoped_entities` is now `despawn_entities_on_exit_state`. Replace all references and imports.
