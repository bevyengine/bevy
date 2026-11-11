---
title: Password-style character masking for text inputs
authors: ["@Cyannide"]
pull_requests: [25117]
---

Text inputs can now conceal their content: set `EditableText::mask` to a glyph
(e.g. `Some('*')`) and the field displays that glyph once per entered
character, while `EditableText::value` keeps returning the real text...the
mask affects display only. Setting it back to `None` reveals the text again,
and the caret and selection stay put, so a show/hide-password toggle is a
single field write.

Masked fields behave like platform password inputs: copy and cut never place
the real text on the clipboard, paste routes into the concealed value, and
IME input commits per keystroke without displaying the in-progress
composition. Character filters and `max_characters` apply to the entered
characters only, not the mask glyphs.
