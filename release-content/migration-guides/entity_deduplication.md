---
title: "`UnsafeEntityCell` functions now have an `AsAccess` parameter"
pull_requests: [22538]
---

The following functions now return a `Result` with a proper error, instead of an
`Option`. Handle accordingly.

- `FilteredEntityRef::get_by_id`
- `FilteredEntityMut::get_by_id`
- `FilteredEntityMut::get_mut_by_id`
- `FilteredEntityMut::get_mut_by_id_unchecked`
- `EntityRefExcept::get_by_id`
- `EntityMutExcept::get_by_id`
- `EntityMutExcept::get_mut_by_id`

The following functions now take an `AsAccess` as an additional argument.
You should pass an access type that most closely matches your access patterns,
and ensure it abides by Rust aliasing rules.

- `UnsafeEntityCell::get`
- `UnsafeEntityCell::get_ref`
- `UnsafeEntityCell::get_change_ticks`
- `UnsafeEntityCell::get_change_ticks_by_id`
- `UnsafeEntityCell::get_mut`
- `UnsafeEntityCell::get_mut_assume_mutable`
- `UnsafeEntityCell::get_by_id`
- `UnsafeEntityCell::get_mut_by_id`
- `UnsafeEntityCell::get_mut_assume_mutable_by_id`

For example, if your cell can access all components without violating aliasing
rules, use `All`. If your cell can only access a specific set of
components without violating aliasing rules, consider using `Filtered` or `Except`.
If you are able to validate externally that you won't violate aliasing
rules by accessing a particular component, you may use `All`.
