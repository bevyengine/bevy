---
title: Font reference counting
authors: ["@ickshonpe"]
pull_requests: [21535]
---

Font use by text entities is now tracked using the new component `ComputedTextFont`. `ComputedTextFont` has `on_insert` and `on_replace` hooks that update a reference counter for each font. Once no references remain, the `FontAtlas`es for the font are placed into buffer, and freed in least recently used order if the number of fonts exceeds `FontAtlasManger::max_fonts`.
