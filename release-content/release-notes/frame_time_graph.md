---
title: Frame Time Graph
authors: ["@IceSentry", "@Zeophlite"]
pull_requests: [12561, 19277]
---

(TODO: Embed frame time graph gif from 12561)

When measuring a game's performance, just seeing a number is often not enough. Seeing a graph that shows the history makes it easier to reason about performance. **Bevy 0.17** introduces a new visual "frame time graph" to solve this problem!

To display the frame time graph, enable the `bevy_dev_tools` cargo feature and add in `FpsOverlayPlugin`:

This displays "frame time" not "frames per second", so a longer frame time results in a bigger and wider bar. The color also scales with that frame time. Red is at or bellow the minimum target fps and green is at or above the target maximum frame rate.
Anything between those 2 values will be interpolated between green and red based on the frame time.

The algorithm is highly inspired by [Adam Sawicki's article on visualizing frame times](https://asawicki.info/news_1758_an_idea_for_visualization_of_frame_times).
