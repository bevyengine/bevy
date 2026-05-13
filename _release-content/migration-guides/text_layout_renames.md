---
title: "`new_with_` prefix removed from `TextLayout` constructors"
pull_requests: [24049]
---

Constructor functions for the `TextLayout` type were simplified:

- `TextLayout::new_with_justify` -> `TextLayout::justify`
- `TextLayout::new_with_linebreak` -> `TextLayout::linebreak`
- `TextLayout::new_with_no_wrap` -> `TextLayout::no_wrap`
