---
title: "`RenderAppChannels::new` requires a `MainThreadExecutor`"
pull_requests: [25722]
---

`RenderAppChannels::new` now takes a `MainThreadExecutor` as its third argument to avoid shutdown deadlocks.
Pass a clone of the executor shared by the main and render worlds.
