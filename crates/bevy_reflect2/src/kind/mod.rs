mod array;
mod enum_;
mod list;
mod map;
mod primitive;
mod set;
mod struct_;

pub use array::*;
pub use enum_::*;
pub use list::*;
pub use map::*;
pub use primitive::*;
pub use set::*;
pub use struct_::*;

/// Structural information (and access) to a type.
#[derive(Debug)]
pub enum Kind {
    /// A [`Primitive`].
    Primitive(Primitive),
    /// A [`Struct`].
    Struct(Struct),
    /// An [`Enum`].
    Enum(Enum),
    /// A [`List`].
    List(List),
    /// An [`Array`].
    Array(Array),
    /// A [`Map`].
    Map(Map),
    /// A [`Set`].
    Set(Set),
}

/// A reference to a reflected value, split by its [`Kind`].
pub enum KindRef<'a> {
    /// A [`StructRef`], corresponding to a [`Struct`].
    Struct(StructRef<'a>),
    /// An [`EnumRef`], corresponding to an [`Enum`].
    Enum(EnumRef<'a>),
    /// A [`ListRef`], corresponding to a [`List`].
    List(ListRef<'a>),
    /// An [`ArrayRef`], corresponding to an [`Array`].
    Array(ArrayRef<'a>),
    /// A [`MapRef`], corresponding to a [`Map`].
    Map(MapRef<'a>),
    /// A [`SetRef`], corresponding to a [`Set`].
    Set(SetRef<'a>),
    /// A [`Primitive`].
    Primitive(&'static Primitive),
}

/// A mutable reference to a reflected value, split by its [`Kind`].
pub enum KindMut<'a> {
    /// A [`StructMut`], corresponding to a [`Struct`].
    Struct(StructMut<'a>),
    /// An [`EnumMut`], corresponding to an [`Enum`].
    Enum(EnumMut<'a>),
    /// A [`ListMut`], corresponding to a [`List`].
    List(ListMut<'a>),
    /// An [`ArrayMut`], corresponding to an [`Array`].
    Array(ArrayMut<'a>),
    /// A [`MapMut`], corresponding to a [`Map`].
    Map(MapMut<'a>),
    /// A [`SetMut`], corresponding to a [`Set`].
    Set(SetMut<'a>),
    /// A [`Primitive`].
    Primitive(&'static Primitive),
}
