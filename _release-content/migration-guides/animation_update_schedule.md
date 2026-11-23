---
title: New schedule `AnimationUpdate` after `SpawnScene` && before `PostUpdate`
pull_requests: [26547]
---

Bevy now has a separate `AnimationUpdate` schedule that is dedicated to process animations.
The schedule is added to the `Main` schedule and placed into the `MainScheduleOrder` like this:
- Update
- SpawnScene
- AnimationUpdate // <--- Here
- PostUpdate

Reason:
Animations can change any animatable component and can run any event fn code. And that are changes
one would do in the `Update` schedule, because many components need to be done updating before they
are processed in the `PostUpdate` schedule. Because all animation systems run in `PostUpdate` this
leads to scheduling conflicts that are hard to resolve. If resolved, they are too annoying to
maintain or it is to easy to create the next scheduling order bug. So they have to run after
`SpawnScene` because an animation might init an entities state or so and before `PostUpdate` because
many systems expect components do be done updating in there. With that ordering systems are not
forced into PostUpdate, even if you need them run before.

TODO:
You need to update the ordering of your systems that are ordered relative to `AnimationSystems` or
any system in `bevy_animation`.
