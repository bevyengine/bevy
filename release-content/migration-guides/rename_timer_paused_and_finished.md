---
title: Renamed `Timer::paused` to `Timer::is_paused` and `Timer::finished` to `Timer::is_finished`
pull_requests: [19386]
---

1. `Timer::paused` is now `Timer::is_paused`
2. `Timer::finished` is now `Timer::is_finished`


This change was made to align the `Timer` public API with that of `Time` and `Stopwatch`.
