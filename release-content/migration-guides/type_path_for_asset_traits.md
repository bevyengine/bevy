---
title: Traits `AssetLoader`, `AssetTransformer`, `AssetSaver`, and `Process` all now require `TypePath`
pull_requests: [21339]
---

The `AssetLoader`, `AssetTransformer`, `AssetSaver`, and `Process` traits now include a super trait
of `TypePath`. This means if you previously had a loader like:

```rust
struct MyFunkyLoader {
    add_funk: u32,
}
```

You will need to add the following derive:

```rust
#[derive(TypePath)]
struct MyFunkyLoader {
    add_funk: u32,
}
```

`TypePath` comes from `bevy_reflect`, so libraries may also need to add a dependency on
`bevy_reflect`.
