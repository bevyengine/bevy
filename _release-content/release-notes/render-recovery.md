---
title: "Render Recovery"
authors: ["@atlv24", "@kfc35"]
pull_requests: [22761, 23350, 23349, 23433, 23458, 23444, 23459, 23461, 23463, 22714, 22759, 16481, 24131]
--- 

GPU errors previously had no recovery path — a driver crash, an out-of-memory condition, or a device loss would silently hang or crash the app.
This was particularly frustrating in long-lived applications (like art installations)
or on devices with frequent failures, such as VR headsets.
Bevy now surfaces these as typed errors and lets you decide what to do with each one:

```rust
use bevy::render::error_handler::{ErrorType, RenderErrorHandler, RenderErrorPolicy};

app.insert_resource(RenderErrorHandler(
    |error, main_world, render_world| match error.ty {
        ErrorType::DeviceLost => RenderErrorPolicy::Recover(default()),
        ErrorType::OutOfMemory => RenderErrorPolicy::StopRendering,
        ErrorType::Validation => RenderErrorPolicy::Ignore,
        ErrorType::Internal => panic!(),
    },
));
```

`DeviceLost` is the case most games will want to handle: it covers GPU driver crashes, thermal shutdowns, and hardware being physically disconnected.
`RenderErrorPolicy::Recover` reinitializes the renderer and keeps the app running.
`StopRendering` halts rendering but leaves the rest of the app alive — useful if you want to show an error screen or save state before exiting.
`Ignore` silently swallows the error, which is the existing behavior for validation errors. Panicking remains appropriate for `Internal` errors, which indicate bugs.

Be sure to test your error recovery carefully in your games; we've seen hardware-specific cases of flickering during repeated failures (as might be caused by an out-of-memory problem), which are a serious accessibility risk for people with photosensitive epilepsy.
While we're looking to solve that problem for good in later releases, we've currently opted for a conservative default.
If you don't configure a `RenderErrorHandler`, behavior is similar to but not identical to before: Vulkan validation errors are ignored, everything else sends an `AppExit` event to gracefully shut down.
