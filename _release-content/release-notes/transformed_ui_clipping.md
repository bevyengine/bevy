---
title: "New CPU UI clipping implementation supporting rotation"
authors: ["@Ickshonpe"]
pull_requests: [24148]
---

UI overflow clipping now supports rotated clipping regions. Descendants are clipped against each of the clipping regions inherited from their ancestors, instead of against a single axis-aligned world-space rectangle.
