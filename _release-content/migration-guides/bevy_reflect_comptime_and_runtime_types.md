---
title: "Update terminology around comptime and runtime types"
pull_requests: [24585]
---

This release introduces new reflection terminology: "comptime" and "runtime" types.
This was done to simplify the mental model and lessen the overloaded usage of "dynamic".

In Bevy's reflection system, "dynamic types" are types that serve to proxy other canonical types.
For example, a `DynamicList` could proxy a `Vec<i32>` and thereby indicate to reflection internals how it can be used.

Historically, these were referred to as "represented" or "dynamic" types, 
with terms like "concrete" or "canonical" acting as their opposites—the actual underlying Rust type.
This release introduces "comptime" and "runtime" in an effort to make the distinction clearer.
Comptime refers to the true compile-time type of a reflected value,
while runtime refers to the type a reflected value presents itself as.

As such, the following APIs have been changed to make use of the new terminology:

- `PartialReflect::get_represented_type_info` is now deprecated. Use `PartialReflect::runtime_type_info` instead.
- `DynamicTyped::reflect_type_info` is now deprecated. Use `DynamicTyped::comptime_type_info` instead.
- The `set_represented_type` method on dynamic types (e.g., `DynamicStruct`, `DynamicTuple`, etc.) is now deprecated. Use `set_runtime_type` instead.
- The `get_represented_***_info` method on the subtraits (e.g., `Struct`, `Tuple`, etc.) is now deprecated. Use `runtime_***_info` instead. 

Additional APIs have been added to provide access to both comptime and runtime type information.