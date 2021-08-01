use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::{
    def::Res, hir_id::HirId, intravisit::FnKind, Body, FnDecl, GenericArg, MutTy, Path, QPath, Ty,
    TyKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::{def_id::DefId, symbol::Symbol, Span};

use crate::bevy_paths;

declare_lint! {
    /// **What it does:**
    /// Detectes unnecessary instances of the `With`
    /// **Why is this bad?**
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
    "description goes here"
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
            if_chain! {
                if let TyKind::Path(QPath::Resolved(_, path)) = &typ.kind;
                if path_matches_symbol_path(ctx, path, bevy_paths::QUERY);
                if let Some(segment) = path.segments.iter().last();
                if let Some(generic_args) = segment.args;
                if let Some(GenericArg::Type(world)) = &generic_args.args.get(1);
                if let Some(GenericArg::Type(filter)) = &generic_args.args.get(2);
                then {
                    check_for_overlap(ctx, world, filter);
                }
            }
        }
    }
}

fn check_for_overlap<'hir>(ctx: &LateContext<'hir>, world: &Ty, filter: &Ty) {
    let mut required_types = Vec::new();
    let mut with_types = Vec::new();

    match &world.kind {
        TyKind::Rptr(_, mut_type) => {
            if let Some(def_id) = get_def_id_of_reference(&mut_type) {
                required_types.push(def_id);
            }
        }
        TyKind::Tup(types) => {
            for typ in *types {
                if let TyKind::Rptr(_, mut_type) = &typ.kind {
                    if let Some(def_id) = get_def_id_of_reference(&mut_type) {
                        required_types.push(def_id);
                    }
                }
            }
        }
        _ => (),
    }

    match &filter.kind {
        TyKind::Path(QPath::Resolved(_, path)) => {
            if path_matches_symbol_path(ctx, path, bevy_paths::OR) {
                with_types.extend(check_or_filter(ctx, path));
            }
            if path_matches_symbol_path(ctx, path, bevy_paths::WITH) {
                if let Some(def_id) = get_def_id_of_first_generic_arg(path) {
                    with_types.push((def_id, filter.span));
                }
            }
        }
        TyKind::Tup(types) => {
            for typ in *types {
                if let TyKind::Path(QPath::Resolved(_, path)) = typ.kind {
                    if path_matches_symbol_path(ctx, path, bevy_paths::OR) {
                        with_types.extend(check_or_filter(ctx, path));
                    }
                    if path_matches_symbol_path(ctx, path, bevy_paths::ADDED)
                        || path_matches_symbol_path(ctx, path, bevy_paths::CHANGED)
                    {
                        if let Some(def_id) = get_def_id_of_first_generic_arg(path) {
                            required_types.push(def_id);
                        }
                    }
                    if path_matches_symbol_path(ctx, path, bevy_paths::WITH) {
                        if let Some(def_id) = get_def_id_of_first_generic_arg(path) {
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

fn get_def_id_of_reference(reference: &MutTy) -> Option<DefId> {
    if let TyKind::Path(QPath::Resolved(_, path)) = reference.ty.kind {
        if let Res::Def(_, def_id) = path.res {
            return Some(def_id);
        }
    }

    None
}

fn path_matches_symbol_path<'hir>(
    ctx: &LateContext<'hir>,
    path: &Path,
    symbol_path: &[&str],
) -> bool {
    if let Res::Def(_, def_id) = path.res {
        return ctx.match_def_path(
            def_id,
            symbol_path
                .iter()
                .map(|str| Symbol::intern(str))
                .collect::<Vec<_>>()
                .as_slice(),
        );
    };

    false
}

fn get_def_id_of_first_generic_arg(path: &Path) -> Option<DefId> {
    if let Some(segment) = path.segments.iter().last() {
        if let Some(generic_args) = segment.args {
            if let Some(GenericArg::Type(component)) = &generic_args.args.get(0) {
                if let TyKind::Path(QPath::Resolved(_, path)) = component.kind {
                    if let Res::Def(_, def_id) = path.res {
                        return Some(def_id);
                    }
                }
            }
        }
    }

    None
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
                            if path_matches_symbol_path(ctx, path, bevy_paths::ADDED)
                                || path_matches_symbol_path(ctx, path, bevy_paths::CHANGED)
                            {
                                if let Some(def_id) = get_def_id_of_first_generic_arg(path) {
                                    local_required_types.push(def_id);
                                }
                            }
                            if path_matches_symbol_path(ctx, path, bevy_paths::WITH) {
                                if let Some(def_id) = get_def_id_of_first_generic_arg(path) {
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
