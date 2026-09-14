---
title: "Exclusive function systems have been unified with regular function systems"
pull_requests: [25507]
---

`ExclusiveFunctionSystem`, `ExclusiveSystemParam`, and `ExclusiveSystemParamFunction`
have been removed. Exclusive function systems now use the same code path as
regular function systems: `FunctionSystem`, `SystemParam`, and `SystemParamFunction`.

`&mut World` now implements `SystemParam`, which means it is no longer required
to be the first parameter of a function system. Instead, it may appear anywhere
in the parameter list, so long as it does not conflict with other system parameters
(same as before). Generally speaking, this means systems' "exclusivity" is now
tracked at runtime, rather than compile time.

All `ExclusiveSystemParam`s that did not previously implement `SystemParam` now
implement it, including `&mut QueryState` and `&mut SystemState`.

It is no longer possible to use `WorldId` in an exclusive system,
since the `SystemParam` implementation needs to read it from the `World`.
If you were using `WorldId` in an exclusive system,
consider calling `World::id()` as the first line of the system.
Alternately, use `Local<WorldId>` as a parameter,
which will be automatically populated with the correct ID.

`System::is_exclusive()` has been removed. Use `SystemAccess::is_exclusive()` instead,
which is created by `System::initialize()` and stored in `SystemWithAccess`.

`System::initialize()` now returns a `SystemAccess` rather than a `FilteredAccessSet`.
`SystemAccess` is a superset of `FilteredAccessSet` that also tracks whether the
system is exclusive, or requires no access to the world at all. If you require
access to a `FilteredAccessSet`, call `SystemAccess::require_shared_access(system_meta)`.

`ExclusiveMarker` has been removed. If you need to mark a system as exclusive,
consider piping the system into a `fn(&mut World)` system, which will automatically
mark it as exclusive.
