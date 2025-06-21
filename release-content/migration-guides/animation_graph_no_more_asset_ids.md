---
title: `AnimationGraph` no longer supports raw AssetIds.
pull_requests: []
---

In previous versions of Bevy, `AnimationGraph` would serialize `Handle<AnimationClip>` as an asset
path, and if that wasn't available it would fallback to serializing `AssetId<AnimationClip>`. In
practice, this was not very useful. `AssetId` is (usually) a runtime-generated ID. This means for an
arbitrary `Handle<AnimationClip>`, it was incredibly unlikely that your handle before serialization
would correspond to the same asset as after serialization.

This confusing behavior has been removed. As a side-effect, any `AnimationGraph`s you previously
saved (via `AnimationGraph::save`) will need to be re-saved. These legacy `AnimationGraph`s can
still be loaded until the next Bevy version. Loading and then saving the `AnimationGraph` again will
automatically migrate the `AnimationGraph`.

If your `AnimationGraph` contained serialized `AssetId`s, you will need to manually load the bytes
of the saved graph, deserialize it into `SerializedAnimationGraph`, and then manually decide how to
migrate those `AssetId`s. Alternatively, you could simply rebuild the graph from scratch and save a
new instance. We expect this to be a very rare situation.
