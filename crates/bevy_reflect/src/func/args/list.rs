use crate::func::args::Arg;
use crate::Reflect;

#[derive(Default)]
pub struct ArgList<'a>(Vec<Arg<'a>>);

impl<'a> ArgList<'a> {
    pub fn push(mut self, arg: Arg<'a>) -> Self {
        self.0.push(arg);
        self
    }

    pub fn push_ref(self, arg: &'a dyn Reflect) -> Self {
        self.push(Arg::Ref(arg))
    }

    pub fn push_mut(self, arg: &'a mut dyn Reflect) -> Self {
        self.push(Arg::Mut(arg))
    }

    pub fn push_owned(self, arg: impl Reflect) -> Self {
        self.push(Arg::Owned(Box::new(arg)))
    }

    pub fn push_boxed(self, arg: Box<dyn Reflect>) -> Self {
        self.push(Arg::Owned(arg))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn take(self) -> Vec<Arg<'a>> {
        self.0
    }
}
