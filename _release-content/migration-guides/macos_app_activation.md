---
title: "macOS app activation now follows `Window::focused` on startup and window creation"
pull_requests: [24702]
---

On macOS, apps now request activation (to become the active/frontmost app) on startup only if the `WindowPlugin::primary_window` (or any additional `Window` entities available before `WinitPlugin::build`) has `focused: true`. This allows apps to avoid stealing focus from the user by setting `focused: false`.

In addition, apps now request activation when a visible `Window` is created after startup with `focused: true`. Set `focused: false` if you'd prefer the app to remain inactive.

Apps that only use the default `WindowPlugin::primary_window`, which is initially `focused: true` by default, are unchanged.
