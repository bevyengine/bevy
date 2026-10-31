use quote::ToTokens;
use syn::{
    spanned::Spanned,
    visit::{visit_macro, Visit},
    Macro,
};

use super::*;
use crate::_bsn::types::*;

impl<'ast, 'a> Visit<'ast> for BsnVisitor<'a> {
    fn visit_macro(&mut self, mac: &'ast Macro) {
        let is_bsn = mac.path.is_ident("bsn");
        let is_bsn_list = mac.path.is_ident("bsn_list");
        if !is_bsn && !is_bsn_list {
            return visit_macro(self, mac);
        };

        let mac_span = mac.span();
        let line_indent = {
            let start_line_number: usize = mac_span.start().line;
            let line = self.source.lines().nth(start_line_number - 1).unwrap();
            line.len() - line.trim_start().len()
        };

        let base_indent: usize = line_indent;
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

        let mac_start_byte =
            col_to_offset(self.source, mac_span.start().line, mac_span.start().column);
        let mac_end_byte = col_to_offset(self.source, mac_span.end().line, mac_span.end().column);
        let mac_text = &self.source[mac_start_byte..mac_end_byte];

        let contains_comments = mac_text.contains("//") || mac_text.contains("/*");
        if contains_comments {
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
                " ".repeat(line_indent)
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

impl Bsn {
    fn fmt_content(&self, base: usize, level: usize) -> String {
        self.entries.iter().fold(String::new(), |mut out, entry| {
            out.push_str(&indent(base, level));
            out.push_str(&entry.fmt(base, level));
            out.push('\n');
            out
        })
    }
}

impl BsnFmt for Bsn {
    fn fmt(&self, base: usize, level: usize) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        out.push_str(&self.fmt_content(base, level + 1));
        out.push_str(&indent(base, level));
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
        if self.0 .0.len() == 0 {
            return String::from("[]");
        }

        let mut out = String::from("[\n");
        out.push_str(&self.0.fmt(base, level));
        out.push(']');
        out
    }
}

impl BsnFmt for BsnSceneListItem {
    fn fmt(&self, base: usize, level: usize) -> String {
        match self {
            BsnSceneListItem::Scene(bsn) => bsn.fmt(base, level),
            BsnSceneListItem::Expression(token_stream) => {
                fmt_rust_expr(token_stream, base, level).trim().to_string()
            }
        }
    }
}

impl BsnFmt for BsnSceneListItems {
    fn fmt(&self, base: usize, level: usize) -> String {
        let mut out = String::new();
        let mut items = self.0.iter().peekable();
        while let Some(item) = items.next() {
            out.push_str(&item.fmt(base, level));
            if items.peek().is_some() {
                out.push_str(&indent(base, level));
                out.push_str("--\n");
            }
        }
        out
    }
}

impl BsnFmt for BsnEntry {
    fn fmt(&self, base: usize, level: usize) -> String {
        match self {
            BsnEntry::Name(ident) => format!("#{}", ident),
            BsnEntry::FromTemplatePatch(ty) => ty.fmt(base, level),
            BsnEntry::TemplatePatch(ty) => {
                format!("@{}", ty.fmt(base, level))
            }
            BsnEntry::TemplateConstructor {
                constructor,
                dot_expression,
            }
            | BsnEntry::FromTemplateConstructor {
                constructor,
                dot_expression,
            } => {
                let prefix = if matches!(self, BsnEntry::TemplateConstructor { .. }) {
                    "@"
                } else {
                    ""
                };
                let mut out = format!("{}{}", prefix, fmt_quote(&constructor.type_path));
                out.push_str("::");
                out.push_str(&constructor.function.to_string());

                let args = constructor.args.fmt(base, level);
                out.push('(');
                out.push_str(&args);
                out.push(')');

                if let Some(dot_expression) = dot_expression {
                    out.push_str(&fmt_rust_dot_expr(dot_expression));
                }

                out
            }
            BsnEntry::TemplateValue(token_stream) => {
                fmt_rust_expr(token_stream, base, level).trim().to_string()
            }
            BsnEntry::Function(fn_call) => fn_call.fmt(base, level),
            BsnEntry::UncachedScene(s) => s.fmt(base, level),
            BsnEntry::CachedScene(s) => s.fmt(base, level),
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

impl BsnFmt for BsnScene {
    fn fmt(&self, base_indent: usize, level: usize) -> String {
        match self {
            BsnScene::Asset(lit) => format!(": {}", fmt_quote(lit)),
            BsnScene::Fn(BsnSceneFn { path, args }) => {
                let mut out = format!("@{}", fmt_quote(path));
                out.push('(');
                if !args.0.is_empty() {
                    let args = args
                        .0
                        .iter()
                        .map(|a| a.fmt(0, 0))
                        .collect::<Vec<String>>()
                        .join(", ");
                    out.push_str(&args);
                }
                out.push(')');
                out
            }
            BsnScene::SceneComponent(BsnType {
                path,
                variant,
                fields,
            }) => {
                let mut out = format!("@{}", fmt_quote(path));
                if let Some(variant) = variant {
                    out.push_str(&format!("::{}", variant.to_string()));
                };
                out.push_str(&fields.fmt(base_indent, level));
                out
            }
            BsnScene::Expression(token_stream) => format!("@{}", fmt_quote(token_stream)),
        }
    }
}

impl BsnFmt for BsnType {
    fn fmt(&self, base: usize, level: usize) -> String {
        let mut out = String::new();
        out.push_str(&fmt_quote(&self.path));

        if let Some(variant) = &self.variant {
            out.push_str("::");
            out.push_str(&variant.to_string());
        }

        out.push_str(&self.fields.fmt(base, level));
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
            BsnValue::Range { .. } => todo!(),
        }
    }
}

impl BsnFmt for BsnFnCall {
    fn fmt(&self, base_indent: usize, level: usize) -> String {
        let mut out = format!("{}", self.path.to_token_stream().to_string());
        out.push('(');
        out.push_str(&self.args.fmt(base_indent, level));
        out.push(')');
        out
    }
}

impl BsnFmt for BsnFields {
    fn fmt(&self, base_indent: usize, level: usize) -> String {
        let mut out = String::new();
        match self {
            BsnFields::Named { fields, .. } => {
                if !fields.is_empty() {
                    out.push_str(" {\n");
                    out.push_str(&fmt_list_with(
                        fields,
                        base_indent,
                        level + 1,
                        ",\n",
                        |f, b, l| {
                            let mut s = String::new();

                            if f.is_prop {
                                s.push_str("@");
                            }

                            s.push_str(&f.name.to_string());
                            if let Some(val) = &f.value {
                                s.push_str(": ");
                                s.push_str(&val.fmt(b, l));
                            }
                            s
                        },
                    ));

                    out.push_str(&indent(base_indent, level));
                    out.push('}');
                }
            }
            BsnFields::Tuple(fields) => {
                out.push('(');
                let formatted_fields: Vec<_> = fields
                    .iter()
                    .map(|f| f.value.fmt(base_indent, level))
                    .collect();
                out.push_str(&formatted_fields.join(", "));
                out.push(')');
            }
            BsnFields::Unit => {}
        }
        out
    }
}

impl BsnFmt for BsnFnArg {
    fn fmt(&self, base_indent: usize, level: usize) -> String {
        match self {
            BsnFnArg::Array(bsn_values) => bsn_values
                .iter()
                .map(|v| fmt_quote(v))
                .collect::<Vec<_>>()
                .join(", "),
            BsnFnArg::EntityName(ident) => ident.to_string(),
            BsnFnArg::Tokens(token_stream) => fmt_rust_expr(token_stream, base_indent, level)
                .trim()
                .to_string(),
        }
    }
}

impl BsnFmt for BsnFnArgs {
    fn fmt(&self, base_indent: usize, level: usize) -> String {
        self.0
            .iter()
            .map(|a| a.fmt(base_indent, level))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
