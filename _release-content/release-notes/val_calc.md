---
title: "Val calc"
authors: ["@ickshonope"]
pull_requests: [26409]
---

`Val` can now hold expressions such as `(percent(50) - px(10)).max(vh(25))` (similar to the CSS `calc` function).

`Val` now supports the `+`, `+=`, `-`, `-=` operators and also has new methods `min`, `max` and `clamp`.

Expressions where all the terms are the same unit are evaluated immediately. Expressions that mix units are stored
on the heap and resolved during layout.
