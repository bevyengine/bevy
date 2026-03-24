---
title: "`RenderSystems::ManageViews` has been split into three system sets"
pull_requests: [22949]
---

`ManageViews` was previously somewhat overloaded with responsibility, and made resolving render system order ambiguities difficult.
To amend this, it has been split into three phases: `CreateViews`, `Specialize`, and `PrepareViews`.
It is very likely whatever you were ordering against `ManageViews` can now be ordered against `PrepareViews` and have identical behavior.
If you are creating additional views, for example for cubemap rendering, please do so in `CreateViews`.
