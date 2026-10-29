---
title: "`AnimationTransitions` supports multiple independent flows"
pull_requests: [18954]
---

`AnimationTransitions` no longer tracks a single "main" animation.
Instead, it manages a number of independent *flows*, each with its own current animation, so that e.g. full body and hand animations can be crossfaded separately.

`AnimationTransitions::new()` creates a single flow, which behaves like the previous main animation. Use `AnimationTransitions::new_with_flow` to create more.

The following items have been renamed:

| Before                                    | After                                         |
|-------------------------------------------|-----------------------------------------------|
| `AnimationTransitions::play`              | `AnimationTransitions::transition`            |
| `AnimationTransitions::get_main_animation`| `AnimationTransitions::current_node(0)`       |
| `advance_transitions`                     | `handle_node_transition`                      |

```rust
// Before
transitions
    .play(&mut player, run, Duration::from_millis(250))
    .repeat();
let main = transitions.get_main_animation();

// After
transitions
    .transition(&mut player, run, Duration::from_millis(250))
    .repeat();
let main = transitions.current_node(0);
```

To transition a flow other than the first, use `AnimationTransitions::transition_flows`, which returns `None` if the flow is out of bounds, or `AnimationTransitions::transition_flows_unchecked`, which panics instead:

```rust
let mut transitions = AnimationTransitions::new_with_flow(2);

// Flow 0 animates the whole body.
transitions.transition_flows_unchecked(&mut player, walk, 0, Duration::from_millis(250));
// Flow 1 animates only the hand bones, through an animation mask.
transitions.transition_flows_unchecked(&mut player, fist, 1, Duration::from_millis(100));
```
