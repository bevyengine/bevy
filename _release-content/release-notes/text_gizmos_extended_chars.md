---
title: Text gizmos extended characters
authors: ["@nuts-rice"]
pull_requests: [ 24003 ]
---

## Goals

Adds support for more characters outside of ASCII.
Currently renders accented Latin characters.
Uses separate arrays for extended glyph data.
Accented characters use un-accented letters
as base and adds strokes and positions for accent marks.
