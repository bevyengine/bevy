use crate::Reflect;

pub enum Arg<'a> {
    Owned(Box<dyn Reflect>),
    Ref(&'a dyn Reflect),
    Mut(&'a mut dyn Reflect),
}
