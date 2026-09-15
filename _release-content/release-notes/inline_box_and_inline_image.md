---
title: "InlineBox and InlineImage"
authors: ["@Ickshonpe"]
pull_requests: [25710]
---

`InlineBox` is a new component added to `bevy_text` that allows space to be reserved within text layouts for custom content. Like `TextSpan`, an `InlineBox` entity is only valid when it's a descendant of a root `Text` or `Text2d` entity. `InlineBox` only reserves space, after layout `TextLayoutInfo::inline_boxes` contains the list of boxes and it's left to the user to draw its content. An inline box can be either `InFlow` or `OutOfFlow`. `InFlow` boxes take up space and flow with the surrounding text. `OutOfFlow` boxes are given a position as if they are zero-sized and they don't displace any text.

`InlineImage` is a new component added to `bevy_ui` that uses `InlineBox` to display images interspersed with text. It takes a color and an image asset handle, the size of its inline box is determined from the size of the image asset.
