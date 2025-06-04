---
title: Adding duplicate subassets names now returns an error.
pull_requests: [19485]
---

Previously, when adding a subasset in a loader through `LoadContext::add_labeled_asset` (and
friends), adding the same subasset name multiple times would silently replace the asset (and which
one would be used was undefined). Now these functions return an error if the subasset label was
already present. You can use `?` to return the error from `AssetLoader`s.
