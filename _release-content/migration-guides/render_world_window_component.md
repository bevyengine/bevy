---
title: Use ECS for render world window data
pull_requests: [25005]
---

The window data used in the render world is now stored directly as a component on a render world entity associated to the window.

If you were using `ExtractedWindows` or `ExtractedWindowSurfaces` you can now use `Query<&ExtractedWindow>` or `Query<&SurfaceData>`.

If you were relying on `ExtractedWindows::primary` you can now use `Query<&ExtractedWindow, With<PrimaryWindow>>`.
