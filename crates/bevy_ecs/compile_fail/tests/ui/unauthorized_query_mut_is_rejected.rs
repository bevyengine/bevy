use bevy_ecs::prelude::*;

#[derive(RestrictedAccess)]
struct Restricted;

fn unauthorized_query(mut query: Query<&mut Restricted>) {
    //~^ E0271
    for mut restricted in query.iter_mut() {
        //~^ E0599
        let _ = &mut *restricted;
    }
}

fn main() {}
