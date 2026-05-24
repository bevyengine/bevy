---
title: Extract Extract
pull_requests: []
---

Extraction used to be specific of Main World to Render World, but will now be generic

- When using traits, specify the `AppLabel`, e.g. `SyncComponent`, `ExtractComponent`

Before:

```rust,ignore
impl SyncComponent for TemporalAntiAliasing {
```

After:

```rust,ignore
impl SyncComponent<RenderApp> for TemporalAntiAliasing {
```

NOTE: more to come
