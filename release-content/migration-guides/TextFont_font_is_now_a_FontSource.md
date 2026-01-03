---
title: "TextFont's `font` field is now a `FontSource`"
pull_requests: [22156]
---

The type of `TextFont`'s `font` field has been changed from a `Handle<Font>` to a `FontSource`. `FontSource` has two variants: `Handle`, which identifies a font by asset handle, and `Family`, which selects a font by its family name.

`FontSource` implements `From<Handle<Font>>`, migration of existing code should only require calling `into()` on the handle.
