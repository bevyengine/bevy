---
title: "Support for using different time resources for different animations"
authors: ["@Leinnan"]
pull_requests: [20717]
---

New helper plugin `TimeDependentAnimationPlugin` is added to make it possible to use different time resources for different animations. This allows developers to create more complex animations that can be controlled by different time resources, such as fixed time or real time. The functions has two generic parameters, first specify the Time generic type to use, second specify the FilterQuery for the animation components that the system will process with given time resource.
