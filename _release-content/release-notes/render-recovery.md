---
title: "Render Recovery"
authors: ["@atlv24"]
pull_requests: [22761, 23350, 23349, 23433, 23458, 23444, 23459, 23461, 23463, 22714, 22759, 16481]
--- 

You can now recover from rendering errors such as device loss by reloading the renderer:

```rs
use bevy::render::error_handler::{ErrorType, RenderErrorHandler, RenderErrorPolicy};

app.insert_resource(RenderErrorHandler(
    |error, main_world, render_world| match error.ty {
        ErrorType::Internal => panic!(),
        ErrorType::OutOfMemory => RenderErrorPolicy::StopRendering,
        ErrorType::Validation => RenderErrorPolicy::Ignore,
        ErrorType::DeviceLost => RenderErrorPolicy::Recover(default()),
    },
));
```

NOTE: this is just an example showing the different errors and policies available, and not a recommendation for how to handle errors.

The default error handler behaves identically to how Bevy behaved before: validation errors are ignored, and other errors crash/hang the application.
