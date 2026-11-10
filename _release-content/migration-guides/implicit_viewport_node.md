---
title: Implicit viewport node moved under TaffyStyle
pull_requests: [26465]
---
 
The implicit viewport node is no longer public and has moved behind a method returning a reference.

- `VIEWPORT_NODE_TAFFY_STYLE` is now private. Replace all references and imports with `TaffyStyle::viewport_node()`

```rust
// 0.20
let vnode_style = VIEWPORT_NODE_TAFFY_STYLE;

// 0.21
let vnode_style = TaffyStyle::viewport_node();
```
