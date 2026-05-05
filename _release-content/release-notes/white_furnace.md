---
title: White furnace test
authors: ["@dylansechet"]
pull_requests: [23194, 23203]
---
The [white furnace test](https://lousodrome.net/blog/light/2023/10/21/the-white-furnace-test/) is a classic sanity check for physically-based renderers. Place a perfectly reflective object inside a uniform white environment, and it should be indistinguishable from the background, no matter how metallic and rough. Any object that remains visible is a sign that the shader is creating or absorbing energy it shouldn't.

Bevy used to fail this test, meaning something was wrong with our shader math. Two bugs were responsible:

- Seams were visible when using `GeneratedEnvironmentMapLight` for certain surface orientations.
- Partially metallic materials absorbed energy, appearing darker than they should be.

After fixing those, Bevy passes the test. That means your materials will behave more correctly under image-based lighting.

A gray image has never been so exciting!
