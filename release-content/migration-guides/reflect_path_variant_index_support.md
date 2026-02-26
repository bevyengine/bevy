---
title: ReflectPath
pull_requests: [23137]
---

`ReflectPath` in  `bevy_reflect` now supports and enforces accessing specified enum variants under reflection.

In previous releases, access to an enum under reflection was treated the same way as accessing a struct with a similar shape.

In example:

```rust
//0.18

struct MyStruct(u32,u32);
let my_struct = MyStruct(6,5);
// String encoding access for the first field of a tuple struct.
let value_index_0 = ".0"
// String encoding access for the second field of a tuple struct.
let value_index_1 = ".1"
// We can use these for accessing a tuple struct....
let six = value_index_0.reflect_element::<u32>(my_struct).unwrap();
let five = value_index_1.reflect_element::<u32>(my_struct).unwrap();


let err_six : Result<u32> = Err(6);
let ok_five : Result<u32> = Ok(5);
// or even an enum with the shape of a tuple struct
let still_six = value_index_0.reflect_element::<u32>(err_six).unwrap();
// But we can see here that our path does not encode any difference between variants Ok and Err!
let still_five = value_index_0.reflect_element(ok_fiveanother_enum).unwrap();
```

This poses obvious soundness issues with our path!


 Now that `ReflectPath` can encode and enforce assumptions about which variant we are accessing, we can enforce a limited degree of soundness:
 
 
 ```rust
 //0.19
 
struct MyStruct(u32,u32);
let my_struct = MyStruct(6,5);
// String encoding access for the first field of a tuple struct.
let value_index_0 = ".0"
// String encoding access for the second field of a tuple struct.
let value_index_1 = ".1"
// String encoding access for the variant at index 0, and the field at index 0
let variant_index_0_value_index_0 = "{0.0}"
// String encoding access for the variant at index 1 and the field at index 0
let variant_index_1_value_index_0 = "{1.0}"
// We can use these for accessing a tuple struct....
let six = path_a.reflect_element::<u32>(my_struct).unwrap();
let five = path_b.reflect_element::<u32>(my_struct).unwrap();


let err_six : Result<u32> = Err(6);
let ok_five : Result<u32> = Ok(5);
// this now panics! 
let _  = value_index_0.reflect_element::<u32>(err_six).unwrap();
//  "Ok" is at index 1, but our path wants index 0, so this will also panic!
let  _  = variant_index_0_value_index_0.reflect_element::<u32>(ok_five).unwrap(); 
// Here we can successfully access specifically value 0 of variant 0
let safe_six = variant_index_0_value_index_0.reflect_element::<u32>(err_six).unwrap();
// Or, value 0 of variant 1
let final_five = variant_index_1_value_index_0.reflect_element::<u32>(ok_five).unwrap();
 ```


This change also changes the syntax supported by `ReflectPath` into something that is capable of statically addressing arbitrary fields on types.

Because of this ambiguity, users wishing to express structural access to a field without an instance of a type could not do so, which impaired the ergonomics and utility of abstractions built on top of `ReflectPath` in spaces such as BRP, editors, or inspectors.



Copy the contents of this file into a new file in `./migration-guides`, update the metadata, and add migration guide content here.

Remember, your aim is to communicate:

- What has changed since the last release?
- Why did we make this breaking change?
- How can users migrate their existing code?

For more specifics about style and content, see the [instructions](./migration_guides.md).
