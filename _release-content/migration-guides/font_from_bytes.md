---
title: "`Font::from_bytes` no longer takes a family name"
pull_requests: [24362]
---

`Font::from_bytes` no longer takes a family name. Loaded font assets are now automatically registered with an internal asset-specific alias for handle lookups, and with their embedded family name from the font data.
