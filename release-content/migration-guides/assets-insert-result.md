---
title: "`Assets::insert` and `Assets::get_or_insert_with` now return `Result`"
pull_requests: [20439]
---

In previous versions of Bevy, there was a bug where inserting an asset into an `AssetId` whose handle was dropped would result in a panic. Now this is an error! Calling `Assets::insert` and
`Assets::get_or_insert_with` returns an error you can inspect.

To match the previous behavior, just `unwrap()` the result.
