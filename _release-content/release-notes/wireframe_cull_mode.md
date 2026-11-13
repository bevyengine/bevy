---
title: Wireframe culling is configurable
authors: ["@tychedelia"]
pull_requests: []
---

`WireframeConfig` has a new `cull_mode` field. The wireframe pass renders a mesh's own
triangles and draws their edges, so until now it inherited the mesh pipeline's back-face
culling: a cube with no fill showed only the edges of its three front faces, and in the wide
line mode the expanded edge quads of back faces disappeared with them. Set `cull_mode` to
`None` to draw the edges of every face, which is the classic wireframe look. The default,
`Some(Face::Back)`, keeps the previous behavior. X-ray mode already draws both sides and is
unaffected.
