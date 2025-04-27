---
title: Global default error handler
pull_requests: [18810]
---

Worlds can now have different default error handlers, so there no longer is a global handler.

Replace uses of `GLOBAL_ERROR_HANDLER` with `App`'s `.set_error_handler(handler)`.
For worlds that do not directly belong to an `App`/`SubApp`,
insert the `DefaultErrorHandler(handler)` resource.
