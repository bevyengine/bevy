use bevy_ecs::{bundle::Bundle, observer::Trigger};
use bevy_ecs::system::In;
use bevy_ecs::{event::Event, system::IntoObserverSystem};

#[derive(Debug, Event)]
struct MyEvent;

fn observer(_: In<Trigger<'static, MyEvent, ()>>) {}

pub fn is_observer<E: Event, B: Bundle, M>(_: impl IntoObserverSystem<E, B, M>) {}

fn main() {
    is_observer(observer);
}
