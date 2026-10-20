---
title: "`ScrollIntoView` moved to `bevy_ui`"
pull_requests: [26262]
---

`ScrollIntoView` has moved from `bevy_ui_widgets` to `bevy_ui`, and gained a `container` field.
It now scrolls every ancestor with `OverflowAxis::Scroll` by default, not just the nearest `ScrollArea`.
To keep the old behavior, ask for the nearest container:

```rust
// 0.20
use bevy::ui_widgets::ScrollIntoView;
commands.trigger(ScrollIntoView { entity });

// 0.21
use bevy::ui::{ScrollIntoView, ScrollIntoViewContainer};
commands.trigger(ScrollIntoView {
    entity,
    container: ScrollIntoViewContainer::Nearest,
});
```

The event no longer propagates.
