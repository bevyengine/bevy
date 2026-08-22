---
title: Password-style character masking for text inputs
authors: ["@Cyannide"]
pull_requests: [25117]
---

Text inputs can now conceal their content: add the `CharacterMask` component
to an `EditableText` entity and the field displays one mask glyph per entered
character, while `EditableText::value` keeps returning the real text — the
mask affects display only. Removing the component reveals the text again, so
a show/hide-password toggle is just inserting and removing a component.

Masked fields behave like platform password inputs: copy and cut never place
the real text on the clipboard, paste routes into the concealed value, and
IME input commits per keystroke without displaying the in-progress
composition. Character filters and `max_characters` apply to the entered
characters, not the mask glyphs.
