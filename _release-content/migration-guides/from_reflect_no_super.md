---
title: Reflect is no longer a super trait of FromReflect.
pull_requests: [26249]
---

Previously, `Reflect` was a super trait of `FromReflect`. Now, this is no longer the case. This
means trait bounds that used to only require `T: FromReflect` may *also* need to add `Reflect` like
`T: FromReflect + Reflect`.

In many cases however, this additional trait bound is **not** needed. Check if you actually depend
on any `Reflect` methods, and consider tightening trait bounds where possible (e.g., only add a
`PartialReflect` bound if you actually use `PartialReflect` methods).
