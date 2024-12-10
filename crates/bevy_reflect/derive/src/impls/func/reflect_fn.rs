use bevy_macro_utils::fq_std::FQResult;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, parse_macro_input, token, Block, ExprBlock, Type};
use syn::{LitStr, ReturnType, Token};

struct Arg {
    mutability: Option<Token![mut]>,
    name: Ident,
    _colon: Token![:],
    ty: Type,
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mutability = if input.peek(Token![mut]) {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            mutability,
            name: input.parse()?,
            _colon: input.parse()?,
            ty: input.parse()?,
        })
    }
}

enum FnName {
    #[expect(
        dead_code,
        reason = "for documenting via the type system what `Anon` expects to parse"
    )]
    Anon(Token![_]),
    Lit(LitStr),
    Name(Ident),
    Expr(ExprBlock),
}

impl Parse for FnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![_]) {
            Ok(Self::Anon(input.parse()?))
        } else if lookahead.peek(LitStr) {
            Ok(Self::Lit(input.parse()?))
        } else if lookahead.peek(syn::Ident) {
            Ok(Self::Name(input.parse()?))
        } else if lookahead.peek(token::Brace) {
            Ok(Self::Expr(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

struct ReflectFn {
    mutability: Option<Token![mut]>,
    movability: Option<Token![move]>,
    _fn_token: Token![fn],
    name: FnName,
    _parens: token::Paren,
    args: Punctuated<Arg, Token![,]>,
    return_type: ReturnType,
    body: Block,
}

impl Parse for ReflectFn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mutability = if input.peek(Token![mut]) {
            Some(input.parse()?)
        } else {
            None
        };
        let movability = if input.peek(Token![move]) {
            Some(input.parse()?)
        } else {
            None
        };

        let content;
        Ok(Self {
            mutability,
            movability,
            _fn_token: input.parse()?,
            name: input.parse()?,
            _parens: parenthesized!(content in input),
            args: content.parse_terminated(Arg::parse, Token![,])?,
            return_type: input.parse()?,
            body: input.parse()?,
        })
    }
}

pub(crate) fn reflect_fn(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ReflectFn);

    let bevy_reflect_path = crate::meta::get_bevy_reflect_path();

    let arg_list = format_ident!("args");

    let dynamic_function = if input.mutability.is_some() {
        quote! { #bevy_reflect_path::func::DynamicFunctionMut }
    } else {
        quote! { #bevy_reflect_path::func::DynamicFunction }
    };

    let movability = &input.movability;

    let extract_args = input.args.iter().map(|arg| {
        let mutability = &arg.mutability;
        let name = &arg.name;
        let ty = &arg.ty;

        quote! { let #mutability #name = #arg_list.take::<#ty>()?; }
    });

    let body = &input.body;

    let info = match &input.name {
        FnName::Anon(_) => quote! { #bevy_reflect_path::func::SignatureInfo::anonymous() },
        FnName::Lit(name) => {
            quote! { #bevy_reflect_path::func::SignatureInfo::named(#name) }
        }
        FnName::Name(name) => {
            let name = name.to_string();
            quote! { #bevy_reflect_path::func::SignatureInfo::named(#name) }
        }
        FnName::Expr(expr) => {
            quote! { #bevy_reflect_path::func::SignatureInfo::named(#expr) }
        }
    };

    let arg_info = input.args.iter().enumerate().map(|(index, arg)| {
        let name = &arg.name;
        let ty = &arg.ty;

        quote! {
            #bevy_reflect_path::func::args::ArgInfo::new::<#ty>(#index).with_name(stringify!(#name))
        }
    });

    let return_ty = match input.return_type {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    proc_macro::TokenStream::from(quote! {{
        #dynamic_function::new(
            #[allow(unused_mut)]
            #movability |mut #arg_list| {
                #(#extract_args)*
                #FQResult::Ok(#bevy_reflect_path::func::IntoReturn::into_return(#body))
            },
            #[allow(unused_braces)]
            #bevy_reflect_path::func::FunctionInfo::new(
                #info
                    .with_args(::alloc::vec![#(#arg_info),*])
                    .with_return::<#return_ty>()
            ),
        )
    }})
}
