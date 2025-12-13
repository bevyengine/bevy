---
title: Make BEVY_REFLECT_AUTO_REGISTER_STATIC require a target dir
authors: ["@eugineerd"]
pull_requests: [21494]
---

`BEVY_REFLECT_AUTO_REGISTER_STATIC` env var, which is required to be set when using `reflect_auto_register_static` feature, now must contain
absolute path to current project's `target` dir.
This fixes a bug where this feature didn't work for projects that obtained `bevy` from a remote source like `crates.io` or through `git`.
`auto_register_static` example was updated to suggest how this path can be obtained automatically for any project managed by `cargo`.
