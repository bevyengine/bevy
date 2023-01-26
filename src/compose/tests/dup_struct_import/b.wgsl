#define_import_path b
#import struct

fn b() -> struct::MyStruct {
    var s: struct::MyStruct;
    s.value = 2.0;
    return s;
}