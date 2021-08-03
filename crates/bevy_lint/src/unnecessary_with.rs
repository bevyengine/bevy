use clippy_utils::diagnostics::span_lint;
use rustc_hir::{
    hir_id::HirId, intravisit::FnKind, Body, FnDecl, GenericArg, Path, QPath, Ty, TyKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::{def_id::DefId, Span};

use crate::{bevy_helpers, bevy_paths};

declare_lint! {
    /// **What it does:**
    /// Detects unnecessary `With` query filters in Bevy query parameters.
    /// **Why is this bad?**
    /// The Filter does not effect the Results of a query, but still wasted space.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// # use bevy_ecs::system::Query;
    /// # use bevy_ecs::query::With;
    /// fn system(query: Query<&A, With<A>>) {}
    /// ```
    /// Use instead:
    /// ```rust
    /// # use bevy_ecs::system::Query;
    /// fn system(query: Query<&A>) {}
    /// ```
    pub UNNECESSARY_WITH,
    Warn,
    "Detects unnecessary `With` query filters in Bevy query parameters."
}

declare_lint_pass!(UnnecessaryWith => [UNNECESSARY_WITH]);

impl<'hir> LateLintPass<'hir> for UnnecessaryWith {
    // A list of things you might check can be found here:
    // https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html

    fn check_fn(
        &mut self,
        ctx: &LateContext<'hir>,
        _: FnKind<'hir>,
        decl: &'hir FnDecl<'hir>,
        _: &'hir Body<'hir>,
        _: Span,
        _: HirId,
    ) {
        for typ in decl.inputs {
            recursively_search_type(ctx, typ, bevy_paths::QUERY, &check_for_unnecesarry_with);
        }
    }
}

fn recursively_search_type<'hir, T: Fn(&LateContext<'hir>, &'hir Ty<'hir>) -> ()>(
    ctx: &LateContext<'hir>,
    typ: &'hir Ty,
    symbol_path: &[&str],
    function: &T,
) {
    match &typ.kind {
        TyKind::Path(QPath::Resolved(_, path)) => {
            if bevy_helpers::path_matches_symbol_path(ctx, path, symbol_path) {
                (function)(ctx, &typ)
            }
        }
        TyKind::Tup(types) => {
            for tup_typ in *types {
                // Todo: Filter out Types that dont implement SystemParam
                // -> Is it possible to go from rustc_hir::Ty to rustc_middle::Ty?
                // Required for using clippy_utils::ty::implements_trait.
                recursively_search_type(ctx, tup_typ, symbol_path, function);
            }
        }
        _ => (),
    }
}

fn check_for_unnecesarry_with<'hir>(ctx: &LateContext<'hir>, query: &'hir Ty<'hir>) {
    if let Some((world, Some(filter))) = bevy_helpers::get_generics_of_query(ctx, query) {
        let mut required_types = Vec::new();
        let mut with_types = Vec::new();

        match &world.kind {
            TyKind::Rptr(_, mut_type) => {
                if let Some(def_id) = bevy_helpers::get_def_id_of_referenced_type(&mut_type) {
                    required_types.push(def_id);
                }
            }
            TyKind::Tup(types) => {
                for typ in *types {
                    if let TyKind::Rptr(_, mut_type) = &typ.kind {
                        if let Some(def_id) = bevy_helpers::get_def_id_of_referenced_type(&mut_type)
                        {
                            required_types.push(def_id);
                        }
                    }
                }
            }
            _ => (),
        }

        match &filter.kind {
            TyKind::Path(QPath::Resolved(_, path)) => {
                if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::OR) {
                    with_types.extend(check_or_filter(ctx, path));
                }
                if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::WITH) {
                    if let Some(def_id) = bevy_helpers::get_def_id_of_first_generic_arg(path) {
                        with_types.push((def_id, filter.span));
                    }
                }
            }
            TyKind::Tup(types) => {
                for typ in *types {
                    if let TyKind::Path(QPath::Resolved(_, path)) = typ.kind {
                        if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::OR) {
                            with_types.extend(check_or_filter(ctx, path));
                        }
                        if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::ADDED)
                            || bevy_helpers::path_matches_symbol_path(
                                ctx,
                                path,
                                bevy_paths::CHANGED,
                            )
                        {
                            if let Some(def_id) =
                                bevy_helpers::get_def_id_of_first_generic_arg(path)
                            {
                                required_types.push(def_id);
                            }
                        }
                        if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::WITH) {
                            if let Some(def_id) =
                                bevy_helpers::get_def_id_of_first_generic_arg(path)
                            {
                                with_types.push((def_id, typ.span));
                            }
                        }
                    }
                }
            }
            _ => (),
        }

        for with_type in with_types {
            if required_types.contains(&with_type.0) {
                span_lint(
                    ctx,
                    UNNECESSARY_WITH,
                    with_type.1,
                    "Unnecessary `With` Filter",
                );
            }
        }
    }
}

fn check_or_filter<'hir>(ctx: &LateContext<'hir>, path: &Path) -> Vec<(DefId, Span)> {
    let mut local_required_types = Vec::new();
    let mut local_with_types = Vec::new();

    if let Some(segment) = path.segments.iter().last() {
        if let Some(generic_args) = segment.args {
            if let GenericArg::Type(tuple) = &generic_args.args[0] {
                if let TyKind::Tup(types) = tuple.kind {
                    for typ in types {
                        if let TyKind::Path(QPath::Resolved(_, path)) = typ.kind {
                            if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::ADDED)
                                || bevy_helpers::path_matches_symbol_path(
                                    ctx,
                                    path,
                                    bevy_paths::CHANGED,
                                )
                            {
                                if let Some(def_id) =
                                    bevy_helpers::get_def_id_of_first_generic_arg(path)
                                {
                                    local_required_types.push(def_id);
                                }
                            }
                            if bevy_helpers::path_matches_symbol_path(ctx, path, bevy_paths::WITH) {
                                if let Some(def_id) =
                                    bevy_helpers::get_def_id_of_first_generic_arg(path)
                                {
                                    local_with_types.push((def_id, typ.span));
                                }
                            }
                        }
                    }

                    for with_type in &local_with_types {
                        if local_required_types.contains(&with_type.0) {
                            span_lint(
                                ctx,
                                UNNECESSARY_WITH,
                                with_type.1,
                                "Unnecessary `With` Filter",
                            );
                        }
                    }
                }
            }
        }
    }

    local_with_types
}
