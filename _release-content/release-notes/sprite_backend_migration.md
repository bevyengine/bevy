---
title: Sprite render backend migration
authors: ["@IceSentry"]
pull_requests: [25432]
---

The sprite render backend was replaced by a new backend that reuses a lot of the infrastructure made for 3d.
This resulted in improved performance in many cases and also makes future maintenance and improvements easier.
