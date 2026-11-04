---
title: Scrolling any UI node into view
authors: ["@viridia"]
pull_requests: [26262]
---

`ScrollIntoView` now lives in `bevy_ui` and works on any scrolled node, not just headless `ScrollArea` widgets.
Trigger it on a UI node, and every ancestor with `overflow: scroll` is scrolled by the least amount that brings the node into view:

```rust
commands.trigger(ScrollIntoView::new(row));
```

Nested scroll views are handled in a single pass, innermost first, so a row inside a scrolled list inside a scrolled page is brought into view at once.
To scroll only the nearest scrolled ancestor, set `container: ScrollIntoViewContainer::Nearest`.

The behavior follows the DOM's `Element.scrollIntoView()` with `"nearest"` alignment, so it will be familiar to anyone who has used it on the web: a node that's already visible doesn't move, and a node bigger than the view shows the edge it's coming from.

The event is also reflected, so tools that drive a running app over the Bevy Remote Protocol, such as end-to-end tests, can send it with `world.trigger_event` to bring a node into view before clicking on it.
