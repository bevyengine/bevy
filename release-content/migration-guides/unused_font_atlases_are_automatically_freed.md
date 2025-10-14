---
title: "Unused font atlases are now freed automatically."
pull_requests: [21535]
---

Font atlases are now freed automatically in least recently used order once they are no longer in use by any text entities. The `max_fonts` field of `FontManager` controls the maximum number of fonts before unused font atlases are freed. In use fonts are never freed, even if the number of in use fonts is greater than `max_fonts`.