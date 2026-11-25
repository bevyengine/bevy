---
title: "`FontSize` is no longer `Copy`"
pull_requests: [26658]
---

`FontSize` no longer implements `Copy`. Replace implicit copies with `.clone()`. 
