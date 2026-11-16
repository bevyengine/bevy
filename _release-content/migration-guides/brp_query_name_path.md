---
title: "`BrpQueryFilter` has a `name_path` field"
pull_requests: [26250]
---

`BrpQueryFilter` has a new field, `name_path`, which filters a `world.query` by a path of entity names and child indices.
Code that builds a `BrpQueryFilter` with a struct literal must set it, or fill the remaining fields with `..Default::default()`:

```rust
// 0.20
BrpQueryFilter {
    with: vec![type_name::<Button>().to_string()],
    without: Vec::new(),
}

// 0.21
BrpQueryFilter {
    with: vec![type_name::<Button>().to_string()],
    ..Default::default()
}
```

The JSON form of `world.query` is unchanged: `name_path` is optional.
