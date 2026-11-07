---
title: Fuzzy matching for tools
authors: ["@jbuehler23"]
pull_requests: [26410]
---

The new `bevy_fuzzy` crate is a small, dependency-free fuzzy matcher for tools like inspectors, pickers and command palettes.
Patterns are split on whitespace and every part must match, ignoring case.
Consecutive characters, word starts and camelCase humps score higher, so `pl` ranks `PointLight` above `Sampler`.

```rust
use bevy::fuzzy::{segments, FuzzyPattern};

let names = ["Sampler", "PointLight", "Transform"];
let ranked = FuzzyPattern::new("pl").rank(&names, |name| *name);
assert_eq!(names[ranked[0].index], "PointLight");

let highlighted: Vec<_> = segments(names[ranked[1].index], &ranked[1].matched.indices).collect();
assert_eq!(highlighted, [("Sam", false), ("pl", true), ("er", false)]);
```
