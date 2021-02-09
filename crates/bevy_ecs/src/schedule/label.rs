use downcast_rs::Downcast;
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
};

use crate::{StageLabel, SystemLabel};

pub struct Label<M>(Arc<dyn IntoLabel<M>>, PhantomData<M>);

impl<T: 'static> Clone for Label<T> {
    fn clone(&self) -> Self {
        Label(self.0.clone(), Default::default())
    }
}

impl<T: 'static> Hash for Label<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.dyn_hash(state);
    }
}

impl<T: 'static> PartialEq for Label<T> {
    fn eq(&self, other: &Self) -> bool {
        // Consider using pointer comparison like https://github.com/rust-lang/rust/issues/46139#issuecomment-416101127
        self.0.downcast_eq(&*other.0.as_ref())
    }
}

impl<T: 'static> Eq for Label<T> {}

impl<T: 'static> Display for Label<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_str())
    }
}

impl<T: 'static> Debug for Label<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        f.write_str(&self.to_str())?;
        f.write_str("\"")?;
        Ok(())
    }
}

impl<T: 'static> Label<T> {
    pub fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }
}

pub trait IntoLabel<M: 'static>: Downcast + Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;
    fn downcast_eq(&self, other: &dyn IntoLabel<M>) -> bool;
    fn dyn_hash(&self, hasher: &mut dyn Hasher);
}

impl<T: IntoLabel<SystemLabel> + Eq + Hash + Clone> From<T> for Label<SystemLabel> {
    fn from(t: T) -> Self {
        Label(Arc::new(t), Default::default())
    }
}

impl<T: IntoLabel<StageLabel> + Eq + Hash + Clone> From<T> for Label<StageLabel> {
    fn from(t: T) -> Self {
        Label(Arc::new(t), Default::default())
    }
}

pub trait FullIntoLabel<M: 'static>: IntoLabel<M> + Eq + Hash + Clone {}

impl<M: 'static, T: IntoLabel<M> + Eq + Hash + Clone> FullIntoLabel<M> for T {}

downcast_rs::impl_downcast!(IntoLabel<M>);

impl<M: 'static> Label<M> {
    pub fn to_str(&self) -> Cow<'static, str> {
        self.0.name()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::IntoLabel;
    struct LabelT;
    #[derive(IntoLabel, PartialEq, Eq, Hash, Debug)]
    #[label_type(LabelT)]
    struct L(&'static str);
    #[test]
    fn label_eq_test() {
        let label_1 = L("A");
        let label_2 = L("A");
        assert_eq!(label_1, label_2);
    }
}
