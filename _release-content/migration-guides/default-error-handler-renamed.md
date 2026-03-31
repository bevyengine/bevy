---
title: "`DefaultErrorHandler` renamed to `FallbackErrorHandler`"
pull_requests: [TODO]
---

`DefaultErrorHandler` has been renamed to `FallbackErrorHandler` to better reflect its role as the handler of last resort when no specific error handling is performed.

A deprecated type alias is provided for one release to ease migration.
To update your code:

```rust
// Before
world.insert_resource(DefaultErrorHandler(my_error_handler));

// After
world.insert_resource(FallbackErrorHandler(my_error_handler));
```
