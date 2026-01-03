---
title: AssetSources now give an `async_channel::Sender` instead of a `crossbeam_channel::Sender`
pull_requests: [21626]
---

Previously, when creating an asset source, `AssetSourceBuilder::with_watcher` would provide users
with a `crossbeam_channel::Sender`. Now, this has been changed to `async_channel::Sender`.

If you were previously calling `sender.send(AssetSourceEvent::ModifiedAsset("hello".into()))`, now
it would be `sender.send_blocking(AssetSourceEvent::ModifiedAsset("hello".into()))`. These channels
are very comparable, so finding an analogous method between `crossbeam_channel` and `async_channel`
should be straight forward.
