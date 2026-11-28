---
title: "`Image::from_buffer` now uses `ImageBufferInfo`"
pull_requests: [26701]
---

`Image::from_buffer` now uses `ImageBufferInfo` to group its configuration settings instead of passing them as separate arguments. The `from_buffer_inner` function has also been removed.
