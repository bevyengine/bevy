use bevy_macro_utils::ensure_no_collision;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote, parse_quote_spanned,
    spanned::Spanned,
    visit_mut::VisitMut,
    Expr, ExprMacro, Ident, Pat, Path, Token,
};

use crate::bevy_ecs_path;

struct ExpandSystemCalls {
    call_ident: Ident,
    systems_ident: Ident,
    system_paths: Vec<Path>,
}

struct SystemCall {
    path: Path,
    _comma: Option<Token![,]>,
    input: Option<Box<Expr>>,
}

impl Parse for SystemCall {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        let comma: Option<Token![,]> = input.parse()?;
        let input = comma.as_ref().and_then(|_| input.parse().ok());
        Ok(Self {
            path,
            _comma: comma,
            input,
        })
    }
}

impl VisitMut for ExpandSystemCalls {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let Expr::Macro(ExprMacro { attrs: _, mac }) = i
            && mac.path.is_ident(&self.call_ident)
        {
            let call = match mac.parse_body::<SystemCall>() {
                Ok(call) => call,
                Err(err) => {
                    *i = Expr::Verbatim(err.into_compile_error());
                    return;
                }
            };

            let call_index = match self.system_paths.iter().position(|p| p == &call.path) {
                Some(i) => i,
                None => {
                    let len = self.system_paths.len();
                    self.system_paths.push(call.path.clone());
                    len
                }
            };

            let systems_ident = &self.systems_ident;
            let system_accessor = format_ident!("p{}", call_index);
            let expr: Expr = match &call.input {
                Some(input) => {
                    parse_quote_spanned!(mac.span()=> #systems_ident.#system_accessor().run_with(#input))
                }
                None => {
                    parse_quote_spanned!(mac.span()=> #systems_ident.#system_accessor().run())
                }
            };
            *i = expr;
        } else {
            syn::visit_mut::visit_expr_mut(self, i);
        }
    }
}

pub fn compose(input: TokenStream, has_input: bool) -> TokenStream {
    let bevy_ecs_path = bevy_ecs_path();
    let call_ident = format_ident!("run");
    let systems_ident = ensure_no_collision(format_ident!("__systems"), input.clone());
    let mut expr_closure = parse_macro_input!(input as syn::ExprClosure);

    let mut visitor = ExpandSystemCalls {
        call_ident,
        systems_ident: systems_ident.clone(),
        system_paths: Vec::new(),
    };

    syn::visit_mut::visit_expr_closure_mut(&mut visitor, &mut expr_closure);

    let runner_types: Vec<syn::Type> = visitor
        .system_paths
        .iter()
        .map(|path| parse_quote_spanned!(path.span()=> #bevy_ecs_path::system::SystemRunner<_, _, _>))
        .collect();

    let param_count = if has_input {
        if expr_closure.inputs.is_empty() {
            return TokenStream::from(
                syn::Error::new_spanned(
                    &expr_closure.inputs,
                    "closure must have at least one parameter",
                )
                .into_compile_error(),
            );
        }
        expr_closure.inputs.len() - 1
    } else {
        expr_closure.inputs.len()
    };

    let mut builders: Vec<Expr> =
        vec![parse_quote!(#bevy_ecs_path::system::ParamBuilder); param_count];
    let system_builders: Vec<Expr> = visitor.system_paths.iter().map(|path| parse_quote_spanned!(path.span()=> #bevy_ecs_path::system::ParamBuilder::system(#path))).collect();
    builders.push(parse_quote!(#bevy_ecs_path::system::ParamSetBuilder((#(#system_builders,)*))));

    expr_closure.inputs.push(Pat::Type(
        parse_quote!(mut #systems_ident: #bevy_ecs_path::system::ParamSet<(#(#runner_types,)*)>),
    ));

    TokenStream::from(quote! {
        #bevy_ecs_path::system::SystemParamBuilder::build_system(
            (#(#builders,)*),
            #expr_closure
        )
    })
}
