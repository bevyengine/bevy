use bevy_async::*;
use bevy_ecs::prelude::*;

fn main() {
    let _ = co!(async move |state: Local<usize>| loop {
        // system params are not accessible outside co_with!
        &*state;

        // system params cannot escape co_with!
        co_with!(|state| state);
        let mut oops = None;
        co_with!(|state| oops = Some(state));

        // a normal reference would be UB to hold over an await point
    });
}
