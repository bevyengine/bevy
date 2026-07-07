---
title: Extract Extract
pull_requests: [24419, 24420, 24423]
---

Extraction used to be specific of Main World to Render World, but will now be generic

- Use `TemporaryRenderEntity::default()` instead of `TemporaryRenderEntity`
- When using extraction related traits e.g. `SyncComponent`, `ExtractComponent` and `ExtractResource`,
you must specify the `AppLabel` for the target world.

Before:

```rust,ignore
impl SyncComponent for TemporalAntiAliasing { ... }

#[derive(Component, ExtractComponent)]
pub struct Foo { ... }
```

After:

```rust,ignore
impl SyncComponent<RenderApp> for TemporalAntiAliasing { ... }

#[derive(Component, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct Foo { ... }
```

You can also extract a component from the main subapp to the render subapp, and to the audio subapp

```rust,ignore
#[derive(Component, Clone, Debug, ExtractComponent)]
#[extract_app(RenderApp, AudioApp)]
struct SomeComponent;
```
