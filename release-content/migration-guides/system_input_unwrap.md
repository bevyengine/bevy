---
title: "New required method on SystemInput"
pull_requests: [21811]
---

Custom implementations of the `SystemInput` trait will need to implement a new
required method: `unwrap`. Like `wrap`, it converts between the inner input item
and the wrapper, but in the opposite direction. In most cases it should be
trivial to add.
