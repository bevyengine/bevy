---
title: Keyboard symbols in Feathers
authors: ["@viridia"]
pull_requests: [26242]
---

Fira Sans, the font used by Feathers, has no glyphs for modifier keys such as command (⌘), shift (⇧) and option (⌥), which an editor needs in order to display keyboard shortcuts.
Feathers now includes a small supplemental font, Key Symbols, that contains these glyphs.

Feathers widgets now set their font with the family list `fonts::SANS` (`"Fira Sans, Key Symbols"`), so any character missing from Fira Sans falls back to Key Symbols automatically.
As a result, a caption such as `"Quit… ⌘Q"` renders correctly.
To make this possible, `InheritableFont` now accepts any `FontSource`, not only a font handle.
`FeathersCorePlugin` loads the fonts that the family list refers to.
Selecting the family by name also lets Feathers render bold text with the real Fira Sans Bold face.
