use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::alloc::collections::BinaryHeap<T: Clone>(Clone));

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use bevy_reflect::Reflect;

    #[test]
    fn should_partial_eq_btree_map() {
        let mut a = BTreeMap::new();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = BTreeMap::new();
        c.insert(0usize, 3.21_f64);

        let a: &dyn Reflect = &a;
        let b: &dyn Reflect = &b;
        let c: &dyn Reflect = &c;
        assert!(a
            .reflect_partial_eq(b.as_partial_reflect())
            .unwrap_or_default());
        assert!(!a
            .reflect_partial_eq(c.as_partial_reflect())
            .unwrap_or_default());
    }
}
