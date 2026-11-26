---
title: "`Table::get_component` is renamed to `Table::get_component_untyped`"
pull_requests: [26315]
---

`Table::get_component` is renamed to `Table::get_component_untyped`. Update your calls to `Table::get_component_untyped`.

`Table::get_component` now takes the component type as the type parameter `T` and returns `Option<&UnsafeCell<T>>`.
