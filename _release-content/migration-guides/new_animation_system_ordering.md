---
title: New animation system ordering
pull_requests: [26547]
---

`bevy_animation` now advances transitions and animations in `PreUpdate`.
`bevy_animation` also inserts a new `AnimationApply` schedule before `PostUpdate`, that is dedicated
catch up animation state changes and put them in action.

- PreUpdate // <--- Advance here
- ...
- Update
- ...
- AnimationApply // <--- Apply here
- PostUpdate

It is important to mention that the `bevy_animation` Plugin adds this new schedule.
This is important to know if you want to place a schedule relative to `PostUpdate` or
`AnimationApply`.

## Reason for advancing transitions and animations in `PreUpdate`

If you use for example `AnimationPlayer::start` in `Update` the `AnimationPlayer` is updated with
the delta time in that same frame and the first frame is the next frame.
Advancing in `PreUpdate`, you can also check the progress of animations in for example the `Update`
schedule the frame they are presented.

## Reason for AnimationApply

Animations can change any animatable component and can run any event fn code. And that are changes
one would do in the `Update` schedule, because many components need to be done updating before they
are processed in the `PostUpdate` schedule. Because all animation systems run in `PostUpdate` this
leads to scheduling conflicts that are hard to resolve. If resolved, they are too annoying to
maintain or it is to easy to create the next scheduling order bug. So they have to run after
`SpawnScene` because an animation might init an entities state or so and before `PostUpdate` because
many systems expect components do be done updating in there. With that ordering systems are not
forced into PostUpdate, even if you need them run before.

## TODO

You should keep in mind that animations are advanced in `PreUpdate` so for example
`AnimationPlayer::all_finished` is true in for example the `Update` schedule before the state is
applied and rendered. If you want to take aktion after all `ActiveAnimation`s are finished and their
state is applied and rendered. You can do this in the `Update` schedule with the new
`AnimationPlayer::all_finished_and_applied` and `ActiveAnimation::is_finished_and_applied`
functions. Any system that reads state from `AnimationTransitions` and `AnimationPlayer`s should
consider that their state is advanced in `PreUpdate`. The systems that to this are in the
`AnimationAdvanceSystems` set.

You should also keep in mind that animations are applied in `AnimationApply` that is inserted before
`PostUpdate`. This means if you want to access the current state of animated components you should
do this in the `AnimationApply` schedule after `AnimationApplySystems::Cleanup`, in any schedule
that runs after `AnimationApply`, or in the next frame with functions like
`AnimationPlayer::all_finished_and_applied` or `ActiveAnimation::is_finished_and_applied`;
