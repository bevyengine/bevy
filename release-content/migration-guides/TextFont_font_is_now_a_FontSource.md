---
title: "TextFont's `font` field is now a `FontSource`"
pull_requests: [22156]
---

The type of `TextFont`'s `font` field has been changed from a `Handle<Font>` to a `FontSource`. `FontSource` has two variants: `Handle`, which identifies a font by asset handle, and `Family`, which selects a font by its family name.

`FontSource` implements `From<Handle<Font>>`, migration of existing code should only require calling `into()` on the handle.

Font texture atlases are no longer automatically cleared when the font asset they were generated from is removed. This is because there is no way to remove individual fonts from cosmic text's `FontSystem`. So even after the asset is removed, the font is still accessible using the family name with `FontSource::family` and removing the text atlases naively could cause a panic as rendering expects them to be present.
