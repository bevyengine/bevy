---
title: "Val is no longer Copy"
pull_requests: [26409]
---

`Val` is no longer `Copy`.

It can now hold expressions, such as `Val::Percent(50.) - Val::Px(10.)` (similar to the CSS `calc` function), and
these expressions are heap-allocated. Numeric values and keywords don't allocate, as before.

Code which copied `Val`s must now call `.clone()` or pass by reference instead.
