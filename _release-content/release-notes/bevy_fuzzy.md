---
title: Fuzzy matching for tools
authors: ["@jbuehler23"]
pull_requests: [26410]
---

The new `bevy_fuzzy` crate is a small, dependency-free fuzzy matcher for tools like inspectors, pickers and command palettes.
Patterns are split on whitespace and every part must match, ignoring case.
Consecutive characters, word starts and camelCase humps score higher, so `pl` ranks `PointLight` above `Sampler`.
Scores range from 0.0 to 1.0, where 1.0 is a full match from a word start, so results can be cut off at a threshold.

```rust
use bevy::fuzzy::{segments, FuzzyPattern};

let names = ["Sampler", "PointLight", "Transform"];
let ranked = FuzzyPattern::new("pl").rank(&names, |name| *name);
assert_eq!(names[ranked[0].index], "PointLight");
assert!(ranked[0].matched.score > 0.5);

let highlighted: Vec<_> = segments(names[ranked[1].index], &ranked[1].matched.indices).collect();
assert_eq!(highlighted, [("Sam", false), ("pl", true), ("er", false)]);
```
