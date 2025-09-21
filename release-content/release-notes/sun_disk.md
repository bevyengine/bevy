---
title: Procedural Sun Disk
authors: ["@defuz"]
pull_requests: [20434]
---

TODO: grab images from 20434 PR description.

Any good [procedural atmosphere] deserves a procedural sun to light it.
To enable this, add the [`SunDisk`] component to your [`DirectionalLight`] entity.
The sun will move with your light, playing nicely with any positioning or movement logic you've implemented.

You can set both the `angular_size` and `intensity` of the sun disk, changing the size and brightness of the sun.
We've included a convenient `SunDisk::EARTH` constant, to spare you the tricky experimental trigonometry.

If you've ever stared directly at the sun in real life (don't!), you'll also be familiar with a spreading glow
that bleeds out into the nearby sky.
In rendering, this achieved through a post-processing effect is known as "bloom", and is enabled by adding the [`Bloom`] component to your camera entity.

[procedural atmosphere]: https://bevy.org/news/bevy-0-16/#procedural-atmospheric-scattering
[`SunDisk`]: https://docs.rs/bevy/0.17.0-rc.1/bevy/light/struct.SunDisk.html
[`Bloom`]: https://docs.rs/bevy/0.17.0-rc.1/bevy/post_process/bloom/struct.Bloom.html
