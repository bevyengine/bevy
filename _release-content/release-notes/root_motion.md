---
title: Root Motion
authors: ["@Hilpogar", "@emberlightstudios"]
pull_requests: [24201]
---

In animation and game development, root motion is a technique where the movement of a character is driven directly by the animation’s root bone instead of being controlled purely by code or physics.
It is used to create more natural, accurate movements such as walking, climbing, or attacks, by synchronizing the character’s in-game position with the animation itself.

You can now use this technique by using the `set_root_motion_target` method in the `AnimationPlayer`. To do so, you need to get the `AnimationTargetId` of your model's root motion bone.
It can be the root bone or any bone you want. When a bone is configured to be used for root motion, its position and / or rotation will be erased each frame and the delta with the previous
frame is stored in the `RootMotion` component inside the `AnimationPlayer`'s entity. You can configure if the root motion should extract translation and rotation or just translation with the `set_root_motion_mode` method in `AnimationPlayer`.
