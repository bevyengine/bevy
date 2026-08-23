---
title: Headless tab widgets
authors: ["@jbuehler23"]
pull_requests: [25515]
---

`bevy_ui_widgets` now has headless tab-strip behavior: a `TabList` container and `Tab` headers, with no built-in visuals.

Selection is externally owned. `SelectedTab` on the list holds the selected tab; interaction emits `ValueChange<Option<Entity>>` as a request, applied by the app or by the optional `tablist_self_update` observer.

Arrow keys move a roving tab index along the list's axis, Home and End jump to the ends, and Enter or Space activates. `TabActivation::Automatic` selects as focus moves; the default `Manual` keeps focus and selection separate.

Tabs derive `Selected`, install tab accessibility roles, and honor `InteractionDisabled` per tab or on the whole list. Only primary-button clicks change selection.

```rust
(
    TabList::default()
    SelectedTab(Some(first_tab))
    on(tablist_self_update)
    Children [
        (Tab Children [Text("General")]),
        (Tab Children [Text("Rendering")]),
    ]
),
```

See the `headless_tabs` example for controlled and self-updating tab lists in both orientations.
