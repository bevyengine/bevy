---
title: "Input focus is now represented by `Focusable`"
pull_requests: []
---

The new `Focusable` component indicates that an entity may receive input focus, independently of
the navigation method used to reach it. When an explicit sequential-navigation order is not needed,
replace a default `TabIndex` with `Focusable`:

```rust
// Before
TabIndex(0)

// After
Focusable
```

`TabIndex` now only controls sequential navigation and automatically requires `Focusable`.
`TabIndex(-1)` therefore remains the way to make an entity focusable but exclude it from sequential
navigation.

A `Focusable` entity inside a `TabGroup` participates in sequential navigation with an implicit
`TabIndex(0)` unless it has a negative `TabIndex`. Manual directional-navigation destinations must
also have `Focusable`; adding an edge to the `DirectionalNavigationMap` no longer makes its
destination focusable by itself.

`DirectionalNavigation` SystemParam struct gained a second lifetime parameter.
