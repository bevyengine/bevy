---
title: Lifecycle event changes
pull_requests: [22789]
---

`Replace` has been renamed to `Discard` and `ComponentHooks::on_replace` has been renamed to `ComponentHooks::on_discard`.
The `#[component(on_replace = ...)]` derive attribute is now `#[component(on_discard = ...)]`.
Replace all references and imports.
