---
title: LoadContext::read_asset_bytes replaced with LoadContext::read_asset.
pull_requests: [26092]
---

Previously, `LoadContext::read_asset_bytes` would read the **whole** asset into memory before
handing it to you. This has been deprecated in favor of `LoadContext::read_asset`, which returns you
the `Reader` to allow you to read the bytes as needed (to avoid loading the whole asset into memory
unless you want to).
