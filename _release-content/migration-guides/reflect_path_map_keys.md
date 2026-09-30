---
title: "`Access` and `AccessErrorKind` have new variants"
pull_requests: [25968]
---

Reflection paths can now access map and set entries by key, which adds a few enum variants.

- `Access` has a new `Key` variant for quoted keys like `["name"]`.
- `AccessErrorKind` has new `InvalidKey` and `MutableSetAccess` variants.

Exhaustive matches on these enums need to handle the new variants.
