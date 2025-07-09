---
title: Frame Time Graph
authors: ["@IceSentry", "@Zeophlite"]
pull_requests: [12561, 19277]
---

(TODO: Embed frame time graph gif from 12561)

Frame time is often more important to know than FPS but because of the temporal nature of it, just seeing a number is not enough.
Seeing a graph that shows the history makes it easier to reason about performance.

Enable the `bevy_dev_tools` feature, and add in `FpsOverlayPlugin` to add a bar graph of the frame time history.
Each bar is scaled based on the frame time where a bigger frame time will give a taller and wider bar.

The color also scales with that frame time where red is at or bellow the minimum target fps and green is at or above the target maximum frame rate.
Anything between those 2 values will be interpolated between green and red based on the frame time.

The algorithm is highly inspired by [Adam Sawicki's article on visualizing frame times](https://asawicki.info/news_1758_an_idea_for_visualization_of_frame_times).
