---
title: Combine now takes an extra parameter
pull_requests: [20689]
---

The `Combine::combine` method now takes an extra parameter that needs to be passed mutably to the two given closures. This allows fixing a soundness issue which manifested when the two closures were called re-entrantly.
