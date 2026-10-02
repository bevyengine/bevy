---
title: "`EditableText::value` returns `Cow<str>`"
pull_requests: [25532]
---
`EditableText::value` now returns `Cow<'_, str>` instead of a parley
`SplitString`. Most call sites keep working unchanged: comparisons against
`&str` and `.to_string()` behave as before, and the returned value derefs
to `&str`. The text is usually borrowed and an owned copy is only made as
needed (e.g. while IME composition is active with the editor's text split
around the preedit). Comparisons against `&String` will need the borrow
removed (`Cow<str>` implements `PartialEq<String>` but not
`PartialEq<&String>`).
