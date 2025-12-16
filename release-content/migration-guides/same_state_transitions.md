---
title: Same State Transitions
pull_requests: [19363, 21792]
---

Setting the next state will now always trigger state transitions like `OnEnter` and `OnExit`, even if the state is already the same.
If you depend on the previous behavior, you can use the `set_if_neq` method instead.

```rust
// 0.17
next_state.set(State::Menu);

// 0.18
next_state.set_if_neq(State::Menu);
```
