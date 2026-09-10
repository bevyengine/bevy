---
title: "`NextState::set_if_neq` renamed to `set_if_different`"
pull_requests: [24676]
---

`NextState::set_if_neq` and related methods and enum variants have been renamed to avoid naming conflicts with `Mut::set_if_neq` / `ReflectMut::set_if_neq` which have a different meaning.

The following names have changed:

- `NextState::set_if_neq` is now `NextState::set_if_different`
- `NextState::PendingIfNeq` is now `NextState::PendingIfDifferent`
- `CommandsStatesExt::set_state_if_neq` is now `CommandsStatesExt::set_state_if_different`
- `ReflectFreelyMutableState::set_next_state_if_neq` is now `ReflectFreelyMutableState::set_next_state_if_different`
- `ReflectFreelyMutableStateFns::set_next_state_if_neq` is now `ReflectFreelyMutableStateFns::set_next_state_if_different`

Deprecated compatibility wrappers have been added for the renamed methods (`NextState::set_if_neq`, `CommandsStatesExt::set_state_if_neq`, and `ReflectFreelyMutableState::set_next_state_if_neq`) to allow existing code to compile with deprecation warnings.
