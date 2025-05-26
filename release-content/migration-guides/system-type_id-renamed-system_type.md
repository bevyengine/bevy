---
title: ECS System type_id method renamed to system_type
pull_requests: [19374]
---

`type_id` method on `System` trait is now `system_type`. Replace all references.

This change was made so that the method names are in line with `SystemSet`.
