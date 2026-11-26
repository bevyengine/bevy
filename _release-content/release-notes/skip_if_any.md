---
title: "SkipIfAny"
authors: ["@BrainBacon"]
pull_requests: [25852]
---

When Fallible System Parameters were [introduced in 0.15](/news/bevy-0-15), the `Single` and `Populated` query params were added. Those can skip system execution if there isn't exactly one result, or no results respectively.

Now `SkipIfAny<F>` has been added to this family. This variant will skip the system if there are any results matched by the provided filter.

This exists as an alternative to using system run conditions such as `not(any_match_filter::<F>)`.

Note that `SkipIfAny` does not contain any data, so make sure that you prefix your arguments with an underscore `_` so they don't cause unused warnings.

```rust
fn init_selection(
    mut commands: Commands,
    item_query: Populated<Entity, With<MenuItem>>,

    // Don't forget to prefix the argument with _ or an unused error will occur
    _skip_existing: SkipIfAny<With<MenuSelection>>,
) {
    let Some(first) = item_query.iter().next() else {
        unreachable!();
    };

    commands.entity(first).try_insert(MenuSelection);
}

```
