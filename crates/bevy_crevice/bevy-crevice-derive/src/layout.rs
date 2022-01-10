use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, Data, DeriveInput, Fields, Ident, Path, Type};

pub fn emit(
    input: DeriveInput,
    trait_name: &'static str,
    mod_name: &'static str,
    min_struct_alignment: usize,
) -> TokenStream {
    let bevy_crevice_path = crate::bevy_crevice_path();

    let mod_name = Ident::new(mod_name, Span::call_site());
    let trait_name = Ident::new(trait_name, Span::call_site());

    let mod_path: Path = parse_quote!(#bevy_crevice_path::#mod_name);
    let trait_path: Path = parse_quote!(#mod_path::#trait_name);

    let as_trait_name = format_ident!("As{}", trait_name);
    let as_trait_path: Path = parse_quote!(#mod_path::#as_trait_name);
    let as_trait_method = format_ident!("as_{}", mod_name);
    let from_trait_method = format_ident!("from_{}", mod_name);

    let padded_name = format_ident!("{}Padded", trait_name);
    let padded_path: Path = parse_quote!(#mod_path::#padded_name);

    let visibility = input.vis;
    let input_name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let generated_name = format_ident!("{}{}", trait_name, input_name);

    // Crevice's derive only works on regular structs. We could potentially
    // support transparent tuple structs in the future.
    let fields: Vec<_> = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields.named.iter().collect(),
            Fields::Unnamed(_) => panic!("Tuple structs are not supported"),
            Fields::Unit => panic!("Unit structs are not supported"),
        },
        Data::Enum(_) | Data::Union(_) => panic!("Only structs are supported"),
    };

    // Gives the layout-specific version of the given type.
    let layout_version_of_ty = |ty: &Type| {
        quote! {
            <#ty as #as_trait_path>::Output
        }
    };

    // Gives an expression returning the layout-specific alignment for the type.
    let layout_alignment_of_ty = |ty: &Type| {
        quote! {
            <<#ty as #as_trait_path>::Output as #trait_path>::ALIGNMENT
        }
    };

    // Gives an expression telling whether the type should have trailing padding
    // at least equal to its alignment.
    let layout_pad_at_end_of_ty = |ty: &Type| {
        quote! {
            <<#ty as #as_trait_path>::Output as #trait_path>::PAD_AT_END
        }
    };

    let field_alignments = fields.iter().map(|field| layout_alignment_of_ty(&field.ty));
    let struct_alignment = quote! {
        #bevy_crevice_path::internal::max_arr([
            #min_struct_alignment,
            #(#field_alignments,)*
        ])
    };

    // Generate names for each padding calculation function.
    let pad_fns: Vec<_> = (0..fields.len())
        .map(|index| format_ident!("_{}__{}Pad{}", input_name, trait_name, index))
        .collect();

    // Computes the offset immediately AFTER the field with the given index.
    //
    // This function depends on the generated padding calculation functions to
    // do correct alignment. Be careful not to cause recursion!
    let offset_after_field = |target: usize| {
        let mut output = vec![quote!(0usize)];

        for index in 0..=target {
            let field_ty = &fields[index].ty;
            let layout_ty = layout_version_of_ty(field_ty);

            output.push(quote! {
                + ::core::mem::size_of::<#layout_ty>()
            });

            // For every field except our target field, also add the generated
            // padding. Padding occurs after each field, so it isn't included in
            // this value.
            if index < target {
                let pad_fn = &pad_fns[index];
                output.push(quote! {
                    + #pad_fn()
                });
            }
        }

        output.into_iter().collect::<TokenStream>()
    };

    let pad_fn_impls: TokenStream = fields
        .iter()
        .enumerate()
        .map(|(index, prev_field)| {
            let pad_fn = &pad_fns[index];

            let starting_offset = offset_after_field(index);
            let prev_field_has_end_padding = layout_pad_at_end_of_ty(&prev_field.ty);
            let prev_field_alignment = layout_alignment_of_ty(&prev_field.ty);

            let next_field_or_self_alignment = fields
                .get(index + 1)
                .map(|next_field| layout_alignment_of_ty(&next_field.ty))
                .unwrap_or(quote!(#struct_alignment));

            quote! {
                /// Tells how many bytes of padding have to be inserted after
                /// the field with index #index.
                #[allow(non_snake_case)]
                const fn #pad_fn() -> usize {
                    // First up, calculate our offset into the struct so far.
                    // We'll use this value to figure out how far out of
                    // alignment we are.
                    let starting_offset = #starting_offset;

                    // If the previous field is a struct or array, we must align
                    // the next field to at least THAT field's alignment.
                    let min_alignment = if #prev_field_has_end_padding {
                        #prev_field_alignment
                    } else {
                        0
                    };

                    // We set our target alignment to the larger of the
                    // alignment due to the previous field and the alignment
                    // requirement of the next field.
                    let alignment = #bevy_crevice_path::internal::max(
                        #next_field_or_self_alignment,
                        min_alignment,
                    );

                    // Using everything we've got, compute our padding amount.
                    #bevy_crevice_path::internal::align_offset(starting_offset, alignment)
                }
            }
        })
        .collect();

    let generated_struct_fields: TokenStream = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let field_name = field.ident.as_ref().unwrap();
            let field_ty = layout_version_of_ty(&field.ty);
            let pad_field_name = format_ident!("_pad{}", index);
            let pad_fn = &pad_fns[index];

            quote! {
                #field_name: #field_ty,
                #pad_field_name: [u8; #pad_fn()],
            }
        })
        .collect();

    let generated_struct_field_init: TokenStream = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();

            quote! {
                #field_name: self.#field_name.#as_trait_method(),
            }
        })
        .collect();

    let input_struct_field_init: TokenStream = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();

            quote! {
                #field_name: #as_trait_path::#from_trait_method(input.#field_name),
            }
        })
        .collect();

    let struct_definition = quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        #[allow(non_snake_case)]
        #visibility struct #generated_name #ty_generics #where_clause {
            #generated_struct_fields
        }
    };

    let debug_methods = if cfg!(feature = "debug-methods") {
        let debug_fields: TokenStream = fields
            .iter()
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap();
                let field_ty = &field.ty;

                quote! {
                    fields.push(Field {
                        name: stringify!(#field_name),
                        size: ::core::mem::size_of::<#field_ty>(),
                        offset: (&zeroed.#field_name as *const _ as usize)
                            - (&zeroed as *const _ as usize),
                    });
                }
            })
            .collect();

        quote! {
            impl #impl_generics #generated_name #ty_generics #where_clause {
                fn debug_metrics() -> String {
                    let size = ::core::mem::size_of::<Self>();
                    let align = <Self as #trait_path>::ALIGNMENT;

                    let zeroed: Self = #bevy_crevice_path::internal::bytemuck::Zeroable::zeroed();

                    #[derive(Debug)]
                    struct Field {
                        name: &'static str,
                        offset: usize,
                        size: usize,
                    }
                    let mut fields = Vec::new();

                    #debug_fields

                    format!("Size {}, Align {}, fields: {:#?}", size, align, fields)
                }

                fn debug_definitions() -> &'static str {
                    stringify!(
                        #struct_definition
                        #pad_fn_impls
                    )
                }
            }
        }
    } else {
        quote!()
    };

    quote! {
        #pad_fn_impls
        #struct_definition

        unsafe impl #impl_generics #bevy_crevice_path::internal::bytemuck::Zeroable for #generated_name #ty_generics #where_clause {}
        unsafe impl #impl_generics #bevy_crevice_path::internal::bytemuck::Pod for #generated_name #ty_generics #where_clause {}

        unsafe impl #impl_generics #mod_path::#trait_name for #generated_name #ty_generics #where_clause {
            const ALIGNMENT: usize = #struct_alignment;
            const PAD_AT_END: bool = true;
            type Padded = #padded_path<Self, {#bevy_crevice_path::internal::align_offset(
                    ::core::mem::size_of::<#generated_name>(),
                    #struct_alignment
                )}>;
        }

        impl #impl_generics #as_trait_path for #input_name #ty_generics #where_clause {
            type Output = #generated_name;

            fn #as_trait_method(&self) -> Self::Output {
                Self::Output {
                    #generated_struct_field_init

                    ..#bevy_crevice_path::internal::bytemuck::Zeroable::zeroed()
                }
            }

            fn #from_trait_method(input: Self::Output) -> Self {
                Self {
                    #input_struct_field_init
                }
            }
        }

        #debug_methods
    }
}
