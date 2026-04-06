use quote::ToTokens;
use syn::{
    spanned::Spanned,
    visit::{visit_macro, Visit},
    Macro,
};

use super::*;
use crate::bsn::types::*;

impl<'ast, 'a> Visit<'ast> for BsnVisitor<'a> {
    fn visit_macro(&mut self, mac: &'ast Macro) {
        let is_bsn = mac.path.is_ident("bsn");
        let is_bsn_list = mac.path.is_ident("bsn_list");
        if !is_bsn && !is_bsn_list {
            return syn::visit::visit_macro(self, mac);
        };

        let base_indent = mac.path.span().start().column;
        let formatted = if is_bsn {
            syn::parse2::<BsnRoot>(mac.tokens.clone())
                .ok()
                .map(|ast| ast.fmt(base_indent, 0))
        } else {
            syn::parse2::<BsnListRoot>(mac.tokens.clone())
                .ok()
                .map(|ast| ast.fmt(base_indent, 0))
        };

        let Some(new_text) = formatted else {
            eprintln!(
                "\x1b[1;31mFailed to parse bsn block at line {}\x1b[0m",
                mac.span().start().line
            );
            return visit_macro(self, mac);
        };

        let mac_span = mac.span();
        let mac_start_byte =
            col_to_offset(self.source, mac_span.start().line, mac_span.start().column);
        let mac_end_byte = col_to_offset(self.source, mac_span.end().line, mac_span.end().column);
        let mac_text = &self.source[mac_start_byte..mac_end_byte];

        let is_comment = mac_text.contains("//") || mac_text.contains("/*");
        if is_comment {
            eprintln!(
                "\x1b[1;33m⚠ Skipped bsn! block at line {}:
                    Formatting blocks with comments is currently unsupported.\x1b[0m",
                mac_span.start().line
            );
            return visit_macro(self, mac);
        };

        let (Some(open_idx), Some(close_idx)) = (
            mac_text.find(['{', '(', '[']),
            mac_text.rfind(['}', ')', ']']),
        ) else {
            return visit_macro(self, mac);
        };

        let start_byte = mac_start_byte + open_idx + 1;
        let end_byte = mac_start_byte + close_idx;
        let (start_line, start_col) = offset_to_col(self.source, start_byte);
        let (end_line, end_col) = offset_to_col(self.source, end_byte);
        let original = &self.source[start_byte..end_byte];

        let r#final = if is_bsn_list && !new_text.contains('\n') {
            new_text.trim().to_string()
        } else {
            format!(
                "\n{}\n{}",
                new_text.trim_start_matches('\n').trim_end(),
                " ".repeat(base_indent)
            )
        };

        if original != r#final {
            self.edits.push(Edit {
                start_line,
                start_col,
                end_line,
                end_col,
                original_text: original.to_string(),
                new_text: r#final,
            });
        }

        visit_macro(self, mac);
    }
}

impl BsnFmt for BsnRoot {
    fn fmt(&self, base: usize, level: usize) -> String {
        self.0.fmt_content(base, level + 1).trim_end().to_string()
    }
}

impl<const ALLOW_FLAT: bool> Bsn<ALLOW_FLAT> {
    fn fmt_content(&self, base: usize, level: usize) -> String {
        self.entries.iter().fold(String::new(), |mut out, entry| {
            out.push_str(&indent(base, level));
            out.push_str(&entry.fmt(base, level));
            out.push('\n');
            out
        })
    }
}

impl<const ALLOW_FLAT: bool> BsnFmt for Bsn<ALLOW_FLAT> {
    fn fmt(&self, base: usize, level: usize) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        if self.entries.len() == 1 {
            return self.entries[0].fmt(base, level);
        }

        let mut out = String::from("(\n");
        out.push_str(&self.fmt_content(base, level + 1));
        out.push_str(&indent(base, level));
        out.push(')');
        out
    }
}

impl BsnFmt for BsnListRoot {
    fn fmt(&self, base: usize, level: usize) -> String {
        let items = &self.0 .0;
        let single_line_items: Vec<String> = items.iter().map(|item| item.fmt(0, 0)).collect();

        let combined = single_line_items.join(", ");
        if combined.len() < 40 && !combined.contains('\n') {
            combined
        } else {
            self.0.fmt(base, level + 1)
        }
    }
}

impl BsnFmt for BsnSceneList {
    fn fmt(&self, base: usize, level: usize) -> String {
        // Keep it compact if it's only 1 item
        if self.0 .0.len() == 1
            && let BsnSceneListItem::Scene(bsn) = &self.0 .0[0]
        {
            return format!("[{}]", bsn.fmt(base, level));
        }

        let mut out = String::from("[\n");
        out.push_str(&self.0.fmt(base, level + 1));
        out.push_str(&indent(base, level));
        out.push(']');
        out
    }
}

impl BsnFmt for BsnSceneListItem {
    fn fmt(&self, base: usize, level: usize) -> String {
        match self {
            BsnSceneListItem::Scene(bsn) => bsn.fmt(base, level),
            BsnSceneListItem::Expression(stmts) => stmts
                .iter()
                .map(|s| {
                    fmt_rust_expr(&s.to_token_stream(), base, level)
                        .trim()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

impl BsnFmt for BsnSceneListItems {
    fn fmt(&self, base: usize, level: usize) -> String {
        self.0.iter().fold(String::new(), |mut out, item| {
            out.push_str(&indent(base, level));
            out.push_str(&item.fmt(base, level));
            out.push_str(",\n");
            out
        })
    }
}

impl BsnFmt for BsnEntry {
    fn fmt(&self, base: usize, level: usize) -> String {
        match self {
            BsnEntry::InheritedScene(s) => s.fmt(base, level),
            BsnEntry::Name(ident) => format!("#{}", ident),
            BsnEntry::NameExpression(expr) => {
                format!("#{{ {} }}", fmt_quote(expr))
            }
            BsnEntry::SceneExpression(block) => {
                fmt_rust_expr(&block.to_token_stream(), base, level)
                    .trim()
                    .to_string()
            }
            BsnEntry::TemplatePatch(ty) => {
                format!("@{}", ty.fmt(base, level))
            }
            BsnEntry::FromTemplatePatch(ty) => ty.fmt(base, level),
            BsnEntry::TemplateConst {
                type_path,
                const_ident,
            } => {
                format!("@{}::{}", fmt_quote(type_path), const_ident)
            }
            BsnEntry::TemplateConstructor(c) | BsnEntry::FromTemplateConstructor(c) => {
                let prefix = if matches!(self, BsnEntry::TemplateConstructor(_)) {
                    "@"
                } else {
                    ""
                };
                let mut out = format!("{}{}", prefix, fmt_quote(&c.type_path));
                out.push_str("::");
                out.push_str(&c.function.to_string());

                if let Some(args) = &c.args {
                    let formatted_args: Vec<_> = args.iter().map(fmt_quote).collect();
                    out.push('(');
                    out.push_str(&formatted_args.join(", "));
                    out.push(')');
                }
                out
            }
            BsnEntry::RelatedSceneList(r) => {
                format!(
                    "{} {}",
                    fmt_quote(&r.relationship_path),
                    r.scene_list.fmt(base, level)
                )
            }
        }
    }
}

impl BsnFmt for BsnInheritedScene {
    fn fmt(&self, _base: usize, _level: usize) -> String {
        match self {
            BsnInheritedScene::Asset(lit) => format!(": {}", fmt_quote(lit)),
            BsnInheritedScene::Fn { function, args } => {
                let mut out = format!(":{}", function);
                if let Some(a) = args {
                    out.push('(');
                    out.push_str(&a.iter().map(fmt_quote).collect::<Vec<_>>().join(", "));
                    out.push(')');
                }
                out
            }
        }
    }
}

impl BsnFmt for BsnType {
    fn fmt(&self, base: usize, level: usize) -> String {
        let mut out = String::new();
        out.push_str(&fmt_quote(&self.path));

        if let Some(variant) = &self.enum_variant {
            out.push_str("::");
            out.push_str(&variant.to_string());
        }

        match &self.fields {
            BsnFields::Named(fields) => {
                if !fields.is_empty() {
                    out.push_str(" {\n");
                    out.push_str(&fmt_list_with(fields, base, level + 1, ",\n", |f, b, l| {
                        let mut s = f.name.to_string();
                        if let Some(val) = &f.value {
                            s.push_str(": ");
                            s.push_str(&val.fmt(b, l));
                        }
                        s
                    }));

                    out.push_str(&indent(base, level));
                    out.push('}');
                }
            }
            BsnFields::Tuple(fields) => {
                if !fields.is_empty() {
                    out.push('(');
                    let formatted_fields: Vec<_> =
                        fields.iter().map(|f| f.value.fmt(base, level)).collect();

                    out.push_str(&formatted_fields.join(", "));
                    out.push(')');
                }
            }
        }
        out
    }
}

impl BsnFmt for BsnValue {
    fn fmt(&self, base: usize, level: usize) -> String {
        match self {
            BsnValue::Expr(ts) => fmt_rust_expr(ts, base, level).trim().to_string(),
            BsnValue::Closure(ts) => fmt_rust_expr(ts, base, level).trim().to_string(),
            BsnValue::Ident(i) => i.to_string(),
            BsnValue::Lit(l) => fmt_quote(l),
            BsnValue::Tuple(t) => {
                let formatted_items: Vec<_> = t.0.iter().map(|val| val.fmt(base, level)).collect();
                format!("({})", formatted_items.join(" "))
            }
            BsnValue::Type(ty) => ty.fmt(base, level),
            BsnValue::Name(ident) => format!("#{}", ident),
        }
    }
}
