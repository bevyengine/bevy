---
title: QueryFilter has a new filter_table method
pull_requests: [25946]
---

The `QueryFilter` trait now has a new required method `filter_table` which can be used to short-circuit a non-archetypal filter for whole tables. The method can be implemented to return `true` in order to opt-out of the optimization and keep the existing behavior.
