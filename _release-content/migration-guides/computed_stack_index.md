---
title: "`ComputedNode::stack_index` has been replaced by `ComputedStackIndex`"
pull_requests: [23878]
---

The `stack_index` field has been removed from `ComputedNode`. Instead the stack index for each UI node is stored on a specialized component `ComputedStackIndex`, which is required by `ComputedNode`.
