---
title: The `AssetReader` trait can now (optionally) support seeking any direction.
authors: ["@andriyDev", "@cart"]
pull_requests: [22182]
---

_TODO: This release note is not up to date with the changes in [#22182](https://github.com/bevyengine/bevy/pull/22182)._

In Bevy 0.15, we replaced the `AsyncSeek` super trait on `Reader` with `AsyncSeekForward`. This
allowed our `Reader` trait to apply to more cases (e.g., it could allow cases like an HTTP request,
which may not support seeking backwards). However, it also meant that we could no longer use seeking
fully where it was available.

To resolve this issue, we now allow `AssetLoader`s to provide a `ReaderRequiredFeatures` to the
`AssetReader`. The `AssetReader` can then choose how to handle those required features. For example,
it can return an error to indicate that the feature is not supported, or it can choose to use a
different `Reader` implementation to fallback in order to continue to support the feature.

This allowed us to bring back the "requirement" the `Reader: AsyncSeek`, but with a more relaxed
policy: the `Reader` may choose to avoid supporting certain features (corresponding to fields in
`ReaderRequiredFeatures`).

Our general recommendation is that if your `Reader` implementation does not support a feature, make
your `AssetReader` just return an error for that feature. Usually, an `AssetLoader` can implement a
fallback itself (e.g., reading all the data into memory and then loading from that), and loaders can
be selected using `.meta` files (allowing for fine-grained opt-in in these cases). However if there
is some reasonable implementation you can provide (even if not optimal), feel free to provide one!
