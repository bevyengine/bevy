---
title: Remove Bundle::register_required_components
pull_requests: [19967]
---

This method was effectively dead-code as it was never used by the ECS to compute required components, hence it was removed. Please open an issue if you were using it in any way.
