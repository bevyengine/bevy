---
title: "`App::insert_non_send` is deprecated in favor of `App::setup_non_send`"
pull_requests: [25067]
---

`App::insert_non_send` (and the older `App::insert_non_send_resource` alias) construct their
value immediately, at the call site, before the app's runner (e.g. `WinitPlugin`) has chosen
which thread `!Send` data should actually live on. This tightly couples "where you write
app-building code" to "which thread `!Send` data gets built on."

`App::setup_non_send` replaces this: it takes a `Send` closure that is stored and only run once
`App::run` starts, on the thread that calls it, which is where `!Send` data typically needs to
live. `App::insert_non_send` is deprecated in favor of it.

```rust
// 0.19
app.insert_non_send(MyNonSendResource::new());

// 0.20
app.setup_non_send(|world| {
    world.insert_non_send(MyNonSendResource::new());
});
```

`App::init_non_send` is unaffected and does not need to change.
