---
title: "New CPU UI clipping implementation supporting rotation"
authors: ["@Ickshonpe"]
pull_requests: [24148]
---

Until now, Bevy ignored rotation and scaling when clipping overflow, causing content to be distorted or clipped in the wrong place.UI overflow clipping now supports rotated clipping regions, using a simplified version of the Sutherland–Hodgman algorithm.Descendants are clipped against each of the clipping regions inherited from their ancestors, instead of against a single axis aligned rectangle. The overflow API itself is unchanged.
