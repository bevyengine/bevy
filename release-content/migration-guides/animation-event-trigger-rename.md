---
title: "`AnimationEventTrigger::animation_player` has been renamed to `AnimationEventTrigger::target`"
pull_requests: [21593]
---

This field and its docs strongly suggested that it would point to an entity holding an `AnimationPlayer`, but that actually depends on how the event was registered.

- If you used `AnimationClip::add_event`, the field really did point to the `AnimationPlayer`
- But if you used `AnimationClip::add_event_to_target`, this field instead pointed to an `AnimationTargetId`

To make this more clear, the field was renamed to `target` and the docs surrounding it improved.
