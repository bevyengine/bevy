---
title: Reflect Struct QOL
pull_requests: [22708]
---

The `bevy_reflect::Struct` trait has taken ownership of the previously `bevy_reflect::DynamicStruct::index_of` and is now `bevy_reflect::Struct::index_of_name`.
Most utility methods existed already on `Struct`, except for the method to get a fields index by name.

The `bevy_reflect::FieldIter` iterator had its items changed,
from `&dyn PartialReflect` to a tuple of `(&str, &dyn PartialReflect)`, so that you can iterate the names alongside the fields.
