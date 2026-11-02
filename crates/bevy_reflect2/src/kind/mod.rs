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
    /// The [`ListAccess`] of a [`List`].
    List(&'a dyn ListAccess),
    /// An [`ArrayRef`], corresponding to an [`Array`].
    Array(ArrayRef<'a>),
    /// The [`MapAccess`] of a [`Map`].
    Map(&'a dyn MapAccess),
    /// The [`SetAccess`] of a [`Set`].
    Set(&'a dyn SetAccess),
    /// A [`Primitive`].
    Primitive(&'static Primitive),
}

/// A mutable reference to a reflected value, split by its [`Kind`].
pub enum KindMut<'a> {
    /// A [`StructMut`], corresponding to a [`Struct`].
    Struct(StructMut<'a>),
    /// An [`EnumMut`], corresponding to an [`Enum`].
    Enum(EnumMut<'a>),
    /// The mutable [`ListAccess`] of a [`List`].
    List(&'a mut dyn ListAccess),
    /// An [`ArrayMut`], corresponding to an [`Array`].
    Array(ArrayMut<'a>),
    /// The mutable [`MapAccess`] of a [`Map`].
    Map(&'a mut dyn MapAccess),
    /// The mutable [`SetAccess`] of a [`Set`].
    Set(&'a mut dyn SetAccess),
    /// A [`Primitive`].
    Primitive(&'static Primitive),
}
