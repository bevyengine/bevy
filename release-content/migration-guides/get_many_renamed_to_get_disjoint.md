---
title: Renamed `bevy_platform::HashMap::get_many_*` to `bevy_platform::HashMap::get_disjoint_*`
pull_requests: [21898]
---

Matching both [`hashbrown`](https://github.com/rust-lang/hashbrown/pull/648) and the `std` library,
we've renamed all the `get_many_*` methods on `bevy_platform::HashMap` to `get_disjoint_*`. So
rename:

- `get_many_mut` -> `get_disjoint_mut`
- `get_many_unchecked_mut` -> `get_disjoint_unchecked_mut`
- `get_many_key_value_mut` -> `get_disjoint_key_value_mut`
- `get_many_key_value_unchecked_mut` -> `get_disjoint_key_value_unchecked_mut`
