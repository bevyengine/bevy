/// A type that does not describe its structure. These are "leaf" types, from a "structural reflection"
/// perspective.
#[derive(Debug)]
pub enum Primitive {
    /// The "unit" type `()`.
    Unit,
    /// A "boolean" type (true or false),
    Bool,
    /// An "integer" type. This can be signed or unsigned and of any precision.
    Int,
    /// A "float" type of any precision.
    Float,
    /// A single "character" type.
    Char,
    /// A "string" type.
    String,
    /// A type whose internals are not reflected. It can only be interacted with through its
    /// [`TypeVTable`](crate::TypeVTable) and [`TypeData`](crate::TypeData), or by downcasting it.
    Opaque,
}
