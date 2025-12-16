---
title: "`#[reflect(...)]` now supports only parentheses" 
pull_requests: [21400]
---

Previously, the `#[reflect(...)]` attribute of the `Reflect` derive macro
supported parentheses, braces, or brackets, to standardize the syntax going
forward, it now supports only parentheses.

```rust
/// before
#[derive(Clone, Reflect)]
#[reflect[Clone]]

/// after
#[derive(Clone, Reflect)]
#[reflect(Clone)]

/// before
#[derive(Clone, Reflect)]
#[reflect{Clone}]

/// after
#[derive(Clone, Reflect)]
#[reflect(Clone)]
```
