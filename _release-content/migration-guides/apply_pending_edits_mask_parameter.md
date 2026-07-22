---
title: "`apply_pending_edits` takes a mask parameter"
pull_requests: [XXXXX]
---
`EditableText::apply_pending_edits` gained a final parameter
`mask: Option<&mut CharacterMask>`, supporting password-style
character masking:

```rust
// BEFORE
editable_text.apply_pending_edits(&mut font_cx, &mut layout_cx, &mut clipboard, filter);
// AFTER
editable_text.apply_pending_edits(&mut font_cx, &mut layout_cx, &mut clipboard, filter, None);
```

Pass `None` to preserve existing behavior; pass the entity's
`CharacterMask` for masked fields. `apply_text_edits`' query changed
accordingly.

`TextEdit::apply` is unchanged and does not enforce masking; use
`apply_pending_edits` for masked fields.
