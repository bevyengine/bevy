---
title: Same State Transitions
pull_requests: [19363]
---

It is now possible to change to the same state, triggering state transitions.

```rust
// Before: did nothing if the state was already `State::Menu`
next_state.set(State::Menu);
// After: trigger state transitions even if the state is already `State::Menu`
next_state.set_forced(State::Menu);
```
