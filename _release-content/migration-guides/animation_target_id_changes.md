---
title: "`AnimationTargetId` algorithm changes"
pull_requests: [22876]
---

The algorithm used to calculate `AnimationTargetId` has changed. This fixes a
[bug](https://github.com/bevyengine/bevy/issues/22842) where different joint
hierarchies could mistakenly be assigned the same id.

If you have serialized data containing `AnimationTargetId` values then these
will need to be recalculated.
