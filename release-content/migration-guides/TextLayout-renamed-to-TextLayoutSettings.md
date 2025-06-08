---
title: `bevy_text` components refactor
pull_requests: [19444]
---

Renamed `TextLayoutInfo` to `ComputedTextLayout`.
It contains the finalized text layout, the `-Info` suffix is redundant and potentially confusing.

Renamed `ComputedTextBlock` to `TextBuffer`.
This component wraps the cosmic-text buffer. The name `ComputedTextBlock` suggests that it contains the final result of a text relayout, but the buffer can be out-of-date until it is updated during the text schedule.

Removed `TextLayout`. Contains the linebreak and justification settings, not the text layout.

`JustifyText` and `Linebreak` are now components required by `Text` and `Text2d`.
