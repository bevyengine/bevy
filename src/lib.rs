// todo

// prune
// check if you can ImageStore to a function argument - pretty sure not
// barriers - we currently don't make the containing scope required on encountering a barrier. this doesn't feel right since a nested barrier could be ignored?
// atomics

// compose
// use more regexes..?
// generate headers on demand
// check mobile - does everybody use wgsl?
// support glsl compute
// *    purge/replace modules should invalidate dependents
// *    search/replace decorated strings in error reports
// *    use better encoding for decorate
// *    don't allow modules containing decoration

// derive
// better api for entry points

pub mod compose;
pub mod derive;
pub mod prune;
pub mod util;
