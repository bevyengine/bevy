---
title: Consistent `*Systems` naming convention for system sets
authors: ["@Jondolf"]
pull_requests: [18900]
---

Names of `SystemSet` types within Bevy and its ecosystem have historically
been very inconsistent. Examples of system set names include `AccessibilitySystem`,
`PickSet`, `StateTransitionSteps`, and `Animation`.

Naming conventions being so wildly inconsistent can make it harder for users to pick names
for their own types, to search for system sets on docs.rs, or to even discern which types
*are* system sets.

To reign in the inconsistency and help unify the ecosystem, **Bevy 0.17** has renamed most of
its own system sets to follow a consistent `*Systems` naming convention.
As you can see by this very incomplete list of renames, our naming was all over the place:

- `GizmoRenderSystem` → `GizmoRenderSystems`
- `PickSet` → `PickingSystems`
- `Animation` → `AnimationSystems`
- `Update2dText` → `Text2dUpdateSystems`

The `Systems` suffix was chosen over the other popular suffix `Set`,
because `Systems` more clearly communicates that it is specifically
a collection of systems, and it has a lower risk of naming conflicts
with other set types.

For consistency, we recommend that ecosystem crates and users to follow suit and also adopt
the `*Systems` naming convention for their system sets where applicable.
