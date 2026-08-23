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
