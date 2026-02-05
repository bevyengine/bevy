---
title: "`bevy_reflect` reorganized to de-clutter the crate root"
pull_requests: [22342]
---

`bevy_reflect` has undergone a large reorganization. Many modules have been exposed in the crate root, each containing items relevant to their "kind" of reflected type:

* `array`
* `enums`
* `list`
* `map`
* `set`
* `structs`
* `tuple`
* `tuple_struct`

For example, the `structs` module now contains the `Struct` trait, as well as related items like `DynamicStruct` or `StructInfo`.

This change was made to de-clutter the crate root of `bevy_reflect`, hopefully making it easier to find what traits and types you need for your use of reflection.

Migrating should only require editing your `use` statements. The Rust compiler will give hints at the new type paths, should you need any assistance.
