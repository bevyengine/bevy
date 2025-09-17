---
title: Changes to the default error handler mechanism
pull_requests: [18810]
---

We've improved the implementation of Bevy's default error handling.
The performance overhead has been reduced, and as a result it is always enabled.
The `configurable_error_handler` feature no longer exists: simply remove it from your list of enabled features.

Additionally, worlds can now have different default error handlers, so there is no longer a truly global handler.

Replace uses of `GLOBAL_ERROR_HANDLER` with `App::set_error_handler(handler)`.
For worlds that do not directly belong to an `App`/`SubApp`,
insert the `DefaultErrorHandler(handler)` resource.
