---
title: Remove Bundle::register_required_components
pull_requests: [19967]
---

This method was effectively dead-code as it was never used by the ECS to compute required components, hence it was removed. if you were overriding its implementation you can just remove it, as it never did anything. If you were using it in any other way, please open an issue.
