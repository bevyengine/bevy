---
title: "BRP method handlers share one system type"
pull_requests: [26462]
---

`RemoteMethodHandler` should now be used instead of `RemoteMethodSystemId`, and handles registering a handler in the world.

```rust
// 0.20
let system = world.register_system(my_handler);
methods.insert("my.method", RemoteMethodSystemId::Instant(system));

// 0.21
let method = RemoteMethodHandler::Instant(Box::new(IntoSystem::into_system(my_handler))).register(world);
methods.insert("my.method", method);
```

The HTTP transport now streams a response over SSE when the method was registered as watching, instead of when its name contains `+watch`.
