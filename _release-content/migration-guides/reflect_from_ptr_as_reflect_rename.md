---
title: ReflectFromPtr::as_reflect is renamed to ptr_as_reflect
pull_requests: [25754]
---

`ReflectFromPtr::as_reflect` and `ReflectFromPtr::as_reflect_mut` have been replaced with versions
that accept `&dyn Any`. The previous methods still exist, renamed to `ptr_as_reflect` and
`ptr_as_reflect_mut`, respectively.
