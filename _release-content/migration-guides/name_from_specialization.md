---
title: "`&str`s must now have a static lifetime to be converted to `Name`"
pull_requests: [24544]
---

A `From<&str>` implementation for `Name` has been replaced with a `From<&'static str>` implementation for `Name`. This was done to avoid unexpected allocations.

If you do not mind the extra allocation, you can use `Name::new(non_static_str.to_owned())` for previous behavior.
