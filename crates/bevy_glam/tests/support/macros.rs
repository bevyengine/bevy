#[macro_export]
macro_rules! assert_approx_eq {
    ($a:expr, $b:expr) => {{
        #[allow(unused_imports)]
        use support::FloatCompare;
        let eps = core::f32::EPSILON;
        let (a, b) = (&$a, &$b);
        assert!(
            a.approx_eq(b, eps),
            "assertion failed: `(left !== right)` \
             (left: `{:?}`, right: `{:?}`, expect diff: `{:?}`, real diff: `{:?}`)",
            *a,
            *b,
            eps,
            a.abs_diff(b)
        );
    }};
    ($a:expr, $b:expr, $eps:expr) => {{
        use support::FloatCompare;
        let (a, b) = (&$a, &$b);
        let eps = $eps;
        assert!(
            a.approx_eq(b, $eps),
            "assertion failed: `(left !== right)` \
             (left: `{:?}`, right: `{:?}`, expect diff: `{:?}`, real diff: `{:?}`)",
            *a,
            *b,
            eps,
            a.abs_diff(b)
        );
    }};
}
