---
title: Changes to type registration for reflection
pull_requests: [15030, 20435, 20893]
---

Calling `.register_type` has long been a nuisance for Bevy users: both library authors and end users.
This step was previously required in order to register reflected type information in the `TypeRegistry`.

In Bevy 0.17 however, types which implement `Reflect` are now automatically registered, with the help of some compiler magic.
You should be able to remove almost all of your `register_type` calls.
This comes with a few caveats however:

1. This functionality is gated by feature flags. Projects with default features off will not see reflected types unless one of these features is enabled.
2. There are two approaches to do this: one has incomplete platform support, while the other relies on a specific project structure.
3. Generic types are not automatically registered, and must still be manually registered.

In order for Bevy to automatically register your types, you need to turn on the `reflect_auto_register` feature, or the fallback `reflect_auto_register_static`.
The `reflect_auto_register` feature is part of Bevy's default features, and can be overridden by the `reflect_auto_register_static` feature flag.
Be aware that the `reflect_auto_register_static` feature comes with some caveats for project structure: check the docs for [load_type_registrations!](https://docs.rs/bevy/0.17.0-rc.1/bevy/reflect/macro.load_type_registrations.html) and follow the [`auto_register_static` example](https://github.com/bevyengine/bevy/tree/main/examples/reflection/auto_register_static).

We recommend:

1. Enable `reflect_auto_register` in your application code, CI and in examples/tests. You can enable `bevy` features for tests only by adding a matching copy of `bevy` to `dev-dependencies` with the needed features enabled.
   1. Most libraries and some production applications do not need this functionality. This feature flag (like all reflection) is most useful for dev tools, although some workflows enable it during production as well. Reflection can seriously increase both compile times and binary sizes, so enabling reflection in your library or project should be a deliberate choice.
2. Do not enable the `reflect_auto_register` feature, or the fallback `reflect_auto_register_static`, in your library code.
3. As a library author, you can safely remove all non-generic `.register_type` calls.
4. As a user, if you run into an unregistered generic type with the correct feature enabled, file a bug with the project that defined the offending type, and workaround it by calling `.register_type` manually.
5. If you are on an unsupported platform but need reflection support, try the `reflect_autoregister_static` feature, and consider working upstream to add support for your platform in `inventory`. As a last resort, you can still manually register all of the needed types in your application code.
