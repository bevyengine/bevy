---
title: "New `Node::direction` field"
pull_requests: [23605]
---

`Node` has a new field `direction`, which can be used to set the inline axis direction used for layout.
By default it uses `InlineDirection::Ltr`, which should match the layout behavior before this change.
