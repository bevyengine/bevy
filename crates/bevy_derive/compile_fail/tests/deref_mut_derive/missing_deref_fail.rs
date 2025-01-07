// I'd love to check for E0277 errors here but we can't because
// the diagnostic contains a path to the system libraries which
// isn't consistent across systems.

use bevy_derive::DerefMut;

#[derive(DerefMut)]
//~^ ERROR: trait bound
struct TupleStruct(usize, #[deref] String);
//~^ ERROR: trait bound

#[derive(DerefMut)]
//~^ ERROR: trait bound
struct Struct {
    //~^ ERROR: trait bound
    foo: usize,
    #[deref]
    bar: String,
}
