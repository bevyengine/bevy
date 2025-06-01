---
title: Renamed `Condition` to `SystemCondition`
pull_requests: [19328]
---

`Condition` is now `SystemCondition`. Replace all references and imports.

This change was made because `Condition` is an overly generic name that collides too often and is rarely used directly, despite appearing in the prelude.
