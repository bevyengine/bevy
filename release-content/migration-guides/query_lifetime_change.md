---
title: Query lifetime change
pull_requests: [18860]
---

Query lifetimes have been change if you are deriving `SystemParam` with `Query` you will need to update.

```diff
#[derive(SystemParam)]
-- struct MyParam<'w, 's> {
--     query: Query<'w, 's, Entity>
-- }
++ struct MyParam<'w> {
++     query: Query<'w, 'w, Entity>
++ }

```
