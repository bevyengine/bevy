---
title: "`TextRoot`, `TextSpanAccess` and `TextSpanComponent` are replaced by `TextSection`"
pull_requests: [23423]
---

The `TextRoot`, `TextSpanAccess` and `TextSpanComponent` have been consolidated into a single trait `TextSection`.

The methods `read_span` and `write_span` have been renamed to `get_text` and `get_text_mut`, respectively.
