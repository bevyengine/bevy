---
title: "`EditableText::value` returns `Cow<str>`"
pull_requests: [25117]
---
`EditableText::value` now returns `Cow<'_, str>` instead of parley's
`SplitString`. Most call sites keep working unchanged: comparisons against
`&str` and `.to_string()` behave as before, and the returned value derefs to
`&str`. The text is borrowed in the common case; an owned copy is only made
while an IME composition is active (the editor's text is split around the
preedit).

The returned text is now always the *entered* text: with a `CharacterMask`
present, `value` previously returned the mask glyphs and the real text had to
be read from `CharacterMask::value`, which no longer exists. To pre-populate
a masked field, set the text on `EditableText` (e.g. `EditableText::new(...)`)
before adding the mask.
