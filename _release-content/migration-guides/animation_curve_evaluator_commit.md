---
title: Change function signature of commit in AnimationCurveEvaluator
pull_requests: [24201]
---

The root motion feature needs to access the animated entity after the curves are applied. For this reason, the commit method of
AnimationCurveEvaluator changed from :

```rust
    fn commit(&mut self, entity: AnimationEntityMut) -> Result<(), AnimationEvaluationError>;
```

to

```rust
    fn commit(&mut self, entity: &mut AnimationEntityMut) -> Result<(), AnimationEvaluationError>;
```

Passing the AnimationEntityMut by mutable reference is now necessary.
