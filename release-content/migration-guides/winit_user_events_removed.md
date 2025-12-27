---
title: Winit user events removed
pull_requests: [22088]
---

In Bevy 0.17 and earlier, `WinitPlugin` and `EventLoopProxyWrapper` was generic over a `M: Message` type, that could be used to wake up the winit event loop and which was then forwarded to the ECS world. In 0.18 support for this has been removed, and those types are no longer generic.

If you used the default `WakeUp` type via the event loop proxy, you can still do this by using the new `WinitUserEvent` type:

```rust
// 0.17
fn wakeup_system(event_loop_proxy: Res<EventLoopProxyWrapper<WakeUp>>) -> Result {
    event_loop_proxy.send_event(WakeUp)?;

    Ok(())
}

// 0.18
fn wakeup_system(event_loop_proxy: Res<EventLoopProxyWrapper>) -> Result {
    event_loop_proxy.send_event(WinitUserEvent::WakeUp)?;

    Ok(())
}
```

If you were using it to send information into the ECS world from outside Bevy, you will need to create your own channel and system that forwards the messages.
