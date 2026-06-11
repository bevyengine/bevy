---
title: Extract Extract
pull_requests: [24420]
---

Extraction used to be specific of Main World to Render World, but will now be generic

- When using extraction related traits e.g. `SyncComponent`, `ExtractComponent` and `ExtractResource`,
you must specify the `AppLabel` for the target world.

Before:

```rust,ignore
impl SyncComponent for TemporalAntiAliasing { ... }

#[derive(Component, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct Foo { ... }
```

After:

```rust,ignore
impl SyncComponent<RenderApp> for TemporalAntiAliasing { ... }

#[derive(Component, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct Foo { ... }
```

NOTE: more to come
