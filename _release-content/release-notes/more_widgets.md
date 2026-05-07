---
title: "More Feathers widgets"
authors: ["@viridia"]
pull_requests: [23645, 23707, 23788, 23787, 23804, 23817, 23842]
---

*TODO: Add screenshots of the new Feathers widgets (text input, number input, dropdown, pane/group decorators).*

Bevy Feathers, our opinionated UI widget collection designed with the Bevy editor in mind, has added several new widgets this cycle:

- Text input (see the dedicated release note for far more details)
- Number input
- Dropdown menu button and menu divider
- Disclosure toggle (chevron expand/collapse)
- Icon and label (display primitives)
- Pane, subpane, and group (decorative frames for editors)

For full usage and an interactive demo, try out the [`feathers_gallery`] example.

[`feathers_gallery`]: https://github.com/bevyengine/bevy/blob/main/examples/ui/widgets/feathers_gallery.rs

## Feathers + BSN = ❤️

The Feathers widgets are migrating to BSN, Bevy's next-generation scene system.
The new widgets above are BSN-only from the start; the older widgets (button, checkbox, slider, and friends) now have a `bsn!` definition (their original APIs have been deprecated).

BSN is a better foundation for widgets than the old spawn-function approach.
UI controls are inherently multi-entity assemblages — a slider isn't one node, it's a track, a fill, a thumb, and a label wired together.
BSN describes all of that in one place, reduces boilerplate, and lets you compose widgets together, attach props, reference font/image assets and register observers in the same declaration.

```rust
// Before: label children passed as a generic argument, observer wired separately
commands.spawn(checkbox_bundle(
    MyCheckbox,
    children![(Text::new("Enable shadows"), ThemedText)],
)).observe(|trigger: Trigger<ValueChange<bool>>, mut config: ResMut<ShadowConfig>| {
    config.enabled = trigger.value;
});

// After: caption, extra components, and observer all defined in one call
bsn! {
    :FeathersCheckbox {
        @caption: { bsn! { Text("Enable shadows") ThemedText } }
    }
    MyCheckbox
    on(|change: On<ValueChange<bool>>, mut config: ResMut<ShadowConfig>| {
        config.enabled = change.value;
    })
}
```

In the future, the same BSN syntax used in the `bsn!` macro will be portable to `.bsn` files, letting devs choose and rapidly swap between code-first and asset-driven workflows when defining UI.
