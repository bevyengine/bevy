---
title: Interned labels cleanup
pull_requests: [18984]
---

- `DynEq::as_any` has been removed. Use `&value as &dyn Any` instead.
- `DynHash::as_dyn_eq` has been removed. Use `&value as &dyn DynEq` instead.
- `as_dyn_eq` has been removed from 'label' types such as `ScheduleLabel` and `SystemSet`. Call `DynEq::dyn_eq` directly on the label instead.
