---
title: "`EditableText::apply_pending_edits` takes a mask parameter"
pull_requests: [25117]
---
`apply_pending_edits` gained a final `mask: Option<&CharacterMask>` parameter
(pass `None` for prior behavior) and `apply_text_edits`' query changed
accordingly.
