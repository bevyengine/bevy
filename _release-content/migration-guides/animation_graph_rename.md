---
title: Rename `AnimationGraph` to `BlendGraph`
pull_requests: [26367]
---

The `AnimationGraph` and its related structs/traits have been renamed to use the `Blend` prefix:

- `AnimationGraph` has been renamed to `BlendGraph`.
- `ThreadedAnimationGraph` has been renamed to `ThreadedBlendGraph`.
- `ThreadedAnimationGraphs` has been renamed to `ThreadedBlendGraphs`.
- `ThreadedAnimationSubgraph` has been renamed to `ThreadedBlendSubgraph`.
- `AnimationDiGraph` has been renamed to `BlendDiGraph`.
- `AnimationGraphNode` has been renamed to `BlendGraphNode`.
- `AnimationNodeType` has been renamed to `BlendNodeType`.
- `AnimationGraphHandle` has been renamed to `BlendGraphHandle`.
- `AnimationNodeIndex` has been renamed to `BlendNodeIndex`.
- `AnimationGraphAssetLoader` has been renamed to `BlendGraphAssetLoader`.
- `SerializedAnimationGraph` has been renamed to `SerializedBlendGraph`.
- `SerializedAnimationGraphNode` has been renamed to `SerializedBlendGraphNode`.
- `SerializedAnimationNodeType` has been renamed to `SerializedBlendNodeType`.
- `AnimationGraphSaveError` has been renamed to `BlendGraphSaveError`.
- `AnimationGraphLoadError` has been renamed to `BlendGraphLoadError`.
- `Animatable` has been renamed to `Blendable`.
- `AnimatableCurve` has been renamed to `BlendableCurve`.
- `AnimatableProperty` has been renamed to `BlendableProperty`.
- `AnimatableCurveEvaluator` has been renamed to `BlendableCurveEvaluator`.
- `AnimatableKeyframeCurve` has been renamed to `BlendableKeyframeCurve`.
- File extensions `.animgraph` and `.animgraph.ron` have been renamed `.blendgraph` and `.blendgraph.ron`.
- The `bevy_animation::animatable` module has been renamed to `bevy_animation::blendable`.
