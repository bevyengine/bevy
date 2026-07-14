---
title: "`PartialReflect::to_dynamic` and its helpers now return a `Result`"
pull_requests: [24748]
---

`PartialReflect::to_dynamic` now returns `Result<Box<dyn PartialReflect>, ReflectCloneError>`
rather than panicking.
These methods will fail if *any* value stored inside
is an opaque type whose `reflect_clone` fails, *including* nested opaque values.

In order to make that change properly robust, the per-kind helpers are now fallible as well, returning
`Result<_, ReflectCloneError>`:

- `Struct::to_dynamic_struct`
- `TupleStruct::to_dynamic_tuple_struct`
- `Tuple::to_dynamic_tuple`
- `List::to_dynamic_list`
- `Array::to_dynamic_array`
- `Map::to_dynamic_map`
- `Set::to_dynamic_set`
- `Enum::to_dynamic_enum`

Similarly, `DynamicEnum::from` and `DynamicEnum::from_ref` have been deprecated in favor of `try_from` equivalents,
which now return `Result<DynamicEnum, ReflectCloneError>`.

Finally, `PartialReflect::try_apply` (and `apply`) build dynamic values internally when applying a value
onto a larger collection or a different enum variant.
That conversion can now fail (previously it would panic),
so `ApplyError` has grown a new `CloneError(ReflectCloneError)` variant.

The migration here should be easy:
if you were okay with panicking before, just call `.unwrap()`: all panicking cases
have been replaced with an error, and no new failing paths were added.

However, if your code was defensively guarding against the old panic,
you can now handle the returned `Result` directly instead and simplify your error handling.
