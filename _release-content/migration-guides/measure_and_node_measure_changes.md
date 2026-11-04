---
title: "`Measure` and `NodeMeasure` changes"
pull_requests: [25653]
---

Previously, `NodeMeasure`s were moved out of the `ContentSize` components during layout. Now they are accessed directly by querying for `Ref<ContentSize>`.

As a result, the receiver for the `Measure` trait and its implementation for `NodeMeasure` no longer needs to be mutable, instead `&self` is sufficient.
