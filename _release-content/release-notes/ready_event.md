---
title: "Ready Event"
authors: ["@cart"]
pull_requests: []
---

We landed BSN, Bevy's next generation scene system, [in our last release](/news/bevy-0-19). It was missing a key piece though: the ability to easily run logic when a scene is fully "ready" and spawned (ex: all dependencies have loaded, the full hierarchy is present, and all of the initial components are inserted in the scene). This is a critical piece for building cohesive, standalone, composable scenes. It is also necessary to properly layer Bevy logic on top of _other_ scene representations (like glTF).

The closest we had was the `Add` event for a given component, which runs "top down" (meaning children are not available). We needed a "bottom up" equivalent to enable building logic that relies on the complete loaded and spawned scene.

The solution is pretty straightforward: trigger a new `Ready` event for each entity in a spawned scene _after_ the full spawn logic has run for that entity (including its descendants).

This enables the following:

```rust
#[derive(SceneComponent, Default, Clone)]
struct Widget;

impl Widget {
    fn scene() -> impl Scene {
        bsn! {
            Node { width: px(100), height: px(100) }
            @on(|ready: On<Ready>| {
                info!("The full scene, including 'widget.bsn' contents, is available here")
            })
            Children [
                Text("hello"),
                :"widget.bsn"
            ]
        }
    }
}

world.spawn(bsn!{ @Widget })
```
