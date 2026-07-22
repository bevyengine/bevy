---
title: "`bevy_ui::widget::TextScroll` has been replaced by `EditableText::viewport`"
pull_requests: [24634]
---

`bevy_ui::widget::TextScroll` has been removed. Editable text scroll state is now stored in `EditableText::viewport`, using the new `bevy_text::TextViewport` type. `EditableText::viewport.offset` is the direct replacement for `TextScroll`.

The `scroll_editable_text` system has also been removed, cursor reveal behavior is now handled automatically when `TextEdit`s are applied.

A new system `sync_editable_text_viewports` in `bevy_ui` synchronizes each `EditableText`'s viewport size with the size of its respective `ComputedNode`.
