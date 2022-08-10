// todo
// check if you can ImageStore to a function argument - pretty sure not
// barriers - we currently don't make the containing scope required on encountering a barrier. this doesn't feel right since a nested barrier could be ignored?
// atomics

pub mod compose;
pub mod prune;
pub mod util;
pub mod derive;