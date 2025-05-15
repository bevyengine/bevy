---
title: Added support for fragmenting value components
pull_requests: [19153]
---

Archetypes can now be fragmented by component values. Supporting this required some changes to the public api of archetypes and a new associated type for components.

Manual impl of `Component` trait now has a new associated type, `Key`, that should be set to `NoKey<Self>` for all existing components.

`ComponentDescriptor::new` and `ComponentDescriptor::new_with_layout` now require an additional argument - `fragmenting_value_vtable: Option<FragmentingValueVtable>`. It should be set to `None` for all existing implementations.

`Edges::get_archetype_after_bundle_insert` now require an additional argument - `value_components: &FragmentingValuesBorrowed`. These can be
constructed using `FragmentingValuesBorrowed::from_bundle`.
