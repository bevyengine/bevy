---
title: "`GatedReader` and `GatedOpener` are now private."
pull_requests: [18473]
---

The `GatedReader` and `GatedOpener` for `bevy_asset` have been made private. These were really only
for testing, but were being compiled even in release builds. Now they are guarded by `#[cfg(test)]`!

If you were using this in your own tests, you could fork the `GatedReader` (it still exists in the
Bevy repo!) into your own code, or write your own version (if more useful to you).
