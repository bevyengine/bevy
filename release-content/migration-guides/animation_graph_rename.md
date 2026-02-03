---
title: Rename `AnimationGraph` to `BlendGraph`
pull_requests: [22782]
---

The `AnimationGraph` and it's related structs/traits have been renamed to use the `Blend` prefix :

- `AnimationGraph` has been renamed to `BlendGraph`.
- `ThreadedAnimationGraph` has been renamed to `ThreadedBlendGraph`.
- `AnimationGraphNode` has been renamed to `BlendGraphNode`.
- `AnimationGraphNodeType` has been renamed to `BlendGraphNodeType`.
- `AnimationGraphHandle` has been renamed to `BlendGraphHandle`.
- `AnimationNodeIndex` has been renamed to `BlendNodeIndex`.
- `AnimationGraphAssetLoader` has been renamed to `BlendGraphAssetLoader`.
- `SerializedAnimationGraph` has been renamed to `SerializedBlendGraph`.
- `SerializedAnimationGraphNode` has been renamed to `SerializedBlendGraphNode`.
- `SerializedAnimationGraphNodeType` has been renamed to `SerializedBlendGraphNodeType`.
- `AnimationGraphSaveError` has been renamed to `BlendGraphSaveError`.
- `AnimationGraphLoadError` has been renamed to `BlendGraphLoadError`.
- `Animatable` has been renamed to `Blendable`.
- `AnimatableCurve` has been renamed to `BlendableCurve`.
- `AnimatableProperty` has been renamed to `BlendableProperty`.
- `AnimatableCurveEvaluator` has been renamed to `BlendableCurveEvaluator`.
- `AnimatableKeyframeCurve` has been renamed to `BlendableKeyframeCurve`.
- File extensions `.animgraph` and `.animgraph.ron` have been renamed `.blendgraph` and `.blendgraph.ron`.
- `animatable.rs` in `bevy_animation` has been renamed `blendable.rs.`
