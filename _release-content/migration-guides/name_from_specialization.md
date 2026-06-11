---
title: Feature that broke
pull_requests: [24544]
---

* Removed `From<&str>` implementation for `Name` 
* Added `From<&'static str>` implementation for `Name`

`Name` is internally `Cow<'static, str>`, meaning we need to `.to_owned()`, thus heap-allocating new space in memory. By changing the implementation to `&'static str`, we do not need to `.to_owned()` anymore.