---
title: AssetPlugin now has a `use_asset_processor_override` field.
pull_requests: [21409]
---

The `AssetPlugin` now has a `use_asset_processor_override` field. If you were previously setting all
`AssetPlugin` fields, you should either use struct update syntax `..Default::default()`, or set the
new `use_asset_processor_override` to `None`.
