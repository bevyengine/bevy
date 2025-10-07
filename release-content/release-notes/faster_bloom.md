---
title: Add a faster bloom implementation.
authors: ["@beicause"]
pull_requests: [21340]
---

Bloom now has a `high_quality` (default: true) option to control whether to use a high quality implementation, or a faster but lower quality implementation. The lower quality bloom still maintains reasonable visual quality while significantly reducing texture sampling. For low-end devices, this could potentially reduce frame time by a few milliseconds.
