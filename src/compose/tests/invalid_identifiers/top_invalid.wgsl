// #import consts
// #import fns
// #import globals
#import struct_members
// #import structs

fn main() -> f32 {
    // let a = consts::fine + consts::bad_;
    // let b = fns::fine(1.0) + fns::bad_(2.0);
    // let c = globals::fine + globals::bad_;
    // var d: structs::IsFine;
    // d.fine = 3.0;
    // var e: structs::Isbad_;
    // e.fine_member = 4.0;
    var f = struct_members::FineStruct;
    f.fine = 5.0;
    var g = struct_members::BadStruct;
    g.also_fine = 6.0;
    g.bad_ = 7.0;

    // return a + b + c + d.fine + e.fine_member; 
    return f.fine + g.also_fine + g.bad_;
}