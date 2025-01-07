# Bevy Pointer

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_ptr.svg)](https://crates.io/crates/bevy_ptr)
[![Downloads](https://img.shields.io/crates/d/bevy_ptr.svg)](https://crates.io/crates/bevy_ptr)
[![Docs](https://docs.rs/bevy_ptr/badge.svg)](https://docs.rs/bevy_ptr/latest/bevy_ptr/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

Pointers in computer programming are objects that store a memory address. They're a fundamental building block for constructing more
complex data structures.

They're also *the* definitive source of memory safety bugs: you can dereference a invalid (null) pointer, access a pointer after the underlying
memory has been freed, and even ignore type safety and misread or mutate the underlying memory improperly.

Rust is a programming language that heavily relies on its types to enforce correctness, and by proxy, memory safety. As a result,
Rust has an entire zoo of types for working with pointers, and a graph of safe and unsafe conversions that make working with them safer.

`bevy_ptr` is a crate that attempts to bridge the gap between the full blown unsafety of `*mut ()` and the safe `&'a T`, allowing users
to choose what invariants to uphold for their pointer, with the intent to enable building progressively safer abstractions.

## How to Build a Borrow (From Scratch)

Correctly and safety converting a pointer into a valid borrow is at the core of all `unsafe` code in Rust. Looking at the documentation for
[`(*const T)::as_ref`], a pointer must satisfy *all* of the following conditions:

* The pointer must be properly aligned.
* The pointer cannot be null, even for zero sized types.
* The pointer must be within bounds of a valid allocated object (on the stack or the heap).
* The pointer must point to an initialized instance of `T`.
* The newly assigned lifetime should be valid for the value that the pointer is targeting.
* The code must enforce Rust's aliasing rules. Only one mutable borrow or arbitrarily many read-only borrows may exist to a value at any given moment
  in time, and converting from `&T` to `&mut T` is never allowed.

Note these rules aren't final and are still in flux as the Rust Project hashes out what exactly are the pointer aliasing rules, but the expectation is that the
final set of constraints are going to be a superset of this list, not a subset.

This list already is non-trivial to satisfy in isolation. Thankfully, the Rust core/standard library provides a progressive list of pointer types that help
build these safety guarantees...

## Standard Pointers

|Pointer Type       |Lifetime'ed|Mutable|Strongly Typed|Aligned|Not Null|Forbids Aliasing|Forbids Arithmetic|
|-------------------|-----------|-------|--------------|-------|--------|----------------|------------------|
|`Box<T>`           |Owned      |Yes    |Yes           |Yes    |Yes     |Yes             |Yes               |
|`&'a mut T`        |Yes        |Yes    |Yes           |Yes    |Yes     |Yes             |Yes               |
|`&'a T`            |Yes        |No     |Yes           |Yes    |Yes     |No              |Yes               |
|`&'a UnsafeCell<T>`|Yes        |Maybe  |Yes           |Yes    |Yes     |Yes             |Yes               |
|`NonNull<T>`       |No         |Yes    |Yes           |No     |Yes     |No              |No                |
|`*const T`         |No         |No     |Yes           |No     |No      |No              |No                |
|`*mut T`           |No         |Yes    |Yes           |No     |No      |No              |No                |
|`*const ()`        |No         |No     |No            |No     |No      |No              |No                |
|`*mut ()`          |No         |Yes    |No            |No     |No      |No              |No                |

`&T`, `&mut T`, and `Box<T>` are by far the most common pointer types that Rust developers will see. They're the only ones in this list that are entirely usable
without the use of `unsafe`.

`&UnsafeCell<T>` is the first step away from safety. `UnsafeCell` is the *only* way to get a mutable borrow from an immutable one in the language, so it's the
base primitive for all interior mutability in the language: `Cell<T>`, `RefCell<T>`, `Mutex<T>`, `RwLock<T>`, etc. are all built on top of
`UnsafeCell<T>`. To safety convert `&UnsafeCell<T>` into a `&T` or `&mut T`, the caller must guarantee that all simultaneous access follow Rust's aliasing rules.

`NonNull<T>` takes quite a step down from the aforementioned types. In addition to allowing aliasing, it's the first pointer type on this list to drop both
lifetimes and the alignment guarantees of borrows. Its only guarantees are that the pointer is not null and that it points to a valid instance
of type `T`. If you've ever worked with C++, `NonNull<T>` is very close to a C++ reference (`T&`).

`*const T` and `*mut T` are what most developers with a background in C or C++ would consider pointers.

`*const ()` is the bottom of this list. They're the Rust equivalent to C's `void*`.  Note that Rust doesn't formally have a concept of type that holds an arbitrary
untyped memory address. Pointing at the unit type (or some other zero-sized type) just happens to be the convention. The only way to reasonably use them is to
cast back to a typed pointer. They show up occasionally when dealing with FFI and the rare occasion where dynamic dispatch is required, but a trait is too
constraining of an interface to work with. A great example of this are the [RawWaker] APIs, where a singular trait (or set of traits) may be insufficient to capture
all usage patterns. `*mut ()` should only be used to carry the mutability of the target, and as there is no way to mutate an unknown type.

[RawWaker]: https://doc.rust-lang.org/std/task/struct.RawWaker.html

## Available in Nightly

|Pointer Type       |Lifetime'ed|Mutable|Strongly Typed|Aligned|Not Null|Forbids Aliasing|Forbids Arithmetic|
|-------------------|-----------|-------|--------------|-------|--------|----------------|------------------|
|`Unique<T>`        |Owned      |Yes    |Yes           |Yes    |Yes     |Yes             |Yes               |
|`Shared<T>`        |Owned*     |Yes    |Yes           |Yes    |Yes     |No              |Yes               |

`Unique<T>` is currently available in `core::ptr` on nightly Rust builds. It's a pointer type that acts like it owns the value it points to. It can be thought of
as a `Box<T>` that does not allocate on initialization or deallocated when it's dropped, and is in fact used to implement common types like `Box<T>`, `Vec<T>`,
etc.

`Shared<T>` is currently available in `core::ptr` on nightly Rust builds. It's the pointer that backs both `Rc<T>` and `Arc<T>`. Its semantics allow for
multiple instances to collectively own the data it points to, and as a result, forbids getting a mutable borrow.

`bevy_ptr` does not support these types right now, but may support [polyfills] for these pointer types if the need arises.

[polyfills]: https://en.wikipedia.org/wiki/Polyfill_(programming)

## Available in `bevy_ptr`

|Pointer Type         |Lifetime'ed|Mutable|Strongly Typed|Aligned|Not Null|Forbids Aliasing|Forbids Arithmetic|
|---------------------|-----------|-------|--------------|-------|--------|----------------|------------------|
|`ConstNonNull<T>`    |No         |No     |Yes           |No     |Yes     |No              |Yes               |
|`ThinSlicePtr<'a, T>`|Yes        |No     |Yes           |Yes    |Yes     |Yes             |Yes               |
|`OwningPtr<'a>`      |Yes        |Yes    |No            |Maybe  |Yes     |Yes             |No                |
|`Ptr<'a>`            |Yes        |No     |No            |Maybe  |Yes     |No              |No                |
|`PtrMut<'a>`         |Yes        |Yes    |No            |Maybe  |Yes     |Yes             |No                |

`ConstNonNull<T>` is like `NonNull<T>` but disallows safe conversions into types that allow mutable access to the value it points to. It's the `*const T` to
`NonNull<T>`'s `*mut T`.

`ThinSlicePtr<'a, T>` is a `&'a [T]` without the slice length. This means it's smaller on the stack, but it means bounds checking is impossible locally, so
accessing elements in the slice is `unsafe`. In debug builds, the length is included and will be checked.

`OwningPtr<'a>`, `Ptr<'a>`, and `PtrMut<'a>` act like `NonNull<()>`, but attempts to restore much of the safety guarantees of `Unique<T>`, `&T`, and `&mut T`.
They allow working with heterogenous type erased storage (i.e. ECS tables, typemaps) without the overhead of dynamic dispatch in a manner that progressively
translates back to safe borrows. These types also support optional alignment requirements at a type level, and will verify it on dereference in debug builds.
