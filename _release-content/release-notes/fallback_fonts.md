---
title: "Fallback Fonts"
authors: ["@Ickshonpe"]
pull_requests: [24378]
---

`FontSource` now supports font fallback lists. Use `FontSource::families("Arial, 'Noto Sans', sans-serif")` for CSS-style font family lists, or `FontSource::list([...])` to combine font handles, named families, CSS lists, and generic families.
