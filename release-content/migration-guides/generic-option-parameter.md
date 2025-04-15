---
title: Generic `Option` Parameter
pull_requests: [18766]
---

`Option<Single<D, F>>` will now resolve to `None` if there are multiple entities matching the query.
Previously, it would only resolve to `None` if there were no entities, and would skip the system if there were multiple.

We have introduced a blanket `impl SystemParam for Option` that resolves to `None` if the parameter is invalid.
This allows third-party system parameters to work with `Option`, and makes the behavior more consistent.

If you want a system to run when there are no matching entities but skip when there are multiple,
you will need to use `Query<D, F>` and call `single()` yourself.

```rust
// 0.16
fn my_system(single: Option<Single<&Player>>) {
}

// 0.17
fn my_system(query: Query<&Player>) {
    let result = query.single();
    if matches!(r, Err(QuerySingleError(MultipleEntities(_)))) {
        return;
    }
    let single: Option<&Player> = r.ok();
}
```
