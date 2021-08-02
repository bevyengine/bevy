use rustc_hir::{def::Res, GenericArg, MutTy, Path, QPath, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_span::{def_id::DefId, symbol::Symbol};

use crate::bevy_paths;

pub fn path_matches_symbol_path<'hir>(
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

pub fn get_def_id_of_referenced_type(reference: &MutTy) -> Option<DefId> {
    if let TyKind::Path(QPath::Resolved(_, path)) = reference.ty.kind {
        if let Res::Def(_, def_id) = path.res {
            return Some(def_id);
        }
    }

    None
}

pub fn get_def_id_of_first_generic_arg(path: &Path) -> Option<DefId> {
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

pub fn get_generics_of_query<'hir>(
    ctx: &LateContext<'hir>,
    query: &'hir Ty,
) -> Option<(&'hir Ty<'hir>, Option<&'hir Ty<'hir>>)> {
    if let TyKind::Path(QPath::Resolved(_, path)) = query.kind {
        if path_matches_symbol_path(ctx, path, bevy_paths::QUERY) {
            if let Some(segment) = path.segments.iter().last() {
                if let Some(generic_args) = segment.args {
                    if let Some(GenericArg::Type(world)) = &generic_args.args.get(1) {
                        if let Some(GenericArg::Type(filter)) = &generic_args.args.get(2) {
                            return Some((world, Some(filter)));
                        }

                        return Some((world, None));
                    }
                }
            }
        }
    }

    None
}
